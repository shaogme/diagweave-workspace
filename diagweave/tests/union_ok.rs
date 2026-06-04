use diagweave::{DiagnosticError, Report, union};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Test AuthError.
pub enum AuthError {
    InvalidToken,
}

impl Display for AuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "auth token invalid"),
        }
    }
}

impl Error for AuthError {}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Test DbError.
pub enum DbError {
    ConnectionLost,
}

impl Display for DbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionLost => write!(f, "db connection lost"),
        }
    }
}

impl Error for DbError {}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Simple wrapper error used in union macro tests.
pub struct SimpleError(&'static str);

impl Display for SimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "simple error: {}", self.0)
    }
}

impl Error for SimpleError {}

union! {
    #[derive(Clone)]
    pub enum ApiError = AuthError | DbError | {
        #[display("Rate limited for {retry_after_secs}s")]
        RateLimited { retry_after_secs: u64 },
        #[display(transparent)]
        Transparent(u32),
        #[from]
        Simple(SimpleError),
        #[display("Escaped braces {{db}} code={0}")]
        TupleEscaped(u32),
        #[display("Internal error: {message:?}")]
        Internal { message: String },
    }
}

#[test]
fn wraps_external_error_types() {
    let auth = AuthError::InvalidToken;
    let api: ApiError = auth.into();
    match api {
        ApiError::AuthError(inner) => assert_eq!(inner, AuthError::InvalidToken),
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn keeps_inline_variants() {
    let api = ApiError::RateLimited {
        retry_after_secs: 10,
    };
    match api {
        ApiError::RateLimited { retry_after_secs } => assert_eq!(retry_after_secs, 10),
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn converts_second_external_type() {
    let db = DbError::ConnectionLost;
    let api: ApiError = db.into();
    match api {
        ApiError::DbError(inner) => assert_eq!(inner, DbError::ConnectionLost),
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn supports_alias_for_external_types() {
    union! {
        enum AliasedError = AuthError as Auth | DbError as Database
    }

    let auth: AliasedError = AuthError::InvalidToken.into();
    match auth {
        AliasedError::Auth(inner) => assert_eq!(inner, AuthError::InvalidToken),
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn union_display_works_for_wrapped_and_inline_variants() {
    let wrapped: ApiError = AuthError::InvalidToken.into();
    let inline = ApiError::RateLimited {
        retry_after_secs: 12,
    };
    let transparent = ApiError::Transparent(42);
    let escaped = ApiError::TupleEscaped(88);
    let internal = ApiError::Internal {
        message: "something broke".to_string(),
    };
    assert_eq!(wrapped.to_string(), "auth token invalid");
    assert_eq!(inline.to_string(), "Rate limited for 12s");
    assert_eq!(transparent.to_string(), "42");
    assert_eq!(escaped.to_string(), "Escaped braces {db} code=88");
    assert_eq!(internal.to_string(), "Internal error: \"something broke\"");
    let dbg = format!("{:?}", inline);
    assert!(dbg.contains("RateLimited"));
}

#[test]
fn from_attribute_generates_from_impls() {
    let err = SimpleError("boom");
    let api: ApiError = err.into();
    match api {
        ApiError::Simple(inner) => assert_eq!(inner, SimpleError("boom")),
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn union_enum_provides_diag_helpers() {
    let api = ApiError::RateLimited {
        retry_after_secs: 8,
    };
    let report: Report<ApiError> = api.clone().to_report();
    assert_eq!(report.to_string(), "Rate limited for 8s");
    assert!(api.source().is_none());
}

union! {
    pub enum OuterError = ApiError
}

#[test]
fn test_generic_from_conversion_for_union_errors() {
    use diagweave::report::Report;

    // ApiError -> Report<OuterError>
    let api = ApiError::RateLimited {
        retry_after_secs: 5,
    };
    let report: Report<OuterError> = api.into();

    match report.inner() {
        OuterError::ApiError(ApiError::RateLimited { retry_after_secs }) => {
            assert_eq!(*retry_after_secs, 5)
        }
        _ => panic!("unexpected inner error in union report"),
    }
}

#[test]
fn test_union_to_report_trans() {
    use diagweave::report::Report;

    let api = ApiError::RateLimited {
        retry_after_secs: 5,
    };
    let report: Report<OuterError> = api.to_report();

    match report.inner() {
        OuterError::ApiError(ApiError::RateLimited { retry_after_secs }) => {
            assert_eq!(*retry_after_secs, 5)
        }
        _ => panic!("unexpected inner error in union report"),
    }
}
