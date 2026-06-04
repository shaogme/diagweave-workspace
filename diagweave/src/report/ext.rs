use core::error::Error;
use core::fmt::Display;

use super::{
    Attachment, ContextMap, ErrorCode, MissingSeverity, Report, ReportMetadata, ReportOptions,
    ReportSourceErrorIter, Severity, SeverityState, StackTrace,
};

/// A marker trait for raw client error types that can be wrapped in a `Report`.
///
/// This trait is automatically implemented for error types using `#[derive(Error)]`,
/// `set!`, or `union!`.
pub trait DiagnosticError {}

/// Helper trait to convert a type into a `Result`.
pub trait IntoResult<T, E> {
    fn into_result(self) -> Result<T, E>;
}

impl<T, E> IntoResult<T, E> for Result<T, E> {
    fn into_result(self) -> Self {
        self
    }
}

impl<T, E> IntoResult<T, E> for E {
    fn into_result(self) -> Result<T, E> {
        Err(self)
    }
}

/// A trait for types that can be converted into a diagnostic result.
pub trait Diagnostic {
    /// The error type.
    type Error;

    fn to_report<T>(self) -> Result<T, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T, Self::Error>;

    /// Converts the inner error to a different type via `Into` while wrapping in a `Report`.
    fn to_report_trans<T, NewE>(self) -> Result<T, Report<NewE>>
    where
        Self: Sized + IntoResult<T, Self::Error>,
        Self::Error: Into<NewE>,
        Self::Error: Error + Send + Sync + 'static,
        NewE: Error + Send + Sync + 'static,
    {
        self.to_report().map_err(|r| r.map_err(|e| e.into()))
    }

    /// Convenience: perform a transformation on the error path in a single step.
    ///
    /// This is a generic variant that allows transforming both the error type
    /// and the state type. When only adding metadata (context, notes, etc.),
    /// no explicit type annotations are needed. When transforming the error
    /// type (e.g., via `map_err`), the return type must be annotated.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // No type annotation needed when only adding metadata
    /// fail_auth().diag(|r| {
    ///     r.with_ctx("request_id", 77u64)
    ///         .with_error_code("AUTH.INVALID_TOKEN")
    /// })
    ///
    /// // Type annotation needed when transforming error type
    /// let err: Result<(), Report<ApiError>> = fail_auth().diag(|r| {
    ///     r.map_err(|_| ApiError::Unauthorized)
    /// });
    /// ```
    fn diag<T, E2, State2>(
        self,
        f: impl FnOnce(Report<Self::Error>) -> Report<E2, State2>,
    ) -> Result<T, Report<E2, State2>>
    where
        Self: Sized + IntoResult<T, Self::Error>,
        State2: SeverityState,
    {
        self.to_report().map_err(f)
    }

    fn to_report_note<T>(
        self,
        message: impl Display + Send + Sync + 'static,
    ) -> Result<T, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T, Self::Error>,
    {
        self.to_report().map_report(
            |report: Report<Self::Error, MissingSeverity>| -> Report<Self::Error, MissingSeverity> {
                Report::<Self::Error, MissingSeverity>::attach_note(report, message)
            },
        )
    }
}

impl<T, E> Diagnostic for Result<T, E> {
    type Error = E;

    fn to_report<T2>(self) -> Result<T2, Report<Self::Error>>
    where
        Self: Sized + IntoResult<T2, Self::Error>,
    {
        self.into_result().map_err(Report::new)
    }
}

/// Extension trait for `Result<T, Report<E, State>>` to apply diagnostic transformations
/// only on the error path, without duplicating every `Report` method.
///
/// # Example
///
/// ```ignore
/// db_operation()
///     .diag(|r| {
///         r.with_ctx("user_id", user_id)
///             .attach_note("failing over")
///             .capture_stack_trace()
///     })
///     .map_err(|db_err| AppError::from(db_err))?;
/// ```
macro_rules! define_ext_method {
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty),*) -> Self) => {
        $(#[$attr])*
        fn $name(self, $($arg: $ty),*) -> Self;
    };
    ($(#[$attr:meta])* fn $name:ident <$($gen:ident),*> ($($arg:ident : $ty:ty $(,)? )* ) -> Self where $($bound:tt)*) => {
        $(#[$attr])*
        fn $name <$($gen),*> (self, $($arg: $ty),*) -> Self where $($bound)*;
    };
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty $(,)? )* ) -> Result<T, $report:ty> [STATE_CHANGE]) => {
        $(#[$attr])*
        fn $name<T>(self, $($arg: $ty),*) -> Result<T, $report>
        where
            Self: Sized + IntoResult<T, Report<E, State>>;
    };
}

macro_rules! impl_ext_method {
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty),*) -> Self) => {
        fn $name(self, $($arg: $ty),*) -> Self {
            self.map_err(|r| r.$name($($arg),*))
        }
    };
    ($(#[$attr:meta])* fn $name:ident <$($gen:ident),*> ($($arg:ident : $ty:ty $(,)? )* ) -> Self where $($bound:tt)*) => {
        fn $name <$($gen),*> (self, $($arg: $ty),*) -> Self where $($bound)* {
            self.map_err(|r| r.$name($($arg),*))
        }
    };
    ($(#[$attr:meta])* fn $name:ident($($arg:ident : $ty:ty $(,)? )* ) -> Result<T, $report:ty> [STATE_CHANGE]) => {
        fn $name<T2>(self, $($arg: $ty),*) -> Result<T2, $report>
        where
            Self: Sized + IntoResult<T2, Report<E, State>>
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
        NewState: SeverityState;

    /// Maps the inner error type of the report while preserving all diagnostic data.
    ///
    /// This is a convenience wrapper around [`Report::map_err`] that operates
    /// on the error path of a `Result`.
    fn map_inner_err<T, NewE>(self, f: impl FnOnce(E) -> NewE) -> Result<T, Report<NewE, State>>
    where
        Self: Sized + IntoResult<T, Report<E, State>>,
        E: Error + Send + Sync + 'static,
        NewE: Error + Send + Sync + 'static;

    /// A convenient shortcut to convert the inner error to a different type via `Into`.
    fn trans_inner_err<T, NewE>(self) -> Result<T, Report<NewE, State>>
    where
        Self: Sized + IntoResult<T, Report<E, State>>,
        E: Error + Send + Sync + 'static,
        E: Into<NewE>,
        NewE: Error + Send + Sync + 'static;

    /// Consumes the result and returns the inner error if it's an error,
    /// discarding all diagnostic information.
    fn into_inner_err<T>(self) -> Result<T, E>
    where
        Self: Sized + IntoResult<T, Report<E, State>>;

    for_each_report_builder_method!(define_ext_method);
}

impl<T, E, State> ResultReportExt<E, State> for Result<T, Report<E, State>>
where
    State: SeverityState,
{
    fn map_report<T2, NewE, NewState>(
        self,
        f: impl FnOnce(Report<E, State>) -> Report<NewE, NewState>,
    ) -> Result<T2, Report<NewE, NewState>>
    where
        Self: Sized + IntoResult<T2, Report<E, State>>,
        NewState: SeverityState,
    {
        self.into_result().map_err(f)
    }

    fn map_inner_err<T2, NewE>(self, f: impl FnOnce(E) -> NewE) -> Result<T2, Report<NewE, State>>
    where
        Self: Sized + IntoResult<T2, Report<E, State>>,
        E: Error + Send + Sync + 'static,
        NewE: Error + Send + Sync + 'static,
    {
        self.into_result().map_err(|r| r.map_err(f))
    }

    fn trans_inner_err<T2, NewE>(self) -> Result<T2, Report<NewE, State>>
    where
        Self: Sized + IntoResult<T2, Report<E, State>>,
        E: Error + Send + Sync + 'static,
        E: Into<NewE>,
        NewE: Error + Send + Sync + 'static,
    {
        self.into_result().map_err(|r| r.map_err(|e| e.into()))
    }

    fn into_inner_err<T2>(self) -> Result<T2, E>
    where
        Self: Sized + IntoResult<T2, Report<E, State>>,
    {
        self.into_result().map_err(|r| r.into_inner())
    }

    for_each_report_builder_method!(impl_ext_method);
}

/// Read-only inspection trait for `Result<T, Report<E, State>>`.
///
/// Provides convenient accessors that return `None` on the `Ok` path,
/// avoiding the need to manually match before reading report fields.
pub trait InspectReportExt<T, E, State = MissingSeverity>
where
    State: SeverityState,
{
    /// Returns a reference to the inner `Report` on the error path, or `None`.
    fn report_ref(&self) -> Option<&Report<E, State>>;

    /// Returns a reference to the inner error on the error path, or `None`.
    fn report_inner(&self) -> Option<&E>;

    /// Returns the report's attachments on the error path, or `None`.
    fn report_attachments(&self) -> Option<&[Attachment]>;

    /// Returns the report's context on the error path, or `None`.
    fn report_context(&self) -> Option<&ContextMap>;

    /// Returns the report's system context on the error path, or `None`.
    fn report_system(&self) -> Option<&ContextMap>;

    /// Returns the report's metadata on the error path, or `None`.
    fn report_metadata(&self) -> Option<&ReportMetadata<State>>;

    /// Returns the report's error code on the error path, or `None`.
    fn report_error_code(&self) -> Option<&ErrorCode>;

    /// Returns the report's severity on the error path, or `None`.
    fn report_severity(&self) -> Option<Severity>;

    /// Returns the report's category on the error path, or `None`.
    fn report_category(&self) -> Option<&str>;

    /// Returns whether the report is retryable on the error path, or `None`.
    fn report_retryable(&self) -> Option<bool>;

    /// Returns the report's stack trace on the error path, or `None`.
    fn report_stack_trace(&self) -> Option<&StackTrace>;

    /// Returns the report's options on the error path, or `None`.
    fn report_options(&self) -> Option<&ReportOptions>;

    /// Returns the report's display causes on the error path, or `None`.
    fn report_display_causes(
        &self,
    ) -> Option<&[alloc::sync::Arc<dyn core::fmt::Display + Send + Sync>]>;

    /// Returns an iterator over the report's origin source errors on the error path, or `None`.
    fn report_iter_origin_sources(&self) -> Option<ReportSourceErrorIter<'_>>
    where
        E: Error;

    /// Returns an iterator over the report's diagnostic source errors on the error path, or `None`.
    fn report_iter_diag_sources(&self) -> Option<ReportSourceErrorIter<'_>>
    where
        E: Error;
}

impl<T, E, State> InspectReportExt<T, E, State> for Result<T, Report<E, State>>
where
    State: SeverityState,
{
    fn report_ref(&self) -> Option<&Report<E, State>> {
        self.as_ref().err()
    }

    fn report_inner(&self) -> Option<&E> {
        self.report_ref().map(Report::<E, State>::inner)
    }

    fn report_attachments(&self) -> Option<&[Attachment]> {
        self.report_ref().map(Report::<E, State>::attachments)
    }

    fn report_context(&self) -> Option<&ContextMap> {
        self.report_ref().map(Report::<E, State>::context)
    }

    fn report_system(&self) -> Option<&ContextMap> {
        self.report_ref().map(Report::<E, State>::system)
    }

    fn report_metadata(&self) -> Option<&ReportMetadata<State>> {
        self.report_ref().map(Report::<E, State>::metadata)
    }

    fn report_error_code(&self) -> Option<&ErrorCode> {
        self.report_ref().and_then(Report::<E, State>::error_code)
    }

    fn report_severity(&self) -> Option<Severity> {
        self.report_ref().and_then(Report::<E, State>::severity)
    }

    fn report_category(&self) -> Option<&str> {
        self.report_ref().and_then(Report::<E, State>::category)
    }

    fn report_retryable(&self) -> Option<bool> {
        self.report_ref().and_then(Report::<E, State>::retryable)
    }

    fn report_stack_trace(&self) -> Option<&StackTrace> {
        self.report_ref().and_then(Report::<E, State>::stack_trace)
    }

    fn report_options(&self) -> Option<&ReportOptions> {
        self.report_ref().map(Report::<E, State>::options)
    }

    fn report_display_causes(
        &self,
    ) -> Option<&[alloc::sync::Arc<dyn core::fmt::Display + Send + Sync>]> {
        self.report_ref().map(Report::<E, State>::display_causes)
    }

    fn report_iter_origin_sources(&self) -> Option<ReportSourceErrorIter<'_>>
    where
        E: Error,
    {
        self.report_ref()
            .map(Report::<E, State>::iter_origin_sources)
    }

    fn report_iter_diag_sources(&self) -> Option<ReportSourceErrorIter<'_>>
    where
        E: Error,
    {
        self.report_ref().map(Report::<E, State>::iter_diag_sources)
    }
}
