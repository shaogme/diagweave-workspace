# Error Definition and Conversion

## 1. `set!` Macro

### Overview
Used to define a series of structured error enums (Error Sets). It automatically implements composition logic between sets, `From` conversions, snake_case named constructors, report semantics, and enum helpers (`to_report()`/`source()`/`diag()`).

### Syntax Definition
```rust, ignore
set! {
    [#[diagweave(Meta)]]
    Ident = { [VariantDecls] } [ | OtherSet ]
    ...
}
```

### Declaration Parameters (Meta)
| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `report_path` | `String` | `"::diagweave::report::Report"` | Path to the `Report` type returned by `*_report` constructors |
| `constructor_prefix` | `String` | `""` | Prefix for generated constructor function names (e.g., `new_`) |

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
| `AuthError::user_not_found(id: u64)` | `AuthError` | Snake_case constructor |
| `AuthError::user_not_found_report(id: u64)` | `Report<AuthError>` | Returns a report object containing the current error |
| `AuthError::to_report(self)` | `Report<AuthError>` | Converts error instance into a report |
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
- **Constructors**: Generates snake_case constructors and `*_report` helpers for inline and external variants.
- **Options**: Supports `#[diagweave(constructor_prefix = "...", report_path = "...")]` on the union enum.
- **Helpers**: Generates `to_report()`, `source()`, and `diag()` on the union enum.

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

### Generated Member Methods
Any type deriving `Error` automatically gains the following helper methods:
| Method Declaration | Return Type | Description |
| :--- | :--- | :--- |
| `pub fn to_report(self)` | `Report<Self>` | Converts to a basic report object |
| `pub fn source(&self)` | `Option<&dyn Error>` | Convenient access to the underlying error source |

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

## 4. `Result` Extension Traits (`Diagnostic` / `ResultReportExt` / `InspectReportExt`)

### Overview
Provides pipelines for seamless diagnostic info injection on error paths by implementing extension traits for `Result<T, E>` and `Result<T, Report<E>>`.

### Core Traits
#### 1. `Diagnostic` (on `Result<T, E>`)
- `to_report()`: Lifts `Err(E)` to `Err(Report<E>)`.
- `to_report_note(msg)`: Lifts and injects note.
- `diag(...)`: Short-hand for chaining a transformation on the error path. Generic signature:
  `diag<E2, State2>(self, f: impl FnOnce(Report<E>) -> Report<E2, State2>) -> Result<T, Report<E2, State2>>`.
  The closure receives a `Report<E>` and returns a `Report<E2, State2>`. When only adding metadata,
  no explicit type annotations are needed; when transforming the error type (e.g., via `map_err`),
  the return type must be annotated. If you need to control whether the origin source chain continues
  to accumulate, configure the report options first via `set_accumulate_src_chain()`.

#### 2. `ResultReportExt` (on `Result<T, Report<E>>`)
Instead of duplicating every `Report` method, this trait provides a single combinator:
- `and_then_report(|r| ...)` — apply any chain of `Report` builder methods on the error path
- `with_ctx()`, `attach_note()`, `set_severity()`, etc. — all `Report` builder methods are available directly on `Result<T, Report<E>>` and only execute on the `Err` path
- `map_report_err(|e| Outer::from(e))` — transform the inner error type while preserving all diagnostics
- `into_report_inner()` — discard diagnostics and return `Result<T, E>`

The closure receives an owned `Report` and must return an owned `Report`. On the `Ok` path the closure is never invoked, providing natural lazy semantics.

#### 3. `InspectReportExt` (on `Result<T, Report<E>>`)
Read-only helpers for error-path inspection without manually matching `Err`:
- `report_ref()`, `report_inner()`, `report_metadata()`, `report_attachments()`
- `report_error_code()`, `report_severity()`, `report_category()`, `report_retryable()`
- `report_context()`, `report_system()`, `report_stack_trace()`, `report_options()`, `report_display_causes()`
- `report_iter_origin_sources()`, `report_iter_diag_sources()`


### Usage Example
```rust
use diagweave::prelude::*;
use std::{fs, io};
use std::time::SystemTime;

fn process() -> Result<(), Report<io::Error, HasSeverity>> {
    let file_key = "file";
    let timestamp_key = "timestamp";
    fs::read_to_string("config.toml")
        .to_report()
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
        .to_report()
        .map_report_err(|e| io::Error::new(io::ErrorKind::Other, e))
        .and_then_report(|r| r.attach_note("captured at boundary"))
}
```

---

## 5. Display Cause Collection

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

## 6. Log System Integration (`Tracing`)

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

## 7. Advanced Patterns

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

fn db_operation() -> Result<(), DatabaseError> {
    Err(DatabaseError)
}

fn service_layer() -> Result<(), Report<AppError>> {
    db_operation()
        .to_report()
        .and_then_report(|r| {
            r.with_ctx("db", "primary")
                .set_accumulate_src_chain(true)
                .map_err(AppError::Db)
        })?;
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
