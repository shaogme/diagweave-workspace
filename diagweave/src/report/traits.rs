use core::error::Error;
use core::fmt::Display;

use alloc::sync::Arc;

use super::{
    Attachment, ContextMap, ErrorCode, MissingSeverity, Report, ReportMetadata, ReportOptions,
    ReportSourceErrorIter, Severity, SeverityState, StackTrace,
};

pub trait DiagnosticError: Error + Send + Sync + 'static {
    /// Converts the error into a `Report`.
    fn to_report(self) -> Report<Self>
    where
        Self: Sized,
    {
        Report::new(self)
    }

    /// Converts the error into a `Report`, but with a different error type.
    fn to_report_trans<NewE>(self) -> Report<NewE>
    where
        Self: Sized + Into<NewE>,
        NewE: Error + Send + Sync + 'static,
    {
        Report::new(self.into())
    }
}

impl DiagnosticError for core::fmt::Error {}

#[cfg(feature = "std")]
impl DiagnosticError for std::io::Error {}

/// Helper trait to convert a type into a `Result`.
pub trait IntoResult<T, E> {
    fn into_result(self) -> Result<T, E>;
}

impl<T, E> IntoResult<T, E> for Result<T, E> {
    fn into_result(self) -> Self {
        self
    }
}

impl<T, E: Error> IntoResult<T, E> for E {
    fn into_result(self) -> Result<T, E> {
        Err(self)
    }
}

macro_rules! define_diag_res_method {
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty),*) -> Self) => {
        $(#[$attr])*
        fn $name<T>(self, $($arg: $ty),*) -> Result<T, Report<Self::Error, MissingSeverity>>
        where
            Self: Sized + IntoResult<T, Self::Error>,
            Self::Error: Error + Send + Sync + 'static
        {
            self.to_report_res().map_err(|r| r.$name($($arg),*))
        }
    };
    ($(#[$attr:meta])* fn $name:ident <$($gen:ident),*> ($($arg:ident : $ty:ty $(,)? )* ) -> Self where $($bound:tt)*) => {
        $(#[$attr])*
        fn $name <$($gen),* , T> (self, $($arg: $ty),*) -> Result<T, Report<Self::Error, MissingSeverity>>
        where
            Self: Sized + IntoResult<T, Self::Error>,
            Self::Error: Error + Send + Sync + 'static,
            $($bound)*
        {
            self.to_report_res().map_err(|r| r.$name($($arg),*))
        }
    };
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty $(,)? )* ) -> $state:ty [STATE_CHANGE]) => {
        $(#[$attr])*
        fn $name<T>(self, $($arg: $ty),*) -> Result<T, crate::report::Report<Self::Error, $state>>
        where
            Self: Sized + IntoResult<T, Self::Error>,
            Self::Error: Error + Send + Sync + 'static
        {
            self.to_report_res().map_err(|r| r.$name($($arg),*))
        }
    };
}

type State = MissingSeverity;

/// A trait for types that can be converted into a diagnostic result.
pub trait DiagnosticResult {
    /// The error type.
    type Error;

    for_each_report_builder_method!(define_diag_res_method);

    /// Converts the type into a diagnostic result.
    fn to_report_res<T>(self) -> Result<T, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T, Self::Error>,
        Self::Error: Error + Send + Sync + 'static;

    /// Converts the type into a diagnostic result, automatically converting the inner error
    /// type to `TargetE` via `Into`.
    fn to_report_res_trans<T, TargetE>(self) -> Result<T, Report<TargetE>>
    where
        Self: Sized + IntoResult<T, Self::Error>,
        Self::Error: Into<TargetE>,
        Self::Error: Error + Send + Sync + 'static,
        TargetE: Error + Send + Sync + 'static,
    {
        self.to_report_res::<T>().map_err(|e| e.map_err(Into::into))
    }

    fn to_report_note<T>(
        self,
        message: impl Display + Send + Sync + 'static,
    ) -> Result<T, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T, Self::Error>,
        Self::Error: Error + Send + Sync + 'static,
    {
        self.to_report_res::<T>().map_report(
            |report: Report<Self::Error, MissingSeverity>| -> Report<Self::Error, MissingSeverity> {
                Report::<Self::Error, MissingSeverity>::attach_note(report, message)
            },
        )
    }
}

impl<T, E> DiagnosticResult for Result<T, E>
where
    E: DiagnosticError,
{
    type Error = E;

    fn to_report_res<T2>(self) -> Result<T2, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T2, Self::Error>,
        Self::Error: Error + Send + Sync + 'static,
    {
        self.into_result().map_err(Report::new)
    }
}

impl<E> DiagnosticResult for E
where
    E: DiagnosticError,
{
    type Error = E;

    fn to_report_res<T2>(self) -> Result<T2, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T2, Self::Error>,
        Self::Error: Error + Send + Sync + 'static,
    {
        self.into_result().map_err(|e| Report::new(e))
    }
}

macro_rules! define_ext_method {
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty),*) -> Self) => {
        $(#[$attr])*
        fn $name<T>(self, $($arg: $ty),*) -> Result<T, Report<E, State>>
        where
            Self: Sized + IntoResult<T, Report<E, State>>
        {
            self.into_result().map_err(|r| r.$name($($arg),*))
        }
    };
    ($(#[$attr:meta])* fn $name:ident <$($gen:ident),*> ($($arg:ident : $ty:ty $(,)? )* ) -> Self where $($bound:tt)*) => {
        $(#[$attr])*
        fn $name <$($gen),* , T> (self, $($arg: $ty),*) -> Result<T, Report<E, State>>
        where
            Self: Sized + IntoResult<T, Report<E, State>>,
            $($bound)*
        {
            self.into_result().map_err(|r| r.$name($($arg),*))
        }
    };
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty $(,)? )* ) -> $state:ty [STATE_CHANGE]) => {
        $(#[$attr])*
        fn $name<T>(self, $($arg: $ty),*) -> Result<T, crate::report::Report<E, $state>>
        where
            Self: Sized + IntoResult<T, Report<E, State>>
        {
            self.into_result().map_err(|r| r.$name($($arg),*))
        }
    };
}

pub trait ResultReportExt<E, State = MissingSeverity>
where
    State: SeverityState,
{
    /// Applies a transformation to the inner `Report` only on the error path.
    ///
    /// The closure receives an owned `Report` and must return an owned `Report`
    /// of any error and state type. If the result is `Ok`, the
    /// closure is never invoked.
    fn map_report<T, NewE, NewState>(
        self,
        f: impl FnOnce(Report<E, State>) -> Report<NewE, NewState>,
    ) -> Result<T, Report<NewE, NewState>>
    where
        Self: Sized + IntoResult<T, Report<E, State>>,
        NewState: SeverityState,
    {
        self.into_result().map_err(f)
    }

    /// Maps the inner error type of the report while preserving all diagnostic data.
    ///
    /// This is a convenience wrapper around [`Report::map_err`] that operates
    /// on the error path of a `Result`.
    fn map_inner_err<T, NewE>(self, f: impl FnOnce(E) -> NewE) -> Result<T, Report<NewE, State>>
    where
        Self: Sized + IntoResult<T, Report<E, State>>,
        E: Error + Send + Sync + 'static,
        NewE: Error + Send + Sync + 'static,
    {
        self.into_result().map_err(|r| r.map_err(f))
    }

    /// Consumes the result and returns the inner error if it's an error,
    /// discarding all diagnostic information.
    fn into_inner_err<T>(self) -> Result<T, E>
    where
        Self: Sized + IntoResult<T, Report<E, State>>,
    {
        self.into_result().map_err(|r| r.into_inner())
    }

    for_each_report_builder_method!(define_ext_method);

    /// Returns a reference to the inner `Report` on the error path, or `None`.
    fn report_ref<'a>(&'a self) -> Option<&'a Report<E, State>>
    where
        State: 'a,
        E: 'a;

    /// Returns a reference to the inner error on the error path, or `None`.
    fn report_inner<'a>(&'a self) -> Option<&'a E>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::inner)
    }

    /// Returns the report's attachments on the error path, or `None`.
    fn report_attachments<'a>(&'a self) -> Option<&'a [Attachment]>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::attachments)
    }

    /// Returns the report's context on the error path, or `None`.
    fn report_context<'a>(&'a self) -> Option<&'a ContextMap>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::context)
    }

    /// Returns the report's system context on the error path, or `None`.
    fn report_system<'a>(&'a self) -> Option<&'a ContextMap>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::system)
    }

    /// Returns the report's metadata on the error path, or `None`.
    fn report_metadata<'a>(&'a self) -> Option<&'a ReportMetadata<State>>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::metadata)
    }

    /// Returns the report's error code on the error path, or `None`.
    fn report_error_code<'a>(&'a self) -> Option<&'a ErrorCode>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().and_then(Report::<E, State>::error_code)
    }

    /// Returns the report's severity on the error path, or `None`.
    fn report_severity<'a>(&'a self) -> Option<Severity>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().and_then(Report::<E, State>::severity)
    }

    /// Returns the report's category on the error path, or `None`.
    fn report_category<'a>(&'a self) -> Option<&'a str>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().and_then(Report::<E, State>::category)
    }

    /// Returns whether the report is retryable on the error path, or `None`.
    fn report_retryable<'a>(&'a self) -> Option<bool>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().and_then(Report::<E, State>::retryable)
    }

    /// Returns the report's stack trace on the error path, or `None`.
    fn report_stack_trace<'a>(&'a self) -> Option<&'a StackTrace>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().and_then(Report::<E, State>::stack_trace)
    }

    /// Returns the report's options on the error path, or `None`.
    fn report_options<'a>(&'a self) -> Option<&'a ReportOptions>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::options)
    }

    /// Returns the report's display causes on the error path, or `None`.
    fn report_display_causes<'a>(
        &'a self,
    ) -> Option<&'a [Arc<dyn core::fmt::Display + Send + Sync>]>
    where
        State: 'a,
        E: 'a,
    {
        self.report_ref().map(Report::<E, State>::display_causes)
    }

    /// Returns an iterator over the report's origin source errors on the error path, or `None`.
    fn report_iter_origin_sources<'a>(&'a self) -> Option<ReportSourceErrorIter<'a>>
    where
        E: Error + 'a,
        State: 'a,
    {
        self.report_ref()
            .map(Report::<E, State>::iter_origin_sources)
    }

    /// Returns an iterator over the report's diagnostic source errors on the error path, or `None`.
    fn report_iter_diag_sources<'a>(&'a self) -> Option<ReportSourceErrorIter<'a>>
    where
        E: Error + 'a,
        State: 'a,
    {
        self.report_ref().map(Report::<E, State>::iter_diag_sources)
    }
}

impl<T, E, State> ResultReportExt<E, State> for Result<T, Report<E, State>>
where
    State: SeverityState,
{
    fn report_ref<'a>(&'a self) -> Option<&'a Report<E, State>>
    where
        State: 'a,
        E: 'a,
    {
        self.as_ref().err()
    }
}

/// A trait for performing conversions to diagnostic reports or results.
pub trait Transform<Target> {
    /// Perform the conversion.
    fn trans(self) -> Target;
}

impl<E1, E2> Transform<Report<E2>> for E1
where
    E1: DiagnosticError + Into<E2>,
    E2: Error + Send + Sync + 'static,
{
    #[inline]
    fn trans(self) -> Report<E2> {
        self.to_report_trans()
    }
}

impl<T, E1, E2> Transform<Result<T, Report<E2>>> for E1
where
    E1: DiagnosticError + Into<E2>,
    E2: Error + Send + Sync + 'static,
{
    #[inline]
    fn trans(self) -> Result<T, Report<E2>> {
        Err(self.to_report_trans())
    }
}

impl<T, E1, E2, State> Transform<Result<T, Report<E2, State>>> for Report<E1, State>
where
    E1: DiagnosticError + Into<E2>,
    E2: Error + Send + Sync + 'static,
    State: SeverityState,
{
    #[inline]
    fn trans(self) -> Result<T, Report<E2, State>> {
        Err(self.map_err(|e| e.into()))
    }
}

impl<E1, E2, State> Transform<Report<E2, State>> for Report<E1, State>
where
    E1: DiagnosticError + Into<E2>,
    E2: Error + Send + Sync + 'static,
    State: SeverityState,
{
    #[inline]
    fn trans(self) -> Report<E2, State> {
        self.map_err(|e| e.into())
    }
}

impl<T, E1, E2, State> Transform<Result<T, Report<E2, State>>> for Result<T, Report<E1, State>>
where
    E1: DiagnosticError + Into<E2>,
    E2: Error + Send + Sync + 'static,
    State: SeverityState,
{
    #[inline]
    fn trans(self) -> Result<T, Report<E2, State>> {
        self.map_err(|r| r.map_err(|e| e.into()))
    }
}

impl<T, E1, E2> Transform<Result<T, Report<E2>>> for Result<T, E1>
where
    E1: DiagnosticError + Into<E2>,
    E2: Error + Send + Sync + 'static,
{
    #[inline]
    fn trans(self) -> Result<T, Report<E2>> {
        self.map_err(|e| e.to_report_trans())
    }
}
