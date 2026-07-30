use diagweave::{DiagnosticError, union};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericInnerError<T>(pub T);

impl<T: Display> Display for GenericInnerError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic inner error: {}", self.0)
    }
}

impl<T: Display + std::fmt::Debug> Error for GenericInnerError<T> {}

union! {
    #[derive(Clone, Debug, PartialEq)]
    pub enum UnionGeneric<T> where T: Display + std::fmt::Debug + Send + Sync + 'static =
        GenericInnerError<T> |
        {
            #[display("inline value: {0}")]
            InlineValue(T),
            #[display("static message")]
            StaticMsg,
        }
}

union! {
    #[derive(Clone, Debug)]
    pub enum UnionLifetime<'a: 'static> = {
        #[display("ref error: {0}")]
        RefErr(&'a str),
    }
}

#[test]
fn test_union_generic_external_type() {
    let inner = GenericInnerError("database timeout");
    let u: UnionGeneric<&'static str> = inner.into();
    match &u {
        UnionGeneric::GenericInnerError(err) => assert_eq!(err.0, "database timeout"),
        _ => panic!("unexpected variant"),
    }
    assert_eq!(u.to_string(), "generic inner error: database timeout");
}

#[test]
fn test_union_generic_inline_variant() {
    let u = UnionGeneric::InlineValue(404);
    assert_eq!(u.to_string(), "inline value: 404");

    let report = u.to_report();
    assert_eq!(report.to_string(), "inline value: 404");
}

#[test]
fn test_union_lifetime() {
    let str_val: &'static str = "bad parameter";
    let u = UnionLifetime::RefErr(str_val);
    assert_eq!(u.to_string(), "ref error: bad parameter");
}
