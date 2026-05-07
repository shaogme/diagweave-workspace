use crate::utils::{ParentSpanId, SpanId, TraceId, TraceState};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter};
use ref_str::StaticRefStr;

use super::{Report, SeverityState, types::AttachmentValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Severity level for a trace event.
pub enum TraceEventLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl TraceEventLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl Display for TraceEventLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A key-value attribute attached to a trace event.
pub struct TraceEventAttribute {
    pub key: StaticRefStr,
    pub value: AttachmentValue,
}

#[derive(Debug, Clone, PartialEq)]
/// A single event emitted within a trace.
pub struct TraceEvent {
    pub name: StaticRefStr,
    pub level: Option<TraceEventLevel>,
    pub timestamp_unix_nano: Option<u64>,
    pub attributes: Vec<TraceEventAttribute>,
}

impl Default for TraceEvent {
    fn default() -> Self {
        Self {
            name: "".into(),
            level: None,
            timestamp_unix_nano: None,
            attributes: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
/// Trace context values associated with a report.
pub struct TraceContext {
    pub trace_id: Option<TraceId>,
    pub span_id: Option<SpanId>,
    pub parent_span_id: Option<ParentSpanId>,
    pub sampled: Option<bool>,
    pub trace_state: Option<TraceState>,
}

impl TraceContext {
    /// Returns true if the trace context is empty (no IDs or flags).
    pub fn is_empty(&self) -> bool {
        self.trace_id.is_none()
            && self.span_id.is_none()
            && self.parent_span_id.is_none()
            && self.sampled.is_none()
            && self.trace_state.is_none()
    }
}

/// Inner trace payload attached to a report.
/// This struct contains the actual trace data and is boxed inside ReportMetadata.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct ReportTraceInner {
    context: TraceContext,
    events: Vec<TraceEvent>,
}

impl ReportTraceInner {
    /// Returns true if the report trace is empty (no context and no events).
    fn is_empty(&self) -> bool {
        self.context.is_empty() && self.events.is_empty()
    }
}

/// Trace payload attached to a report.
///
/// Contains trace context (trace ID, span ID, etc.) and trace events.
/// Uses lazy allocation via `Option<Box<ReportTraceInner>>` to minimize
/// overhead when no trace information is present.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReportTrace {
    inner: Option<Box<ReportTraceInner>>,
}

impl ReportTrace {
    /// Creates a new empty ReportTrace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the report trace is empty (no context and no events).
    pub fn is_empty(&self) -> bool {
        self.inner.as_ref().is_none_or(|inner| inner.is_empty())
    }

    /// Returns the trace context, if any.
    pub fn context(&self) -> Option<&TraceContext> {
        self.inner.as_ref().map(|inner| &inner.context)
    }

    /// Returns the trace events, if any.
    pub fn events(&self) -> Option<&[TraceEvent]> {
        self.inner.as_ref().map(|inner| inner.events.as_slice())
    }

    /// Ensures the inner trace data is allocated, creating it if necessary.
    fn ensure_inner(&mut self) -> &mut ReportTraceInner {
        self.inner
            .get_or_insert_with(|| Box::new(ReportTraceInner::default()))
    }

    /// Returns a mutable reference to the trace context, if any.
    pub fn context_mut(&mut self) -> Option<&mut TraceContext> {
        self.inner.as_mut().map(|inner| &mut inner.context)
    }

    /// Sets the trace context, replacing any existing value.
    pub fn set_context(mut self, context: TraceContext) -> Self {
        self.ensure_inner().context = context;
        self
    }

    /// Sets the trace context only if not already set.
    pub fn with_context(mut self, context: TraceContext) -> Self {
        if self.context().is_none() || self.context().is_none_or(|c| c.is_empty()) {
            self.ensure_inner().context = context;
        }
        self
    }

    /// Adds a trace event.
    pub fn with_event(mut self, event: TraceEvent) -> Self {
        self.ensure_inner().events.push(event);
        self
    }

    /// Sets the trace ID, replacing any existing value.
    pub fn set_trace_id(mut self, trace_id: TraceId) -> Self {
        self.ensure_inner().context.trace_id = Some(trace_id);
        self
    }

    /// Sets the trace ID only if not already set.
    pub fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        if self.context().and_then(|c| c.trace_id.as_ref()).is_none() {
            self.ensure_inner().context.trace_id = Some(trace_id);
        }
        self
    }

    /// Sets the span ID, replacing any existing value.
    pub fn set_span_id(mut self, span_id: SpanId) -> Self {
        self.ensure_inner().context.span_id = Some(span_id);
        self
    }

    /// Sets the span ID only if not already set.
    pub fn with_span_id(mut self, span_id: SpanId) -> Self {
        if self.context().and_then(|c| c.span_id.as_ref()).is_none() {
            self.ensure_inner().context.span_id = Some(span_id);
        }
        self
    }

    /// Sets the parent span ID, replacing any existing value.
    pub fn set_parent_span_id(mut self, parent_span_id: ParentSpanId) -> Self {
        self.ensure_inner().context.parent_span_id = Some(parent_span_id);
        self
    }

    /// Sets the parent span ID only if not already set.
    pub fn with_parent_span_id(mut self, parent_span_id: ParentSpanId) -> Self {
        if self
            .context()
            .and_then(|c| c.parent_span_id.as_ref())
            .is_none()
        {
            self.ensure_inner().context.parent_span_id = Some(parent_span_id);
        }
        self
    }

    /// Sets whether the trace is sampled, replacing any existing value.
    pub fn set_sampled(mut self, sampled: bool) -> Self {
        let inner = self.ensure_inner();
        inner.context.sampled = Some(sampled);
        self
    }

    /// Sets whether the trace is sampled only if not already set.
    pub fn with_sampled(mut self, sampled: bool) -> Self {
        if self.context().and_then(|c| c.sampled).is_none() {
            let inner = self.ensure_inner();
            inner.context.sampled = Some(sampled);
        }
        self
    }

    /// Sets the trace state, replacing any existing value.
    pub fn set_trace_state(mut self, trace_state: impl Into<StaticRefStr>) -> Self {
        self.ensure_inner().context.trace_state = Some(TraceState::from(trace_state.into()));
        self
    }

    /// Sets the trace state only if not already set.
    pub fn with_trace_state(mut self, trace_state: impl Into<StaticRefStr>) -> Self {
        if self
            .context()
            .and_then(|c| c.trace_state.as_ref())
            .is_none()
        {
            self.ensure_inner().context.trace_state = Some(TraceState::from(trace_state.into()));
        }
        self
    }

    /// Sets the trace ID from an Option, only if not already set.
    pub fn set_trace_id_opt(mut self, trace_id: Option<TraceId>) -> Self {
        if let Some(tid) = trace_id
            && self.context().and_then(|c| c.trace_id.as_ref()).is_none()
        {
            self.ensure_inner().context.trace_id = Some(tid);
        }
        self
    }

    /// Sets the span ID from an Option, only if not already set.
    pub fn set_span_id_opt(mut self, span_id: Option<SpanId>) -> Self {
        if let Some(sid) = span_id
            && self.context().and_then(|c| c.span_id.as_ref()).is_none()
        {
            self.ensure_inner().context.span_id = Some(sid);
        }
        self
    }

    /// Sets the parent span ID from an Option, only if not already set.
    pub fn set_parent_span_id_opt(mut self, parent_span_id: Option<ParentSpanId>) -> Self {
        if let Some(psid) = parent_span_id
            && self
                .context()
                .and_then(|c| c.parent_span_id.as_ref())
                .is_none()
        {
            self.ensure_inner().context.parent_span_id = Some(psid);
        }
        self
    }

    /// Sets the sampled flag from an Option, only if not already set.
    pub fn set_sampled_opt(mut self, sampled: Option<bool>) -> Self {
        if let Some(s) = sampled
            && self.context().and_then(|c| c.sampled).is_none()
        {
            let inner = self.ensure_inner();
            inner.context.sampled = Some(s);
        }
        self
    }

    /// Sets the trace state from an Option, only if not already set.
    pub fn set_trace_state_opt(mut self, trace_state: Option<TraceState>) -> Self {
        if let Some(ts) = trace_state
            && self
                .context()
                .and_then(|c| c.trace_state.as_ref())
                .is_none()
        {
            self.ensure_inner().context.trace_state = Some(ts);
        }
        self
    }

    /// Returns a mutable reference to the inner events, allocating if necessary.
    fn events_mut(&mut self) -> &mut Vec<TraceEvent> {
        &mut self.ensure_inner().events
    }
}

impl<E, State> Report<E, State>
where
    State: SeverityState,
{
    /// Returns the trace information associated with the report, if any.
    pub fn trace(&self) -> &ReportTrace {
        &self.data.trace
    }

    /// Sets the trace information for the report, replacing any existing value.
    pub fn set_trace(mut self, trace: ReportTrace) -> Self {
        self.data.trace = trace;
        self
    }

    /// Sets the trace information only if not already present.
    pub fn with_trace(mut self, trace: ReportTrace) -> Self {
        if self.trace().is_empty() {
            self.data.trace = trace;
        }
        self
    }

    /// Sets the trace and span IDs for the report, replacing any existing values.
    pub fn set_trace_ids(mut self, trace_id: TraceId, span_id: SpanId) -> Self {
        let inner = self.trace_mut().ensure_inner();
        inner.context.trace_id = Some(trace_id);
        inner.context.span_id = Some(span_id);
        self
    }

    /// Sets the trace and span IDs only if not already set.
    pub fn with_trace_ids(mut self, trace_id: TraceId, span_id: SpanId) -> Self {
        let trace_ref = self.trace();
        let needs_trace_id = trace_ref.is_empty()
            || trace_ref
                .context()
                .and_then(|c| c.trace_id.as_ref())
                .is_none();
        let needs_span_id = trace_ref.is_empty()
            || trace_ref
                .context()
                .and_then(|c| c.span_id.as_ref())
                .is_none();
        if needs_trace_id {
            self.trace_mut().ensure_inner().context.trace_id = Some(trace_id);
        }
        if needs_span_id {
            self.trace_mut().ensure_inner().context.span_id = Some(span_id);
        }
        self
    }

    /// Sets the parent span ID for the report, replacing any existing value.
    pub fn set_parent_span_id(mut self, parent_span_id: ParentSpanId) -> Self {
        self.trace_mut().ensure_inner().context.parent_span_id = Some(parent_span_id);
        self
    }

    /// Sets the parent span ID only if not already set.
    pub fn with_parent_span_id(mut self, parent_span_id: ParentSpanId) -> Self {
        if self
            .trace()
            .context()
            .and_then(|c| c.parent_span_id.as_ref())
            .is_none()
        {
            self.trace_mut().ensure_inner().context.parent_span_id = Some(parent_span_id);
        }
        self
    }

    /// Sets whether the trace is sampled, replacing any existing value.
    pub fn set_trace_sampled(mut self, sampled: bool) -> Self {
        let inner = self.trace_mut().ensure_inner();
        inner.context.sampled = Some(sampled);
        self
    }

    /// Sets whether the trace is sampled only if not already set.
    pub fn with_trace_sampled(mut self, sampled: bool) -> Self {
        if self.trace().context().and_then(|c| c.sampled).is_none() {
            let inner = self.trace_mut().ensure_inner();
            inner.context.sampled = Some(sampled);
        }
        self
    }

    /// Sets the trace state, replacing any existing value.
    pub fn set_trace_state(mut self, trace_state: impl Into<StaticRefStr>) -> Self {
        self.trace_mut().ensure_inner().context.trace_state =
            Some(TraceState::from(trace_state.into()));
        self
    }

    /// Sets the trace state only if not already set.
    pub fn with_trace_state(mut self, trace_state: impl Into<StaticRefStr>) -> Self {
        if self
            .trace()
            .context()
            .and_then(|c| c.trace_state.as_ref())
            .is_none()
        {
            self.trace_mut().ensure_inner().context.trace_state =
                Some(TraceState::from(trace_state.into()));
        }
        self
    }

    /// Adds a trace event to the report.
    pub fn with_trace_event(mut self, event: TraceEvent) -> Self {
        self.trace_mut().events_mut().push(event);
        self
    }

    /// Pushes a trace event with the specified name.
    pub fn push_trace_event(mut self, name: impl Into<StaticRefStr>) -> Self {
        self.trace_mut().events_mut().push(TraceEvent {
            name: name.into(),
            ..TraceEvent::default()
        });
        self
    }

    /// Pushes a trace event with detailed information.
    pub fn push_trace_event_with(
        mut self,
        name: impl Into<StaticRefStr>,
        level: Option<TraceEventLevel>,
        timestamp_unix_nano: Option<u64>,
        attributes: impl IntoIterator<Item = TraceEventAttribute>,
    ) -> Self {
        self.trace_mut().events_mut().push(TraceEvent {
            name: name.into(),
            level,
            timestamp_unix_nano,
            attributes: attributes.into_iter().collect::<Vec<_>>(),
        });
        self
    }

    fn trace_mut(&mut self) -> &mut ReportTrace {
        &mut self.data.trace
    }
}
