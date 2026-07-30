use diagweave::{DiagnosticError, set};
use std::fmt::Display;

set! {
    #[derive(Clone, Debug, PartialEq)]
    pub SetGenericA<T: Display + std::fmt::Debug + Send + Sync + 'static> = {
        #[display("item: {0}")]
        Item(T),
    }

    #[derive(Clone, Debug, PartialEq)]
    pub SetGenericB<T> where T: Display + std::fmt::Debug + Send + Sync + 'static = SetGenericA<T> | {
        #[display("custom failure")]
        Custom,
    }

    #[derive(Clone, Debug)]
    pub SetLifetime<'a: 'static> = {
        #[display("borrowed msg: {0}")]
        Borrowed(&'a str),
    }

    #[derive(Clone, Debug)]
    pub SetSuperLifetime<'a: 'static> = SetLifetime<'a> | {
        #[display("owned: {0}")]
        Owned(String),
    }
}

#[test]
fn test_generic_set_basic() {
    let a: SetGenericA<i32> = SetGenericA::Item(42);
    assert_eq!(a.to_string(), "item: 42");

    let b: SetGenericB<i32> = a.into();
    assert_eq!(b, SetGenericB::Item(42));
    assert_eq!(b.to_string(), "item: 42");
}

#[test]
fn test_generic_set_diagnostic_helpers() {
    let a = SetGenericA::Item("hello".to_string());
    let report = a.to_report();
    assert_eq!(report.to_string(), "item: hello");

    let a_trans = SetGenericA::Item(100);
    let report_trans: diagweave::report::Report<SetGenericB<i32>> = a_trans.to_report_trans();
    assert_eq!(report_trans.inner(), &SetGenericB::Item(100));
}

#[test]
fn test_lifetime_set() {
    let msg: &'static str = "temporary buffer";
    let s: SetLifetime<'static> = SetLifetime::Borrowed(msg);
    assert_eq!(s.to_string(), "borrowed msg: temporary buffer");

    let sup: SetSuperLifetime<'static> = s.into();
    assert_eq!(sup.to_string(), "borrowed msg: temporary buffer");
}
