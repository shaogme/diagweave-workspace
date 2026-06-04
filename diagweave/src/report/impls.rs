use super::ReportMetadata;
use core::error::Error;
use core::fmt::{self, Debug, Display, Formatter};

use super::Attachment;
use super::SourceErrorChain;

use super::{Report, SeverityState};

/// Macro to write a field with separator handling.
/// Writes ", " before the field if idx > 0, then increments idx.
macro_rules! write_field {
    ($f:expr, $idx:expr, $name:expr, $val:expr) => {{
        if $idx == 0 {
            write!($f, " [")?;
        } else {
            write!($f, ", ")?;
        }
        write!($f, "{}={}", $name, $val)?;
        $idx += 1;
    }};
}

/// Macro to write raw content with separator handling.
/// Writes ", " before the content if idx > 0, then increments idx.
macro_rules! write_raw {
    ($f:expr, $idx:expr, $($arg:tt)*) => {{
        if $idx == 0 {
            write!($f, " [")?;
        } else {
            write!($f, ", ")?;
        }
        write!($f, $($arg)*)?;
        $idx += 1;
    }};
}

impl<E, State> Debug for Report<E, State>
where
    E: Debug,
    State: SeverityState + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(debug_assertions)]
        {
            writeln!(f, "Report:")?;
            writeln!(f, " - error: {:?}", Report::<E, State>::inner(self))?;
            writeln!(f, " - metadata: {:?}", Report::<E, State>::metadata(self))?;
            let diag: &super::DiagnosticBag = Report::<E, State>::diagnostics(self);
            writeln!(f, " - attachments:")?;
            if diag.attachments().is_empty() {
                writeln!(f, " - (none)")?;
            } else {
                for attachment in diag.attachments() {
                    writeln!(f, " - {:?}", attachment)?;
                }
            }
            #[cfg(feature = "trace")]
            writeln!(f, " - trace: {:?}", Report::<E, State>::trace(self))?;
            let display_causes = diag
                .display_causes()
                .map(|v| v.items.as_slice())
                .unwrap_or(&[]);
            if display_causes.is_empty() {
                writeln!(f, " - display_causes: (none)")?;
            } else {
                writeln!(f, " - display_causes:")?;
                for cause in display_causes {
                    writeln!(f, " - {}", cause)?;
                }
            }
            Ok(())
        }
        #[cfg(not(debug_assertions))]
        {
            f.debug_struct("Report")
                .field("inner", Report::<E, State>::inner(self))
                .field("bag", &self.data.bag)
                .finish()
        }
    }
}

impl<E, State> Display for Report<E, State>
where
    E: Display,
    State: SeverityState,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Report::<E, State>::inner(self))?;
        let metadata = Report::<E, State>::metadata(self);
        let has_metadata = metadata.has_metadata();
        let diag: &super::DiagnosticBag = Report::<E, State>::diagnostics(self);
        let has_diagnostics = diag.has_diagnostics() || {
            #[cfg(feature = "trace")]
            {
                !Report::<E, State>::trace(self).is_empty()
            }
            #[cfg(not(feature = "trace"))]
            {
                false
            }
        };
        if !has_diagnostics && !has_metadata {
            return Ok(());
        }
        write!(f, " [")?;
        let mut idx = 0usize;
        Report::<E, State>::fmt_metadata_fields(self, f, &mut idx)?;
        Report::<E, State>::fmt_diag_fields(self, f, &mut idx)?;
        if idx > 0 {
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl<E, State> Report<E, State>
where
    E: Display,
    State: SeverityState,
{
    fn fmt_metadata_fields(&self, f: &mut Formatter<'_>, idx: &mut usize) -> fmt::Result {
        let metadata: &ReportMetadata<State> = Report::<E, State>::metadata(self);
        if let Some(code) = metadata.error_code() {
            write_field!(f, *idx, "code", code);
        }
        if let Some(sev) = Report::<E, State>::severity(self) {
            write_field!(f, *idx, "severity", &sev);
        }
        if let Some(cat) = metadata.category() {
            write_field!(f, *idx, "category", &cat);
        }
        if let Some(ret) = metadata.retryable() {
            write_field!(f, *idx, "retryable", &ret);
        }
        Ok(())
    }

    fn fmt_diag_fields(&self, f: &mut Formatter<'_>, idx: &mut usize) -> fmt::Result {
        let diag: &super::DiagnosticBag = Report::<E, State>::diagnostics(self);

        #[cfg(feature = "trace")]
        {
            let trace = Report::<E, State>::trace(self);
            if let Some(context) = trace.context() {
                if let Some(tid) = &context.trace_id {
                    write_field!(f, *idx, "trace_id", tid.as_ref());
                }
                if let Some(sid) = &context.span_id {
                    write_field!(f, *idx, "span_id", sid.as_ref());
                }
            }
        }

        if diag.stack_trace().is_some() {
            write_field!(f, *idx, "stack_trace", "present");
        }

        for (key, value) in diag.context().sorted_entries() {
            write_field!(f, *idx, key, value);
        }

        for (key, value) in diag.system().sorted_entries() {
            write_field!(f, *idx, format_args!("system.{}", key), value);
        }

        for attachment in diag.attachments() {
            match attachment {
                Attachment::Note { message } => {
                    write_raw!(f, *idx, "{}", message);
                }
                Attachment::Payload {
                    name,
                    value,
                    media_type,
                } => match media_type {
                    Some(mt) => write_raw!(f, *idx, "{}={} ({})", name, value, mt),
                    None => write_raw!(f, *idx, "{}={}", name, value),
                },
            }
        }

        Ok(())
    }
}

impl<E, State> Error for Report<E, State>
where
    E: Error + 'static,
    State: SeverityState + Debug,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Report::<E, State>::diagnostics(self)
            .origin_src_errors()
            .and_then(SourceErrorChain::first_error)
            .or_else(|| Report::<E, State>::inner(self).source())
    }
}

impl<E, NewE> From<E> for Report<NewE, super::MissingSeverity>
where
    E: super::DiagnosticError,
    NewE: From<E>,
{
    #[inline]
    fn from(err: E) -> Self {
        Report::new(NewE::from(err))
    }
}
