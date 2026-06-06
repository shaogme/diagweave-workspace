/// A macro that applies a callback macro to all `Report` builder methods.
macro_rules! for_each_report_builder_method {
    ($callback:ident) => {
        $callback! {
            /// Adds a business context key-value pair to the report if the key is absent.
            fn with_ctx(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Adds business context values to the report if the key is absent.
            fn with_ctx_values<I, V>(key: impl Into<crate::StaticRefStr>, values: I) -> Self
            where
                I: IntoIterator<Item = V>,
                V: Into<crate::report::ContextValue>
        }
        $callback! {
            /// Appends a business context key-value pair to the report.
            fn push_ctx(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Appends a business context key-value pair to the report.
            fn append_ctx(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Adds a system context key-value pair to the report if the key is absent.
            fn with_system(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Adds system context values to the report if the key is absent.
            fn with_system_values<I, V>(key: impl Into<crate::StaticRefStr>, values: I) -> Self
            where
                I: IntoIterator<Item = V>,
                V: Into<crate::report::ContextValue>
        }
        $callback! {
            /// Appends a system context key-value pair to the report.
            fn push_system(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Appends a system context key-value pair to the report.
            fn append_system(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Sets a system context key-value pair, replacing any existing value for the key.
            fn set_system(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Sets system context values, replacing any existing values for the key.
            fn set_system_values<I, V>(key: impl Into<crate::StaticRefStr>, values: I) -> Self
            where
                I: IntoIterator<Item = V>,
                V: Into<crate::report::ContextValue>
        }
        $callback! {
            /// Sets a business context key-value pair, replacing any existing value for the key.
            fn set_ctx(key: impl Into<crate::StaticRefStr>, value: impl Into<crate::report::ContextValue>) -> Self
        }
        $callback! {
            /// Sets business context values, replacing any existing values for the key.
            fn set_ctx_values<I, V>(key: impl Into<crate::StaticRefStr>, values: I) -> Self
            where
                I: IntoIterator<Item = V>,
                V: Into<crate::report::ContextValue>
        }
        $callback! {
            /// Attaches a printable note to the report.
            fn attach_printable(message: impl core::fmt::Display + Send + Sync + 'static) -> Self
        }
        $callback! {
            /// Attaches a payload with an optional media type to the report.
            fn attach_payload(
                name: impl Into<crate::StaticRefStr>,
                value: impl Into<crate::report::AttachmentValue>,
                media_type: Option<impl Into<crate::StaticRefStr>>
            ) -> Self
        }
        $callback! {
            /// Adds a note to the report (alias for `attach_printable`).
            fn attach_note(message: impl core::fmt::Display + Send + Sync + 'static) -> Self
        }
        $callback! {
            /// Sets the metadata for the report.
            fn with_metadata(metadata: crate::report::ReportMetadata<State>) -> Self
        }
        $callback! {
            /// Sets the error code for the report, replacing any existing value.
            fn set_error_code(error_code: impl Into<crate::report::ErrorCode>) -> Self
        }
        $callback! {
            /// Sets the error code only if not already set.
            fn with_error_code(error_code: impl Into<crate::report::ErrorCode>) -> Self
        }
        $callback! {
            /// Sets the category for the report, replacing any existing value.
            fn set_category(category: impl Into<crate::StaticRefStr>) -> Self
        }
        $callback! {
            /// Sets the category only if not already set.
            fn with_category(category: impl Into<crate::StaticRefStr>) -> Self
        }
        $callback! {
            /// Sets whether the error is retryable, replacing any existing value.
            fn set_retryable(retryable: bool) -> Self
        }
        $callback! {
            /// Sets whether the error is retryable only if not already set.
            fn with_retryable(retryable: bool) -> Self
        }
        $callback! {
            /// Sets the stack trace for the report, replacing any existing value.
            fn set_stack_trace(stack_trace: crate::report::StackTrace) -> Self
        }
        $callback! {
            /// Sets the stack trace only if not already present.
            fn with_stack_trace(stack_trace: crate::report::StackTrace) -> Self
        }
        $callback! {
            /// Clears the stack trace from the report.
            fn clear_stack_trace() -> Self
        }
        #[cfg(feature = "std")]
        $callback! {
            /// Captures the stack trace for the report if not already present.
            fn capture_stack_trace() -> Self
        }
        #[cfg(feature = "std")]
        $callback! {
            /// Forcefully captures the stack trace for the report.
            fn force_capture_stack() -> Self
        }
        $callback! {
            /// Adds a display cause to the report.
            fn with_display_cause(cause: impl core::fmt::Display + Send + Sync + 'static) -> Self
        }
        $callback! {
            /// Adds multiple display causes to the report.
            fn with_display_causes<I, C>(causes: I) -> Self
            where
                I: IntoIterator<Item = C>,
                C: core::fmt::Display + Send + Sync + 'static
        }
        $callback! {
            /// Replaces the display-cause chain for the report.
            fn set_display_causes(display_causes: crate::report::DisplayCauseChain) -> Self
        }
        $callback! {
            /// Adds an error source to the report's diagnostic source chain.
            fn with_diag_src_err(err: impl core::error::Error + Send + Sync + 'static) -> Self
        }
        $callback! {
            /// Replaces the diagnostic source-error chain for the report.
            fn set_diag_src_errs(source_errors: crate::report::SourceErrorChain) -> Self
        }
        $callback! {
            /// Sets the report options for this report.
            fn set_options(options: crate::report::ReportOptions) -> Self
        }
        $callback! {
            /// Sets whether source chains should be accumulated during `map_err()`.
            fn set_accumulate_src_chain(accumulate: bool) -> Self
        }
        $callback! {
            /// Sets the severity for the report, transitioning to `HasSeverity` typestate.
            fn set_severity(severity: crate::report::Severity) -> crate::report::HasSeverity [STATE_CHANGE]
        }
        $callback! {
            /// Sets the severity only if not already set, transitioning to `HasSeverity` typestate.
            fn with_severity(severity: crate::report::Severity) -> crate::report::HasSeverity [STATE_CHANGE]
        }
    };
}
