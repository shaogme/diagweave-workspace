# 错误定义与转换

## 1. `set!` 宏

### 概览
用于定义一系列结构化的错误枚举（Error Set），自动实现集合间的组合逻辑、`From` 转换、报告语义，并实现 `DiagnosticError` trait（提供 `to_report()`/`diag()` 默认方法）以及生成 `source()` 辅助方法。

### 语法定义
```rust, ignore
set! {
    Ident = { [VariantDecls] } [ | OtherSet ]
    ...
}
```


### 支持属性 (Attributes)
| 属性 | 位置 | 参数 | 说明 |
| :--- | :--- | :--- | :--- |
| `#[display("...")]` | 变体 | 格式化字符串 | 使用 `{field}` 或 `{0}` 引用命名字段或匿名元组字段 |
| `#[display(transparent)]` | 变体 | 无 | 直接将内部字段的 `Display` 委托给该变体 (需恰好 1 个字段) |
| `#[from]` | 变体 | 无 | 标记该变体可从其单字段类型直接转换 (需恰好 1 个字段) |

### 核心用法
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

### 生成方法/实现 (以 `AuthError` 为例)
| 声明 | 返回类型 | 说明 |
| :--- | :--- | :--- |
| `DiagnosticError::to_report::<NewE>(self)` | `Report<NewE>` | (来自 `DiagnosticError` trait) 将错误实例转换为报告，支持可选的目标类型转换 (要求 `Self: Into<NewE>`，默认为 `Self`) |
| `DiagnosticError::diag(self, f)` | `Report<E2, State2>` | (来自 `DiagnosticError` trait) 链式诊断信息构造的便捷入口 |
| `AuthError::source(&self)` | `Option<&dyn Error>` | 读取底层 source 错误 |
| `From<AuthError> for ServiceError` | `ServiceError` | 自动实现子集到超集的映射 |

---

## 2. `union!` 宏

### 概览
用于在架构边界组合多个不相关的错误类型、其他错误集合或内联定义的变体。

### 语法定义
```rust, ignore
union! {
    [Attributes]
    [vis] enum Ident = Item1 | Item2 | ...
}
```

### 声明项 (UnionItem)
| 项类型 | 语法 | 说明 |
| :--- | :--- | :--- |
| 外部类型 | `Path` | 自动实现 `From<Path>` 并委托 `Display` |
| 外部类型别名 | `Path as Ident` | 将 Path 的内容包装在名为 Ident 的变体中 |
| 内联变体 | `{ VariantDecls }` | 直接在 union 中定义本地变体，支持 `#[display]` |

### 核心用法
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
        AuthError |                     // 自动使用 AuthError 作为变体名
        std::io::Error as Io |          // 显式起名为 Io
        {                               // 内联定义
            #[display("fatal system failure")]
            Fatal
        }
}
```

### 特性描述
- **自动实现 `Display`**：对于外部类型，生成 `match` 分支调用 `inner.fmt(f)`；对于内联变体，基于 `#[display]` 模板生成渲染逻辑。
- **自动实现 `Error`**：如果未提供 `Debug`，会自动附加 `#[derive(Debug)]`。
- **From 注入**：为每一个外部成员类型注入 `impl From<T> for Union`。
- **辅助方法与 trait**：自动实现 `DiagnosticError` 以获得 `to_report::<NewE>()` 和 `diag()` 默认方法，并自动生成 `source()` 方法。

---

## 3. `#[derive(Error)]` 派生宏

### 概览
为已有的独立 `struct` 或 `enum` 类型提供 `Display` 和 `std::error::Error` trait 的便捷实现，并桥接到 `diagweave` 诊断体系。

### 支持属性 (Attributes)
| 属性 | 位置 | 参数 | 说明 |
| :--- | :--- | :--- | :--- |
| `#[display]` | 变体/结构体 | `"template"` / `transparent` | 同 `set!` 中的渲染逻辑 |
| `#[from]` | 字段 | 无 | 自动实现 `From<FieldType>`，生成的实现会构造包含该字段的 Self |
| `#[source]` | 字段 | 无 | 标记该字段为 `Error::source()` 的返回值 |

### 生成成员方法与 trait 实现
任何派生了 `Error` 的类型会自动获得以下辅助方法与 trait 实现：
| 声明 | 返回类型/trait  | 说明 |
| :--- | :--- | :--- |
| `impl DiagnosticError` | `DiagnosticError` | 实现 `DiagnosticError` trait，从而自动获得 `to_report::<NewE>()` 和 `diag()` 辅助方法，并标记该客户端错误可以通过 `From` trait 自动转换为任何兼容的 `Report<NewE>` |
| `pub fn source(&self)` | `Option<&dyn Error>` | 便捷访问底层 Error 源 |

此外，派生宏、`set!` 宏和 `union!` 宏会自动实现标记 trait  `DiagnosticError`。当目标错误类型满足 `NewE: From<E>` 时，允许直接将原始错误 `E` 转换为诊断报告：
- `let report: Report<NewE> = raw_err.into();`
- 或者在返回 `Result<_, Report<NewE>>` 的函数中，直接使用 `?` 对 `Result<_, E>` 进行自动类型提升与传播。

### 示例用法
```rust
#[derive(diagweave::Error, Debug)]
#[display("system failure")] // Struct 级别的 display 模板
struct GlobalError {
    #[source] // 手动指定 source
    inner: std::io::Error,
    
    msg: String,
}

#[derive(diagweave::Error, Debug)]
enum FileError {
    #[display("read error: {0}")]
    Read(#[from] std::io::Error), // 自动实现 From 并作为 source
}
```

---

## 4. `Result` 扩展 trait  (`DiagnosticResult` / `ResultReportExt`)

### 概览
通过为 `Result<T, E>` 和 `Result<T, Report<E>>` 实现扩展 trait，提供在错误路径上无缝注入诊断信息的管道。

### 核心 trait
#### 1. `DiagnosticResult` (作用于 `Result<T, E>`)
- `to_report_res<TargetE>()`: 提升 `Err(E)` 为 `Err(Report<TargetE>)`，支持自动转换内部错误类型（要求 `E: Into<TargetE>`）。
- `to_report_note(msg)`: 提升并注入备注。
- `diag_res(...)`：Result<T, E> 上的快捷入口，泛型版本允许转换错误类型和状态类型；签名：
  `diag_res<E2, State2>(self, f: impl FnOnce(Report<E>) -> Report<E2, State2>) -> Result<T, Report<E2, State2>>`。
  闭包接收 `Report<E>` 并返回 `Report<E2, State2>`。当仅添加元数据时无需显式类型标注；
  当转换错误类型（如通过 `map_err`）时需要标注返回类型。若需要控制原生 source 链是否继续累积，可先通过 `set_accumulate_src_chain()` 配置报告选项。

#### 2. `ResultReportExt` (作用于 `Result<T, Report<E>>`)
不再重复每个 `Report` 方法，而是提供单一组合子和只读查询：
- `map_report(|r| r.with_ctx(...).with_severity(...))` — 仅在错误路径上应用任意 `Report` 方法链
- `map_inner_err(|e| Outer::from(e))` — 转换内部错误类型，并保留所有诊断信息
- `trans_inner_err()` — 转换内部错误类型的便捷快捷方式（当 `E: Into<NewE>` 时）
- `into_inner_err()` — 丢弃诊断信息，返回 `Result<T, E>`
- **只读查询**：用于在错误路径做只读查询，避免手动 `match Err(report)`：
  - `report_ref()`、`report_inner()`、`report_metadata()`、`report_attachments()`
  - `report_error_code()`、`report_severity()`、`report_category()`、`report_retryable()`
  - `report_context()`、`report_system()`、`report_stack_trace()`、`report_options()`、`report_display_causes()`
  - `report_iter_origin_sources()`、`report_iter_diag_sources()`

闭包接收 owned `Report` 并返回 owned `Report`。在 `Ok` 路径上闭包永远不会被调用，提供天然的延迟语义。


### 用法示例
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

// 示例：在保留诊断信息的同时转换错误类型
fn boundary_op() -> Result<String, Report<io::Error>> {
    fs::read_to_string("config.toml")
        .diag_res(|r| r.attach_note("captured at boundary"))
        .map_inner_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

// 示例：利用通用 From/Into 隐式转换简化错误提升
fn boundary_op_simplified() -> Result<String, Report<ApiError>> {
    // 假设 verify 返回 Result<String, AuthError>，因为 ApiError 实现了 From<AuthError>，
    // 并且 AuthError 实现了 DiagnosticError，我们可以极其简单地用 `?` 自动转换并提升为 Report<ApiError>：
    let res = verify(7)?;
    Ok(res)
}
```

## 5. 通用转换 Trait (`Transform`)

### 概览
提供统一的 `.trans()` 方法，用于在错误类型、报告类型以及结果类型之间进行灵活的跨层级、跨类型转换。它特别适用于在架构边界（例如 Service 层到 API 层）需要将底层错误转换为带有不同错误类型的 `Report` 或 `Result<T, Report<TargetE>>` 的场景。

### 支持的转换模式
若 `E1` 实现了 `DiagnosticError`，且能够转换为 `E2`（即满足 `E1: Into<E2>`）：
1. **`E1` -> `Report<E2>`**：将原始错误直接转换为目标类型的 `Report`。
2. **`E1` -> `Result<T, Report<E2>>`**：将原始错误转换为包含目标类型 `Report` 的 `Result::Err`。
3. **`Report<E1, State>` -> `Result<T, Report<E2, State>>`**：将已有的 `Report<E1>` 转换为包含映射后 `Report<E2>` 的 `Result::Err`，同时保留所有诊断上下文和附件。

### 示例用法
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

// 示例 1: E1 -> Result<T, Report<E2>>
fn handle_request() -> Result<(), Report<ApiError>> {
    // query_database() 返回 Result<(), DbError>
    // 使用 .trans() 可以一步将其内部的 DbError 转换为 Report<ApiError>
    query_database().map_err(|e| e.trans())
}

// 示例 2: Report<E1> -> Result<T, Report<E2>>
fn handle_request_with_ctx() -> Result<(), Report<ApiError>> {
    let report_e1: Report<DbError> = query_database()
        .diag_res(|r| r.with_ctx("db", "users"))
        .expect_err("captured");
        
    // 将 Report<DbError> 转换为 Result<_, Report<ApiError>>，并保留 "db"="users" 上下文
    let res: Result<(), Report<ApiError>> = report_e1.trans();
    res?;
    Ok(())
}
```

---

## 6. 展示原因收集

### 概览
负责管理诊断发生的诱因链。`diagweave` 的优势在于它不仅支持 `std::error::Error` 链，还支持跨线程/跨进程的事件消息。

### 展示原因数据
| 类型名 | 说明 |
| :--- | :--- |
| `DisplayCauseChain` | 运行时展示原因链摘要，包含 `items: Vec<Arc<dyn Display + Send + Sync + 'static>>`、`truncated` 与 `cycle_detected`。 |

### 核心数据转换：`AttachmentValue`
`Report` 附件支持的强类型值，支持自动从基础类型转换：
| 类型 | Rust 实现类型 | 说明 |
| :--- | :--- | :--- |
| `String` | `&str`, `String` | UTF-8 文本 |
| `Integer` | `i8..i64` | 有符号整数 |
| `Unsigned` | `u8..u64` | 无符号整数 |
| `Float` | `f32`, `f64` | 浮点数 |
| `Bool` | `bool` | 布尔值 |
| `Array` | `Vec<AttachmentValue>` | 列表/序列 |
| `Object` | `BTreeMap<String, AttachmentValue>`| 键值对映射 |
| `Bytes` | `Vec<u8>` | 二进制数据内容 |
| `Redacted` | `{kind, reason}` | 脱敏 data 占位符 |

Note 附件读取：
- `Attachment::as_note() -> Option<String>`：返回物化后的 note 文本。
- `Attachment::as_note_display() -> Option<&(dyn Display + Send + Sync + 'static)>`：返回零分配的显示引用。

---

## 7. 日志系统集成 (`Tracing`)

### 概览
将诊断报告导出到监控系统或日志流。
- **`trace` 特性**：提供数据模型、`PreparedTracingEmission` 以及供自定义导出器使用的 `TracingExporterTrait`。
- **`tracing` 特性**：提供针对 `tracing` crate 的默认实现，以及 `prepare_tracing` / `emit_tracing` 辅助方法。

### 核心 API
| 方法 | 说明 |
| :--- | :--- |
| `prepare_tracing(&self)` | 仅在 `Report<_, HasSeverity>` / `DiagnosticIr<_, HasSeverity>` 上可用；会解析最终 report/event level 并返回可直接发射的 typestate 对象 |
| `emit_tracing(&self)` | `prepare_tracing().emit()` 的便捷封装 |
| `with_trace_ids(tid, sid)` | 手动绑定追踪上下文 (Trace ID / Span ID)，参数为 `TraceId` / `SpanId` |

### 导出行为
- **属性映射**：`Context` 会被映射为 `tracing` 事件的命名字段。
- **结构化字段**：`report_display_causes`、`report_origin_source_errors / report_diagnostic_source_errors`、`report_stack_trace`、`report_context` 和 `report_attachments` 会作为结构化调试字段导出。
- **空部分**：空的 `trace`、`context`、`attachments` 部分默认会省略。
- **Trace ID 绑定**：若 Report 包含 `TraceContext`，导出时会自动关联，或通过注入器自动关联当前 Span环境信息。

### 用法示例
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

// 绑定 trace/span ids
#[cfg(feature = "trace")]
let report = report.with_trace_ids(
    TraceId::from_str("4bf92f3577b34da6a3ce929d0e0e4736").unwrap(),
    SpanId::from_str("00f067aa0ba902b7").unwrap(),
);

// 使用默认选项导出到当前 tracing span
#[cfg(feature = "tracing")]
report.prepare_tracing().emit();

// 使用自定义导出器
#[cfg(feature = "trace")]
report
    .prepare_tracing()
    .emit_with(&MyCustomExporter);
```

---

## 8. 高阶模式 (Advanced Patterns)

### 1. 复杂附件：结构化 JSON 关联
利用 `serde_json` 宏直接注入结构化数据。
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

### 2. 多层包装与错误链透传 (Wrap)
在架构各层之间传递时保留完整的 `source` 错误链。`map_err()` 默认会在 debug 构建中累积原生 source 链，在 release 构建中默认关闭；可以通过 `set_accumulate_src_chain(true/false)` 显式控制。
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
        .diag_res(|r| {
            r.with_ctx("db", "primary")
                .set_accumulate_src_chain(true)
                .map_err(AppError::Db)
        })?;
    Ok(())
}
```

### 3. 自定义渲染器实现
通过实现 `ReportRenderer` trait 来自定义输出格式（如输出 to HTML 或 Web UI）。
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
        write!(f, "<div>{}</div>", report.pretty())
    }
}
```
