# Error Definition and Conversion

## 1. `set!` Macro

### Overview
Used to define a series of structured error enums (Error Sets). It automatically implements composition logic between sets, `From` conversions, report semantics, and implements the `DiagnosticError` trait (providing `to_report()`, `to_report_trans()`, and direct diagnostic builder methods) as well as generating `source()` helper methods.

### Syntax Definition
```rust, ignore
set! {
    Ident = { [VariantDecls] } [ | OtherSet ]
    ...
}
```


### Supported Attributes
| Attribute | Scope | Parameters | Description |
| :--- | :--- | :--- | :--- |
| `#[display("...")]`| Variant | Format string | Use `{field}` or `{0}` to reference named fields or anonymous tuple fields |
| `#[display(transparent)]` | Variant | None | Delegate `Display` directly to the inner field (requires exactly 1 field) |
| `#[from]` | Variant | None | Mark that this variant can be directly converted from its single field type |

### Core Usage
```rust
use diagweave::set;

set! {
    AuthError = {
        #[display("user {id} not found")]
        UserNotFound { id: u64 },
        
        #[display(transparent)]
        Io(#[from] std::io::Error),
    }

    ServiceError = AuthError | {
        #[display("unexpected error")]
        Unknown
    }
}
```

### Generated Methods (Example: `AuthError`)
| Declaration | Return Type | Description |
| :--- | :--- | :--- |
| `DiagnosticError::to_report(self)` | `Report<Self>` | (From `DiagnosticError` trait) Converts error instance into a report of the same error type (requires `Self: Sized`). |
| `DiagnosticError::to_report_trans::<NewE>(self)` | `Report<NewE>` | (From `DiagnosticError` trait) Converts error instance into a report of a different error type (requires `Self: Into<NewE>`). |
| `DiagnosticError::[builder_method](self, ...)` | `Report<Self, _>` | (From `DiagnosticError` trait) Direct chained diagnostic construction method, bypassing manual conversion |
| `AuthError::source(&self)` | `Option<&dyn Error>` | Access to the underlying error source |
| `From<AuthError> for ServiceError` | `ServiceError` | Automatic mapping from subset to superset |

---

## 2. `union!` Macro

### Overview
Used at architecture boundaries to combine unrelated error types, other error sets, or inline-defined variants.

### Syntax Definition
```rust, ignore
union! {
    [Attributes]
    [vis] enum Ident = Item1 | Item2 | ...
}
```

### Declaration Items (UnionItem)
| Item Type | Syntax | Description |
| :--- | :--- | :--- |
| External Type | `Path` | Auto-implements `From<Path>` and delegates `Display` |
| External Type Alias | `Path as Ident` | Wraps Path content in a variant named Ident |
| Inline Variant | `{ VariantDecls }` | Defines local variants directly in the union, supporting `#[display]` |

### Core Usage
```rust
use diagweave::union;
use std::fmt;

#[derive(Debug)]
struct AuthError;

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "auth error")
    }
}

impl std::error::Error for AuthError {}
impl diagweave::prelude::DiagnosticError for AuthError {}

union! {
    pub enum AppError = 
        AuthError |                     // Uses AuthError as variant name automatically
        std::io::Error as Io |          // Explicitly named as Io
        {                               // Inline definition
            #[display("fatal system failure")]
            Fatal
        }
}
```

### Feature Descriptions
- **Auto `Display`**: For external types, generates `match` branches calling `inner.fmt(f)`; for inline variants, generates rendering logic based on `#[display]`.
- **Auto `Error`**: If `Debug` is not provided, `#[derive(Debug)]` is automatically attached.
- **From Injection**: Injects `impl From<T> for Union` for every external member type.
- **Helpers & Traits**: Automatically implements `DiagnosticError` to provide `to_report()`, `to_report_trans::<NewE>()`, and direct diagnostic builder methods, and generates `source()` on the union enum.

---

## 3. `#[derive(Error)]` Derive Macro

### Overview
Provides convenient implementations of `Display` and `std::error::Error` traits for existing independent `struct` or `enum` types, bridging them into the `diagweave` diagnostic system.

### Supported Attributes
| Attribute | Scope | Parameters | Description |
| :--- | :--- | :--- | :--- |
| `#[display]` | Variant/Struct | `"template"` / `transparent` | Same rendering logic as in `set!` |
| `#[from]` | Field | None | Auto-implements `From<FieldType>`, constructing Self containing this field |
| `#[source]` | Field | None | Marks the field as the return value for `Error::source()` |

### Generated Member Methods & Trait Implementations
Any type deriving `Error` automatically gains the following helper methods and trait implementations:
| Declaration | Return Type/Trait | Description |
| :--- | :--- | :--- |
| `impl DiagnosticError` | `DiagnosticError` | Implements `DiagnosticError` trait, which provides `to_report()`, `to_report_trans::<NewE>()`, and direct diagnostic builder methods, and marks this client error type for automatic conversion to any compatible `Report<NewE>` via the `From` trait |
| `pub fn source(&self)` | `Option<&dyn Error>` | Convenient access to the underlying error source |

Furthermore, macro-generated/derived types (`#[derive(Error)]`, `set!`, and `union!`) automatically implement the marker trait `DiagnosticError`. If the target error type satisfies `NewE: From<E>`, the raw error `E` can be directly converted into a diagnostic report:
- `let report: Report<NewE> = raw_err.into();`
- Or in a function returning `Result<_, Report<NewE>>`, use `?` on `Result<_, E>` for automatic error conversion and propagation.

### Usage Example
```rust
#[derive(diagweave::Error, Debug)]
#[display("system failure")] // Struct-level display template
struct GlobalError {
    #[source] // Manually specify source
    inner: std::io::Error,
    
    msg: String,
}

#[derive(diagweave::Error, Debug)]
enum FileError {
    #[display("read error: {0}")]
    Read(#[from] std::io::Error), // Auto From impl and source
}
```

---

## 4. `Result` Extension Traits (`DiagnosticResult` / `ResultReportExt`)

### Overview
Provides pipelines for seamless diagnostic info injection on error paths by implementing extension traits for `Result<T, E>` and `Result<T, Report<E>>`.

### Core Traits
#### 1. `DiagnosticResult` (on `Result<T, E>`)
- `to_report_res()`: Lifts `Err(E)` to `Err(Report<E>)` without error type conversion.
- `to_report_res_trans::<TargetE>()`: Lifts `Err(E)` to `Err(Report<TargetE>)`, supporting automatic conversion of the inner error (requires `E: Into<TargetE>`).
- `to_report_note(msg)`: Lifts and injects note.
- **Direct Chained Diagnostic Methods**: `Result<T, E> where E: DiagnosticError` now automatically supports all `Report` builder methods (such as `.with_ctx()`, `.attach_note()`, `.with_severity()`, etc.). These methods automatically wrap the error into a `Report` and apply the diagnostics on the `Err` path, passing through on the `Ok` path.

#### 2. `ResultReportExt` (on `Result<T, Report<E>>`)
Instead of duplicating every `Report` method, this trait provides a single combinator and read-only helpers:
- `map_report(|r| ...)` — apply any chain of `Report` builder methods on the error path
- `with_ctx()`, `attach_note()`, `set_severity()`, etc. — all `Report` builder methods are available directly on `Result<T, Report<E>>` and only execute on the `Err` path
- `map_inner_err(|e| Outer::from(e))` — transform the inner error type while preserving all diagnostics
- `into_inner_err()` — discard diagnostics and return `Result<T, E>`
- **Read-Only Helpers**: For error-path inspection without manually matching `Err`:
  - `report_ref()`, `report_inner()`, `report_metadata()`, `report_attachments()`
  - `report_error_code()`, `report_severity()`, `report_category()`, `report_retryable()`
  - `report_context()`, `report_system()`, `report_stack_trace()`, `report_options()`, `report_display_causes()`
  - `report_iter_origin_sources()`, `report_iter_diag_sources()`

The closure receives an owned `Report` and must return an owned `Report`. On the `Ok` path the closure is never invoked, providing natural lazy semantics.


### Usage Example
```rust
# use diagweave::prelude::set;
# set! {
#     AuthError = {
#         #[display("user {user_id} token is invalid")]
#         InvalidToken { user_id: u64 },
#     }
#     ApiError = AuthError | {
#         Unknown
#     }
# }
# fn verify(user_id: u64) -> Result<String, AuthError> {
#     Ok("success".to_string())
# }
use diagweave::prelude::*;
use std::{fs, io};
use std::time::SystemTime;

fn process() -> Result<(), Report<io::Error, HasSeverity>> {
    let file_key = "file";
    let timestamp_key = "timestamp";
    fs::read_to_string("config.toml")
        .to_report_res()
        .with_ctx(file_key, "config.toml")
        .with_severity(Severity::Warn)
        .with_ctx(timestamp_key, ContextValue::String(format!("{:?}", SystemTime::now()).into()))
        .attach_printable("failed to load system config")
        .set_severity(Severity::Error)?;
        
    Ok(())
}

// Example: Mapping error types while preserving diagnostics
fn boundary_op() -> Result<String, Report<io::Error>> {
    fs::read_to_string("config.toml")
        .attach_note("captured at boundary")
        .map_inner_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

// Example: Using the generic From/Into implicit conversion for simplified error promotion
fn boundary_op_simplified() -> Result<String, Report<ApiError>> {
    // Assuming verify returns Result<String, AuthError>. Since ApiError implements From<AuthError>
    // and AuthError implements DiagnosticError, we can simply use `?` for automatic conversion and promotion to Report<ApiError>:
    let res = verify(7)?;
    Ok(res)
}
```

## 5. Universal Conversion Trait (`Transform`)

### Overview
Provides a unified `.trans()` method to perform flexible, cross-layer, and cross-type conversions between raw error types, report types, and result types. It is particularly useful at architectural boundaries (e.g., from the Service layer to the API layer) where you need to transition from low-level errors into a `Report` or a `Result<T, Report<TargetE>>` with a different target error type.

### Supported Conversion Patterns
If `E1` implements `DiagnosticError` and can be converted into `E2` (i.e., `E1: Into<E2>`):
1. **`E1` -> `Report<E2>`**: Converts the raw error directly into a `Report` of the target type.
2. **`E1` -> `Result<T, Report<E2>>`**: Converts the raw error into a `Result::Err` containing a `Report` of the target type.
3. **`Report<E1, State>` -> `Report<E2, State>`**: Converts an existing `Report<E1>` into a `Report<E2>` of the target type, preserving all diagnostic context, attachments, and severity state.
4. **`Report<E1, State>` -> `Result<T, Report<E2, State>>`**: Converts an existing `Report<E1>` into a `Result::Err` containing the mapped `Report<E2>`, while preserving all diagnostic context and attachments.
5. **`Result<T, Report<E1, State>>` -> `Result<T, Report<E2, State>>`**: Converts a `Result` containing `Report<E1>` into a `Result` containing the mapped `Report<E2>`, while preserving all diagnostic context and state.
6. **`Result<T, E1>` -> `Result<T, Report<E2>>`**: Converts a `Result` containing the raw error `E1` into a `Result` containing the target `Report<E2>`.

### Usage Example
```rust
use diagweave::prelude::*;

#[derive(diagweave::Error, Debug)]
#[display("database error")]
struct DbError;

#[derive(diagweave::Error, Debug)]
#[display("api error")]
enum ApiError {
    #[display("internal service failure")]
    Internal(#[from] DbError),
}

fn query_database() -> Result<(), DbError> {
    Err(DbError)
}

// Example 1: E1 -> Result<T, Report<E2>>
fn handle_request() -> Result<(), Report<ApiError>> {
    // query_database() returns Result<(), DbError>
    // Use .trans() to convert the inner DbError into Report<ApiError> in a single step
    query_database().map_err(|e| e.trans())
}

// Example 2: Report<E1> -> Result<T, Report<E2>>
fn handle_request_with_ctx() -> Result<(), Report<ApiError>> {
    let report_e1: Report<DbError> = query_database()
        .with_ctx("db", "users")
        .expect_err("captured");
        
    // Convert Report<DbError> into Result<_, Report<ApiError>>, retaining "db"="users" context
    let res: Result<(), Report<ApiError>> = report_e1.trans();
    res?;
    Ok(())
}
```

---

## 6. Display Cause Collection

### Overview
Manages the chain of triggers for a diagnostic. `diagweave` supports not only `std::error::Error` chains but also cross-thread/cross-process event messages.

### Display Cause Data
| Type Name | Description |
| :--- | :--- |
| `DisplayCauseChain` | Runtime chain summary with `items: Vec<Arc<dyn Display + Send + Sync + 'static>>`, plus `truncated` and `cycle_detected`. |

### Core Data Conversion: `AttachmentValue`
Strongly typed values supported by `Report` attachments, converted automatically from base types:
| Type | Rust Implementation Type | Description |
| :--- | :--- | :--- |
| `String` | `&str`, `String` | UTF-8 Text |
| `Integer` | `i8..i64` | Signed Integer |
| `Unsigned` | `u8..u64` | Unsigned Integer |
| `Float` | `f32`, `f64` | Floating Point |
| `Bool` | `bool` | Boolean |
| `Array` | `Vec<AttachmentValue>` | List/Sequence |
| `Object` | `BTreeMap<String, AttachmentValue>` | Key-Value mapping |
| `Bytes` | `Vec<u8>` | Binary data content |
| `Redacted` | `{ kind, reason }` | Placeholder for sensitive data |

Attachment note access:
- `Attachment::as_note() -> Option<String>` returns a materialized note string.
- `Attachment::as_note_display() -> Option<&(dyn Display + Send + Sync + 'static)>` returns a zero-allocation display reference.

---

## 7. Log System Integration (`Tracing`)

### Overview
Exports diagnostic reports to monitoring systems or log streams.
- **`trace` feature**: Provides the data model, `PreparedTracingEmission`, and `TracingExporterTrait` for custom exporters.
- **`tracing` feature**: Provides the default implementation for the `tracing` crate plus `prepare_tracing` / `emit_tracing` helpers.

### Core API
| Method | Description |
| :--- | :--- |
| `prepare_tracing(&self)` | Available on `Report<_, HasSeverity>` / `DiagnosticIr<_, HasSeverity>`; resolves final report/event levels and returns a prepared emission typestate |
| `emit_tracing(&self)` | Convenience wrapper for `prepare_tracing().emit()` |
| `with_trace_ids(tid, sid)` | Manually binds tracing context (Trace ID / Span ID). Parameters are `TraceId` / `SpanId`. |

### Export Behavior
- **Attribute Mapping**: `Context` is mapped as named fields for the `tracing` event.
- **Structured Fields**: `report_display_causes`, `report_origin_source_errors / report_diagnostic_source_errors`, `report_stack_trace`, `report_context`, and `report_attachments` are emitted as structured debug fields.
- **Empty Sections**: Empty `trace`, `context`, and `attachments` sections are omitted.
- **Trace ID Binding**: If Report contains `TraceContext`, it is automatically associated, or associated via injector from current Span environment.

### Usage Example
```rust
use diagweave::prelude::Report;
use std::fmt;

#[cfg(feature = "trace")]
use diagweave::prelude::{Severity, SpanId, TraceId};
#[cfg(feature = "trace")]
use diagweave::trace::{EmitStats, PreparedTracingEmission, TracingExporterTrait};

#[derive(Debug)]
struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error")
    }
}

impl std::error::Error for MyError {}

#[cfg(feature = "trace")]
struct MyCustomExporter;

#[cfg(feature = "trace")]
impl TracingExporterTrait for MyCustomExporter {
    fn export_prepared(&self, emission: PreparedTracingEmission<'_>) -> EmitStats {
        emission.stats()
    }
}

let report = Report::new(MyError);
#[cfg(feature = "trace")]
let report = report.with_severity(Severity::Error);

// Bind trace/span ids
#[cfg(feature = "trace")]
let report = report.with_trace_ids(
    TraceId::from_str("4bf92f3577b34da6a3ce929d0e0e4736").unwrap(),
    SpanId::from_str("00f067aa0ba902b7").unwrap(),
);

// Export to current tracing span with default options
#[cfg(feature = "tracing")]
report.prepare_tracing().emit();

// Use a custom exporter
#[cfg(feature = "trace")]
report
    .prepare_tracing()
    .emit_with(&MyCustomExporter);
```

---

## 8. Advanced Patterns

### 1. Complex Attachments: Structured JSON Correlation
Leverage `serde_json` macro to inject structured data directly.
```rust
use diagweave::prelude::*;
use std::fmt;

#[cfg(feature = "json")]
use serde_json::json;

#[derive(Debug)]
struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error")
    }
}

impl std::error::Error for MyError {}

#[cfg(feature = "json")]
let _report = Report::new(MyError).attach_payload(
    "request_meta",
    AttachmentValue::from(json!({ "version": "v1", "retry": 3 })),
    Some("application/json")
);
```

### 2. Multi-Level Wrapping Across Layers
Preserve the full error source chain when passing through architectural layers.
```rust
use diagweave::prelude::*;
use std::fmt;

#[derive(Debug)]
struct DatabaseError;

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "db error")
    }
}

impl std::error::Error for DatabaseError {}
impl diagweave::prelude::DiagnosticError for DatabaseError {}

#[derive(Debug)]
enum AppError {
    Db(DatabaseError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(_) => write!(f, "app db error"),
        }
    }
}

impl std::error::Error for AppError {}
impl diagweave::prelude::DiagnosticError for AppError {}

fn db_operation() -> Result<(), DatabaseError> {
    Err(DatabaseError)
}

fn service_layer() -> Result<(), Report<AppError>> {
    db_operation()
        .with_ctx("db", "primary")
        .set_accumulate_src_chain(true)
        .map_inner_err(AppError::Db)?;
    Ok(())
}
```

### 3. Custom Renderer Implementation
Customize output format (e.g., output to HTML or Web UI) by implementing the `ReportRenderer` trait.
```rust
use diagweave::prelude::*;
use std::fmt::{self, Display, Formatter};

struct MyHtmlRenderer;
impl<E, State> ReportRenderer<E, State> for MyHtmlRenderer
where
    E: Display + std::error::Error + 'static,
    State: SeverityState,
{
    fn render(&self, report: &Report<E, State>, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", report.pretty())
    }
}
```
