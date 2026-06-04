use diagweave::{DiagnosticError, Error};

#[derive(Debug, Error)]
enum DemoError {
    #[display("not found: {id}")]
    NotFound { id: u64 },
    #[display(transparent)]
    Io(#[from] std::io::Error),
    #[display("upstream failed: {0}")]
    Upstream(#[source] std::io::Error),
    #[display("{step} 失败（退出码: {code:?}）")]
    SetupFailed { step: String, code: Option<i32> },
}

#[derive(Debug, Error)]
#[display("client error code={code}")]
struct ClientError {
    code: u16,
}

#[test]
fn derive_error_template_and_transparent_display_work() {
    let msg = DemoError::NotFound { id: 7 };
    assert_eq!(msg.to_string(), "not found: 7");

    let io: DemoError = std::io::Error::other("socket closed").into();
    assert_eq!(io.to_string(), "socket closed");

    let setup_failed = DemoError::SetupFailed {
        step: "编译".to_string(),
        code: Some(1),
    };
    assert_eq!(setup_failed.to_string(), "编译 失败（退出码: Some(1)）");
}

#[test]
fn derive_source_errors_and_diag_work() {
    let up = DemoError::Upstream(std::io::Error::other("db down"));
    let src = up.source().expect("source exists");
    assert_eq!(src.to_string(), "db down");

    let report = ClientError { code: 403 }.to_report();
    assert_eq!(report.into_inner().to_string(), "client error code=403");
}

#[test]
fn derive_direct_diag_on_client_error() {
    // Directly call diag on a ClientError instance (inherent method provided by macro)
    let report = ClientError { code: 403 }.attach_note("note");
    // Ensure we got a report and inner error renders as expected
    assert_eq!(report.inner().to_string(), "client error code=403");
}

#[derive(Debug, Error)]
enum AppError {
    #[display("net error: {0}")]
    Net(#[from] DemoError),
}

#[test]
fn test_generic_from_conversion_for_report() {
    use diagweave::report::Report;

    // Test direct .into() conversion: DemoError -> Report<AppError>
    let demo = DemoError::NotFound { id: 101 };
    let report: Report<AppError> = demo.into();

    match report.inner() {
        AppError::Net(DemoError::NotFound { id }) => assert_eq!(*id, 101),
        _ => panic!("unexpected inner error"),
    }

    // Test automatic conversion with `?` operator in a function returning Result<_, Report<AppError>>
    fn trigger_error() -> Result<(), Report<AppError>> {
        let res: Result<(), DemoError> = Err(DemoError::NotFound { id: 202 });
        res?; // should automatically convert via From to Report<AppError>!
        Ok(())
    }

    let err_report = trigger_error().unwrap_err();
    match err_report.inner() {
        AppError::Net(DemoError::NotFound { id }) => assert_eq!(*id, 202),
        _ => panic!("unexpected inner error from propagation"),
    }
}

#[test]
fn test_res_to_report_trans() {
    use diagweave::prelude::*;
    let res: Result<(), DemoError> = Err(DemoError::NotFound { id: 101 });
    let report_res: Result<(), Report<AppError>> = res.to_report_res_trans::<_, AppError>();

    assert!(report_res.is_err());
    let report = report_res.unwrap_err();
    match report.inner() {
        AppError::Net(DemoError::NotFound { id }) => assert_eq!(*id, 101),
        _ => panic!("unexpected inner error"),
    }
}

#[test]
fn test_err_to_report_trans() {
    use diagweave::prelude::*;
    let err = DemoError::NotFound { id: 101 };
    let report = err.to_report_trans();

    match report.inner() {
        AppError::Net(DemoError::NotFound { id }) => assert_eq!(*id, 101),
        _ => panic!("unexpected inner error"),
    }
}
