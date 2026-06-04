#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "otel")]
#[path = "otel.rs"]
mod otel_impl;
#[path = "render.rs"]
mod render_impl;
#[path = "report.rs"]
mod report_impl;
#[cfg(feature = "trace")]
#[path = "trace.rs"]
mod trace_impl;
mod utils;

pub use diagweave_macros::{Error, set, union};
pub use ref_str::{RefStr, StaticRefStr};
pub use report::{DiagnosticError, DiagnosticResult, Report};

#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
mod readme_doctests {}

#[cfg(doctest)]
#[doc = include_str!("../../README_CN.md")]
mod readme_cn_doctests {}

#[cfg(doctest)]
#[doc = include_str!("../../docs/ai/en/error_definition_and_conversion.md")]
mod ai_en_error_def_doctests {}

#[cfg(doctest)]
#[doc = include_str!("../../docs/ai/en/diagnostic_report_container.md")]
mod ai_en_report_container_doctests {}

#[cfg(doctest)]
#[doc = include_str!("../../docs/ai/cn/error_definition_and_conversion.md")]
mod ai_cn_error_def_doctests {}

#[cfg(doctest)]
#[doc = include_str!("../../docs/ai/cn/diagnostic_report_container.md")]
mod ai_cn_report_container_doctests {}

#[cfg(any(feature = "otel", feature = "opentelemetry"))]
pub mod otel {
    #[cfg(feature = "opentelemetry")]
    pub use crate::otel_impl::opentelemetry;
    pub use crate::otel_impl::*;
}

pub mod render {
    pub use crate::render_impl::{
        Compact, CompactProfile, DiagnosticIr, DiagnosticIrError, DiagnosticIrMessage,
        DiagnosticIrMetadata, Pretty, PrettyIndent, RenderedReport, ReportRenderOptions,
        ReportRenderer, StackTraceFilter,
    };
    #[cfg(feature = "json")]
    pub use crate::render_impl::{
        Json, REPORT_JSON_SCHEMA_DRAFT, REPORT_JSON_SCHEMA_VERSION, report_json_schema,
    };
}

pub mod report {
    pub use crate::report_impl::{
        Attachment, AttachmentValue, AttachmentVisit, CauseCollectOptions, CauseKind,
        CauseTraversalState, ContextMap, ContextValue, DiagnosticError, DiagnosticResult,
        DisplayCauseChain, ErrorCode, ErrorCodeIntError, GlobalContext, GlobalErrorMeta,
        HasSeverity, IntoResult, MissingSeverity, Report, ReportMetadata, ReportOptions,
        ReportSourceErrorIter, ResultReportExt, Severity, SeverityParseError, SeverityState,
        SourceErrorChain, SourceErrorEntry, SourceErrorItem, StackFrame, StackTrace,
        StackTraceFormat,
    };
    #[cfg(feature = "std")]
    pub use crate::report_impl::{
        GlobalConfig, RegisterGlobalContextError, SetGlobalConfigError, register_global_injector,
        set_global_config,
    };
    #[cfg(feature = "json")]
    pub use crate::report_impl::{JsonContext, JsonContextEntry};
    #[cfg(feature = "trace")]
    pub use crate::report_impl::{
        ReportTrace, TraceContext, TraceEvent, TraceEventAttribute, TraceEventLevel,
    };
    pub use crate::utils::{HexId, ParentSpanId, SpanId, TraceId, TraceState};
}

#[cfg(feature = "trace")]
pub mod trace {
    #[cfg(feature = "tracing")]
    pub use crate::trace_impl::TracingExporter;
    pub use crate::trace_impl::TracingExporterTrait;
    pub use crate::trace_impl::{
        EmitStats, PreparedTraceEvent, PreparedTracingEmission, PreparedTracingLevel, TracingField,
    };
}

pub mod prelude {
    pub use crate::render::{
        Compact, CompactProfile, Pretty, ReportRenderOptions, ReportRenderer, StackTraceFilter,
    };
    pub use crate::report::{
        AttachmentValue, ContextMap, ContextValue, DiagnosticError, DiagnosticResult, HasSeverity,
        MissingSeverity, Report, ResultReportExt, Severity, SeverityState, SourceErrorItem,
    };
    #[cfg(feature = "std")]
    pub use crate::report::{GlobalContext, register_global_injector};
    #[cfg(any(feature = "trace", feature = "otel"))]
    pub use crate::report::{ParentSpanId, SpanId, TraceId, TraceState};
    #[cfg(feature = "trace")]
    pub use crate::report::{TraceEvent, TraceEventAttribute, TraceEventLevel};
    pub use crate::{Error, RefStr, StaticRefStr, set, union};
}
