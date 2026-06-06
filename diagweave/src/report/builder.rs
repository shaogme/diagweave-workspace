//! Builder methods for Report.
//!
//! This module contains all builder-style methods for constructing and
//! configuring Report instances. These methods follow the builder pattern,
//! consuming `self` and returning a modified `Self`.

use alloc::sync::Arc;
use core::error::Error;
use core::fmt::Display;
use ref_str::StaticRefStr;

use super::types::{
    Attachment, AttachmentValue, ContextValue, DisplayCauseChain, ErrorCode, SourceErrorChain,
    StackTrace, append_source_chain,
};
use super::{Report, ReportMetadata, ReportOptions, SeverityState};

impl<E, State> Report<E, State>
where
    State: SeverityState,
{
    /// Adds a business context key-value pair to the report if the key is absent.
    ///
    /// Business context provides additional information about the error's
    /// operational context, such as user IDs, request IDs, or other
    /// domain-specific metadata. Existing values are preserved when the
    /// same key is used more than once.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .with_ctx("user_id", "12345")
    ///     .with_ctx("request_id", "abc-def-ghi");
    /// ```
    pub fn with_ctx(
        mut self,
        key: impl Into<StaticRefStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).insert_context_if_absent(key, value);
        self
    }

    /// Adds a business context key with multiple values if the key is absent.
    ///
    /// Existing values are preserved when the same key is already present.
    pub fn with_ctx_values<I, V>(mut self, key: impl Into<StaticRefStr>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<ContextValue>,
    {
        Report::<E, State>::diagnostics_mut(&mut self).insert_context_values_if_absent(key, values);
        self
    }

    /// Appends a business context value for the key.
    ///
    /// Unlike [`Report::with_ctx`], this method preserves existing values and adds
    /// a new repeated entry for the key.
    pub fn push_ctx(
        mut self,
        key: impl Into<StaticRefStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).push_context(key, value);
        self
    }

    /// Appends a business context value for the key.
    ///
    /// This is an alias for [`Report::push_ctx`].
    pub fn append_ctx(self, key: impl Into<StaticRefStr>, value: impl Into<ContextValue>) -> Self {
        Report::<E, State>::push_ctx(self, key, value)
    }

    /// Adds a system context key-value pair to the report if the key is absent.
    ///
    /// System context contains infrastructure-level information such as
    /// hostname, service name, deployment environment, etc. Existing values
    /// are preserved when the same key is used more than once.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .with_system("hostname", "prod-server-01")
    ///     .with_system("service", "payment-service");
    /// ```
    pub fn with_system(
        mut self,
        key: impl Into<StaticRefStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).insert_system_if_absent(key, value);
        self
    }

    /// Adds a system context key with multiple values if the key is absent.
    ///
    /// Existing values are preserved when the same key is already present.
    pub fn with_system_values<I, V>(mut self, key: impl Into<StaticRefStr>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<ContextValue>,
    {
        Report::<E, State>::diagnostics_mut(&mut self).insert_system_values_if_absent(key, values);
        self
    }

    /// Appends a system context value for the key.
    pub fn push_system(
        mut self,
        key: impl Into<StaticRefStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).push_system(key, value);
        self
    }

    /// Appends a system context value for the key.
    ///
    /// This is an alias for [`Report::push_system`].
    pub fn append_system(
        self,
        key: impl Into<StaticRefStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        Report::<E, State>::push_system(self, key, value)
    }

    /// Sets a system context key-value pair for the report.
    ///
    /// System context contains infrastructure-level information such as
    /// hostname, service name, deployment environment, etc. Existing values
    /// for the same key are replaced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .set_system("hostname", "prod-server-01")
    ///     .set_system("service", "payment-service");
    /// ```
    pub fn set_system(
        mut self,
        key: impl Into<StaticRefStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).insert_system(key, value);
        self
    }

    /// Sets all system context values for the key, replacing any existing values.
    pub fn set_system_values<I, V>(mut self, key: impl Into<StaticRefStr>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<ContextValue>,
    {
        Report::<E, State>::diagnostics_mut(&mut self).insert_system_values(key, values);
        self
    }

    /// Sets a business context key-value pair for the report.
    ///
    /// Business context provides additional information about the error's
    /// operational context, such as user IDs, request IDs, or other
    /// domain-specific metadata. Existing values for the same key are replaced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .set_ctx("user_id", "12345")
    ///     .set_ctx("request_id", "abc-def-ghi");
    /// ```
    pub fn set_ctx(mut self, key: impl Into<StaticRefStr>, value: impl Into<ContextValue>) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).insert_context(key, value);
        self
    }

    /// Sets all business context values for the key, replacing any existing values.
    pub fn set_ctx_values<I, V>(mut self, key: impl Into<StaticRefStr>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<ContextValue>,
    {
        Report::<E, State>::diagnostics_mut(&mut self).insert_context_values(key, values);
        self
    }

    /// Attaches a printable note to the report.
    ///
    /// Notes are human-readable messages that provide additional context
    /// about the error. They are displayed in pretty-printed output and
    /// included in JSON representations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .attach_printable("This error occurred while processing payment")
    ///     .attach_printable("User was attempting to checkout cart #12345");
    /// ```
    pub fn attach_printable(mut self, message: impl Display + Send + Sync + 'static) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).add_attachment(Attachment::note(message));
        self
    }

    /// Attaches a payload with an optional media type to the report.
    ///
    /// Payloads are structured data attachments that can contain arbitrary
    /// data. They are useful for attaching debugging information, request
    /// bodies, or other structured data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .attach_payload("request_body", r#"{"amount": 100}"#, Some("application/json"))
    ///     .attach_payload("debug_info", vec!["step1", "step2"], None::<&str>);
    /// ```
    pub fn attach_payload(
        mut self,
        name: impl Into<StaticRefStr>,
        value: impl Into<AttachmentValue>,
        media_type: Option<impl Into<StaticRefStr>>,
    ) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self)
            .add_attachment(Attachment::payload(name, value, media_type));
        self
    }

    /// Adds a note to the report (alias for `attach_printable`).
    ///
    /// This is a convenience alias for [`Report::attach_printable`].
    pub fn attach_note(self, message: impl Display + Send + Sync + 'static) -> Self {
        Report::<E, State>::attach_printable(self, message)
    }

    /// Sets the metadata for the report.
    ///
    /// Metadata contains error code, category, and retryable flag.
    /// This method replaces any existing metadata entirely.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::report::ReportMetadata;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let metadata = ReportMetadata::new()
    ///     .set_error_code("ERR-001")
    ///     .set_category("payment")
    ///     .set_retryable(true);
    ///
    /// let report = Report::new(MyError).with_metadata(metadata);
    /// ```
    pub fn with_metadata(mut self, metadata: ReportMetadata<State>) -> Self {
        self.data.metadata = metadata;
        self
    }

    /// Sets the error code for the report, replacing any existing value.
    ///
    /// Error codes are machine-readable identifiers that can be used for
    /// error categorization and automated handling.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError).set_error_code("PAYMENT_FAILED");
    /// ```
    pub fn set_error_code(mut self, error_code: impl Into<ErrorCode>) -> Self {
        self.data.metadata.set_error_code_mut(error_code);
        self
    }

    /// Sets the error code only if not already set.
    ///
    /// This is useful for setting default error codes while allowing
    /// explicit codes to take precedence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// // First call sets the error code
    /// let report = Report::new(MyError).with_error_code("ERR-001");
    ///
    /// // Second call is ignored because error code is already set
    /// let report = report.with_error_code("ERR-002");
    /// assert_eq!(report.error_code().unwrap().to_string(), "ERR-001".to_string());
    /// ```
    pub fn with_error_code(mut self, error_code: impl Into<ErrorCode>) -> Self {
        self.data.metadata.with_error_code_mut(error_code);
        self
    }

    /// Sets the category for the report, replacing any existing value.
    ///
    /// Categories are used to group related errors together for analysis
    /// and reporting purposes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError).set_category("payment");
    /// ```
    pub fn set_category(mut self, category: impl Into<StaticRefStr>) -> Self {
        self.data.metadata.set_category_mut(category);
        self
    }

    /// Sets the category only if not already set.
    ///
    /// This is useful for setting default categories while allowing
    /// explicit categories to take precedence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .with_category("payment")
    ///     .with_category("checkout"); // Ignored, category already set
    /// assert_eq!(report.category(), Some("payment"));
    /// ```
    pub fn with_category(mut self, category: impl Into<StaticRefStr>) -> Self {
        self.data.metadata.with_category_mut(category);
        self
    }

    /// Sets whether the error is retryable, replacing any existing value.
    ///
    /// This flag indicates whether the operation that caused the error
    /// can be safely retried.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError).set_retryable(true);
    /// ```
    pub fn set_retryable(mut self, retryable: bool) -> Self {
        self.data.metadata.set_retryable_mut(retryable);
        self
    }

    /// Sets whether the error is retryable only if not already set.
    ///
    /// This is useful for setting default retryable behavior while allowing
    /// explicit settings to take precedence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .with_retryable(true)
    ///     .with_retryable(false); // Ignored, retryable already set
    /// assert_eq!(report.retryable(), Some(true));
    /// ```
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.data.metadata.with_retryable_mut(retryable);
        self
    }

    /// Sets the stack trace for the report, replacing any existing value.
    ///
    /// Stack traces provide debugging information about where the error
    /// occurred in the code.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "std")]
    /// # {
    /// use diagweave::prelude::Report;
    /// use diagweave::report::StackTrace;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let stack_trace = StackTrace::capture_raw();
    /// let report = Report::new(MyError).set_stack_trace(stack_trace);
    /// # }
    /// ```
    pub fn set_stack_trace(mut self, stack_trace: StackTrace) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).set_stack_trace(stack_trace);
        self
    }

    /// Sets the stack trace only if not already present.
    ///
    /// This is useful for conditionally capturing stack traces.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "std")]
    /// # {
    /// use diagweave::prelude::Report;
    /// use diagweave::report::StackTrace;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// // Only capture if not already set
    /// let report = Report::new(MyError).with_stack_trace(StackTrace::capture_raw());
    /// # }
    /// ```
    pub fn with_stack_trace(mut self, stack_trace: StackTrace) -> Self {
        if Report::<E, State>::stack_trace(&self).is_none() {
            Report::<E, State>::diagnostics_mut(&mut self).set_stack_trace(stack_trace);
        }
        self
    }

    /// Clears the stack trace from the report.
    ///
    /// This can be useful when you want to remove potentially sensitive
    /// stack information before serializing or logging.
    pub fn clear_stack_trace(mut self) -> Self {
        *Report::<E, State>::diagnostics_mut(&mut self).stack_trace_mut() = None;
        self
    }

    /// Captures the stack trace for the report if not already present.
    ///
    /// This is a convenience method that captures the current stack trace
    /// only if one hasn't been set yet.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError).capture_stack_trace();
    /// ```
    #[cfg(feature = "std")]
    pub fn capture_stack_trace(mut self) -> Self {
        if Report::<E, State>::stack_trace(&self).is_none() {
            Report::<E, State>::diagnostics_mut(&mut self)
                .set_stack_trace(StackTrace::capture_raw());
        }
        self
    }

    /// Forcefully captures the stack trace for the report.
    ///
    /// Unlike [`Report::capture_stack_trace`], this method always captures
    /// a new stack trace, replacing any existing one.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError).force_capture_stack();
    /// ```
    #[cfg(feature = "std")]
    pub fn force_capture_stack(mut self) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).set_stack_trace(StackTrace::capture_raw());
        self
    }

    /// Adds a display cause to the report.
    ///
    /// Display causes are human-readable messages that provide additional
    /// context about the error. They are separate from the error chain
    /// and are displayed alongside the error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let report = Report::new(MyError)
    ///     .with_display_cause("Failed to connect to database")
    ///     .with_display_cause("Connection timeout after 30s");
    /// ```
    pub fn with_display_cause(mut self, cause: impl Display + Send + Sync + 'static) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self)
            .display_causes_mut()
            .get_or_insert_with(DisplayCauseChain::default)
            .items
            .push(Arc::new(cause) as Arc<dyn Display + Send + Sync + 'static>);
        self
    }

    /// Replaces the display-cause chain for the report.
    ///
    /// This method completely replaces any existing display causes with
    /// the provided chain.
    pub fn set_display_causes(mut self, display_causes: DisplayCauseChain) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).set_display_causes(display_causes);
        self
    }

    /// Adds multiple display causes to the report.
    ///
    /// This is a convenience method for adding multiple display causes
    /// at once.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let causes = vec![
    ///     "Failed to connect to database",
    ///     "Connection timeout after 30s",
    /// ];
    /// let report = Report::new(MyError).with_display_causes(causes);
    /// ```
    pub fn with_display_causes<I, T>(mut self, causes: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Display + Send + Sync + 'static,
    {
        Report::<E, State>::diagnostics_mut(&mut self)
            .display_causes_mut()
            .get_or_insert_with(DisplayCauseChain::default)
            .items
            .extend(
                causes
                    .into_iter()
                    .map(|cause| Arc::new(cause) as Arc<dyn Display + Send + Sync + 'static>),
            );
        self
    }

    /// Adds an error source to the report's diagnostic source chain.
    ///
    /// Diagnostic source errors are separate from the origin source chain.
    /// They represent additional errors that are related to but not directly
    /// caused by the main error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("main error")]
    /// struct MainError;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("related error")]
    /// struct RelatedError;
    ///
    /// let report = Report::new(MainError)
    ///     .with_diag_src_err(RelatedError);
    /// ```
    pub fn with_diag_src_err(mut self, err: impl Error + Send + Sync + 'static) -> Self {
        let existing = self
            .diagnostics_mut()
            .diag_src_errors_mut()
            .get_or_insert_with(SourceErrorChain::default);
        append_source_chain(existing, SourceErrorChain::from_error(err));
        self
    }

    /// Replaces the diagnostic source-error chain for the report.
    ///
    /// This method completely replaces any existing diagnostic source
    /// errors with the provided chain.
    pub fn set_diag_src_errs(mut self, source_errors: SourceErrorChain) -> Self {
        Report::<E, State>::diagnostics_mut(&mut self).set_diag_src_errors(source_errors);
        self
    }

    /// Sets the report options for this report.
    ///
    /// This replaces any existing options with the provided ones.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::report::ReportOptions;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let my_error = MyError;
    /// let report = Report::new(my_error);
    ///
    /// // Disable source chain accumulation for this specific report
    /// let _report = report.set_options(ReportOptions::new().with_accumulate_src_chain(false));
    /// ```
    pub fn set_options(mut self, options: ReportOptions) -> Self {
        self.data.options = options;
        self
    }

    /// Sets whether source chains should be accumulated during `map_err()`.
    ///
    /// This is a convenience method for setting the `accumulate_src_chain` option.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diagweave::prelude::Report;
    /// use diagweave::Error;
    ///
    /// #[derive(Debug, Error)]
    /// #[display("my error")]
    /// struct MyError;
    ///
    /// let my_error = MyError;
    /// let report = Report::new(my_error);
    ///
    /// // Enable source chain accumulation for this specific report
    /// let _report = report.set_accumulate_src_chain(true);
    /// ```
    pub fn set_accumulate_src_chain(mut self, accumulate: bool) -> Self {
        self.data.options = self.data.options.with_accumulate_src_chain(accumulate);
        self
    }
}
