use diagweave::{DiagnosticError, set};

set! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    SetA = {
        Variant1,
    }

    #[derive(Clone, Debug, PartialEq)]
    SetB = SetA | {
        Variant2(String),
    }

    #[derive(Clone)]
    SetC = {
        Variant3,
    }

    #[derive(Clone, Debug, PartialEq)]
    SetD = {
        #[display("{step} 失败（退出码: {code:?}）")]
        SetupFailed {
            step: String,
            code: Option<i32>,
        },
    }
}

#[test]
fn test_set_derives_clone_copy() {
    let a = SetA::Variant1;
    let b = a; // Copy works
    assert_eq!(a, b); // PartialEq works
}

#[test]
fn test_set_derives_clone_only() {
    let b = SetB::Variant2("hello".to_string());
    let b2 = b.clone(); // Clone works
    assert_eq!(b, b2); // PartialEq works
}

#[test]
fn test_set_derives_implicit_debug() {
    let c = SetC::Variant3;
    let dbg = format!("{:?}", c); // Debug works (it's always added)
    assert!(dbg.contains("Variant3"));
}

#[test]
fn test_set_display_with_format_specifier() {
    let d = SetD::SetupFailed {
        step: "运行".to_string(),
        code: Some(2),
    };
    assert_eq!(d.to_string(), "运行 失败（退出码: Some(2)）");
}

#[test]
fn test_conversion_between_differently_derived_sets() {
    let a = SetA::Variant1;
    let b: SetB = a.into(); // Conversion works
    match b {
        SetB::Variant1 => {}
        _ => panic!("unexpected"),
    }
}

#[test]
fn test_generic_from_conversion_for_set_errors() {
    use diagweave::report::Report;

    // SetA -> Report<SetB>
    let err_a = SetA::Variant1;
    let report: Report<SetB> = err_a.into();

    match report.inner() {
        SetB::Variant1 => {}
        _ => panic!("unexpected inner error in set report"),
    }
}

#[test]
fn test_set_to_report_trans() {
    let err_a = SetA::Variant1;
    let report = err_a.to_report_trans();

    match report.inner() {
        SetB::Variant1 => {}
        _ => panic!("unexpected inner error in set report"),
    }
}

set! {
    SetDedupA = {
        #[display("shared variant code={code}")]
        SharedVariant { code: u32 },
        #[display("only A")]
        OnlyA,
    }

    SetDedupB = {
        #[display("shared variant code={code}")]
        SharedVariant { code: u32 },
        #[display("only B")]
        OnlyB,
    }

    SetDedupUnion = SetDedupA | SetDedupB | {
        #[display("shared variant code={code}")]
        SharedVariant { code: u32 },
        #[display("only Union")]
        OnlyUnion,
    }
}

#[test]
fn test_non_generic_set_deduplication() {
    let a = SetDedupA::SharedVariant { code: 10 };
    let u: SetDedupUnion = a.into();
    assert_eq!(u.to_string(), "shared variant code=10");

    let a_only = SetDedupA::OnlyA;
    let u_a: SetDedupUnion = a_only.into();
    assert_eq!(u_a.to_string(), "only A");

    let b = SetDedupB::SharedVariant { code: 20 };
    let u2: SetDedupUnion = b.into();
    assert_eq!(u2.to_string(), "shared variant code=20");

    let b_only = SetDedupB::OnlyB;
    let u3: SetDedupUnion = b_only.into();
    match u3 {
        SetDedupUnion::OnlyB => {}
        _ => panic!("unexpected variant"),
    }

    let u_union = SetDedupUnion::OnlyUnion;
    assert_eq!(u_union.to_string(), "only Union");
}
