# 诊断报告容器

## 1. `Report<E>` 诊断报告

### 概览
核心诊断容器，封装原始错误 `E`并持有可选的“冷数据”（元数据、附件、展示原因链、追踪信息）以及按报告粒度生效的 `ReportOptions`。采用延迟分配策略，仅在添加辅助信息时才分配堆内存。
`category`、`trace_state`、trace 事件名和 stack trace 原始文本等高频字符串在捕获后会以共享 `StaticRefStr` 持有。
`map_err()` 是当前推荐的错误类型转换入口；其是否继续累积原生 `source` 链由 `ReportOptions` 控制。

### 声明定义

`Report` 结构体是一个高级诊断容器，对辅助数据采用延迟分配策略。元数据和诊断信息封装在一个装箱的 `ReportData` 结构中，以保持 `Report` 结构体本身足够小。所有字段都是**私有的**，无法从模块外部直接访问。

```rust, ignore
pub struct Report<E, State: SeverityState = MissingSeverity> {
    inner: E, // 私有 - 被包装的错误值
    data: Box<ReportData<State>>, // 私有 - 装箱的辅助数据
}

struct ReportData<State: SeverityState> {
    metadata: ReportMetadata<State>, // 私有 - 包含严重性的元数据
    options: ReportOptions, // 私有 - 按报告粒度的配置（内部采用延迟分配）
    #[cfg(feature = "trace")]
    trace: ReportTrace, // 私有 - 追踪上下文和事件（内部采用延迟分配）
    bag: DiagnosticBag, // 私有 - 延迟分配的诊断包
}
```

**关键点：**
- `inner`：被包装的错误值（私有）
- `data`：装箱的所有辅助 diagnostic 数据容器（私有）。这使得 `Report` 结构体非常轻量（仅包含两个指针）。
- `metadata`：（位于 `ReportData` 内部）包含严重性类型状态和可选的 error_code/category/retryable（私有）
- `options`：（位于 `ReportData` 内部）按报告粒度的配置，用于控制源链累积和原因收集行为（私有）。内部采用延迟分配策略。
- `trace`：（位于 `ReportData` 内部）追踪上下文和事件（私有，仅 `trace` feature 下可用）。内部使用 `Option<Box<ReportTraceInner>>` 实现延迟分配
- `bag`：（位于 `ReportData` 内部）延迟分配的诊断包，用于附件、展示原因、源错误、上下文和系统上下文（私有）。内部使用 `Option<Box<DiagnosticBagInner>>` 实现延迟分配
- 字段访问通过方法提供，如 `inner()`、`severity()`、`options()` 等

### 核心构造与转换
| 方法声明 | 说明 |
| :--- | :--- |
| `Report::new(err: E)` | 创建报告 |
| `report.inner()` | 获取内部错误引用 |
| `report.into_inner()` | 消费报告并返回原始错误 |
| `report.attachments()` | 返回关联的所有附件列表 (`&[Attachment]`) |
| `report.metadata()` | 返回原始元数据引用 (`&ReportMetadata`) |
| `report.error_code()` | 读取元数据错误码 (`Option<&ErrorCode>`) |
| `report.severity()` | 从类型状态读取严重级别 (`Option<Severity>`) |
| `report.category()` | 读取元数据分类 (`Option<&str>`) |
| `report.retryable()` | 读取元数据重试标记 (`Option<bool>`) |
| `report.stack_trace()` | 获取关联的堆栈信息 (`Option<&StackTrace>`) |
| `report.trace()` | 获取关联的追踪信息 (`&ReportTrace`)。始终返回引用，使用 `trace.is_empty()` 检查是否有追踪数据 |
| `report.visit_causes(visit)` | 使用默认选项流式遍历展示原因 |
| `report.visit_causes_ext(options, visit)` | 使用自定义选项流式遍历展示原因 |
| `report.visit_origin_sources(visit)` | 使用默认选项流式遍历原生传播链 |
| `report.visit_origin_src_ext(options, visit)` | 使用自定义选项流式遍历原生传播链 |
| `report.visit_diag_sources(visit)` | 使用默认选项流式遍历诊断补充链 |
| `report.visit_diag_srcs_ext(options, visit)` | 使用自定义选项流式遍历诊断补充链 |
| `report.iter_origin_sources()` | 使用默认选项迭代原生传播链 |
| `report.iter_origin_src_ext(options)` | 使用自定义选项迭代原生传播链 |
| `report.iter_diag_sources()` | 使用默认选项迭代诊断补充链 |
| `report.iter_diag_srcs_ext(options)` | 使用自定义选项迭代诊断补充链 |
| `report.options()` | 读取当前 `ReportOptions` 配置 |
| `report.set_options(options: ReportOptions)` | 替换当前报告的选项配置 |
| `report.set_accumulate_src_chain(accumulate: bool)` | 快速设置 `map_err()` 的原生 `source` 链累积行为 |
| `report.map_err(map: FnOnce(E) -> Outer)` | 映射内部错误类型并保留诊断信息；当启用 source 链累积时，会把旧的内层错误接到新错误的 `source` 链上 |

`ReportMetadata<State>` 现在包含严重性类型状态和可选元数据字段。严重性作为直接字段存储（不包装在 Option 或 Box 中），而 error_code/category/retryable 存储在内部的 `Option<Box<MetadataInner>>` 中以实现延迟分配。读取请使用 `error_code()`、`category()`、`retryable()`、`severity()` 等接口。写入式组合请使用 `with_error_code(...)`、`set_severity(...)` 等 builder 方法。

### `ReportOptions` 和 `GlobalConfig`

`ReportOptions` 用于控制单个 `Report` 的错误源链累积行为和原因收集行为。内部采用延迟分配策略（`Option<Box<ReportOptionsInner>>`），仅在显式设置选项时才分配堆内存。配置值通过以下优先级解析：

**配置优先级**：ReportOptions > GlobalConfig > Profile defaults

#### 配置文件相关的默认值

| 选项 | Debug 构建 | Release 构建 |
|------|------------|--------------|
| `accumulate_src_chain` | `true` | `false` |
| `detect_cycle` | `true` | `false` |
| `max_depth` | `16` | `16` |

#### 配置选项说明

- `accumulate_src_chain`：设置后，`map_err()` 会保留并延伸原生 `source` 链；未设置时从 `GlobalConfig` 或 profile 默认值继承。
- `max_depth`：原因收集时的最大深度限制。较高的值提供更完整的错误上下文，但对于非常深的错误链可能影响性能。未设置时从 `GlobalConfig` 或默认值 16 继承。
- `detect_cycle`：设置后，错误链遍历将跟踪已访问 of 错误并在检测到循环时标记。未设置时从 `GlobalConfig` 或 profile 默认值继承。

#### 构建方法

| 方法 | 说明 |
|------|------|
| `ReportOptions::new()` | 创建延迟分配的选项（在设置任何选项前不会分配堆内存） |
| `.with_accumulate_src_chain(bool)` | 设置源链累积行为（如需要会分配内部存储） |
| `.with_max_depth(usize)` | 设置原因收集深度限制 |
| `.with_cycle_detection(bool)` | 启用/禁用循环检测 |

#### 解析方法

| 方法 | 说明 |
|------|------|
| `.resolve_accumulate_src_chain()` | 解析实际使用的累积设置（优先级：ReportOptions > GlobalConfig > Profile default） |
| `.resolve_max_depth()` | 解析实际使用的深度限制 |
| `.resolve_detect_cycle()` | 解析实际使用的循环检测设置 |
| `.resolve_*_with_source()` | 解析值并返回来源追踪（返回 `ResolvedValue<T>`） |

#### 访问方法

| 方法 | 说明 |
|------|------|
| `.is_set()` | 返回 `true` 表示至少有一个选项被显式配置 |
| `.accumulate_src_chain()` | 返回 `Option<bool>` - 显式设置的值，若继承则返回 `None` |
| `.max_depth()` | 返回 `Option<usize>` - 显式设置的值，若继承则返回 `None` |
| `.detect_cycle()` | 返回 `Option<bool>` - 显式设置的值，若继承则返回 `None` |

#### `GlobalConfig` 全局配置

`GlobalConfig` 提供应用级别的配置默认值。当 `ReportOptions` 的字段未设置时，会从 `GlobalConfig` 继承值。所有字段都是私有的，通过方法访问。

| 方法 | 说明 |
|------|------|
| `GlobalConfig::new()` | 创建具有 profile 相关默认值的全局配置 |
| `.with_accumulate_src_chain(bool)` | 设置默认累积行为 |
| `.with_max_depth(usize)` | 设置默认深度限制 |
| `.with_cycle_detection(bool)` | 设置默认循环检测 |
| `.accumulate_src_chain()` | 返回配置的累积默认值 |
| `.max_depth()` | 返回配置的深度默认值 |
| `.detect_cycle()` | 返回配置的循环检测默认值 |
| `set_global_config(config)` | 设置全局配置（仅可调用一次） |

#### 使用示例

```rust
# #[cfg(feature = "std")]
# {
use diagweave::prelude::*;
use diagweave::report::{GlobalConfig, ReportOptions, set_global_config};

// 设置全局默认值（应用启动时调用一次）
let config = GlobalConfig::new()
    .with_accumulate_src_chain(true)
    .with_max_depth(32)
    .with_cycle_detection(true);
set_global_config(config).expect("全局配置已设置");

// 使用配置文件相关的默认值
let error = std::io::Error::new(std::io::ErrorKind::Other, "测试错误");
let report = Report::new(error);

// 为性能关键路径配置
let error2 = std::io::Error::new(std::io::ErrorKind::Other, "测试错误");
let report2 = Report::new(error2).set_options(
    ReportOptions::new()
        .with_max_depth(8)
        .with_cycle_detection(false)
);

// 启用完整诊断用于调试
let error3 = std::io::Error::new(std::io::ErrorKind::Other, "测试错误");
let report3 = Report::new(error3).set_options(
    ReportOptions::new()
        .with_accumulate_src_chain(true)
        .with_max_depth(32)
        .with_cycle_detection(true)
);
# }
```

### `ErrorCode` 设计与转换规则
- 内部模型：
  - `ErrorCode::Integer(i64)`：紧凑数值错误码
  - `ErrorCode::String(StaticRefStr)`：符号型错误码或超范围数值错误码
- 输入转换（`impl Into<ErrorCode>`）：
  - 整型输入（`i8..i128`、`u8..u128`、`isize`、`usize`）先尝试 `TryInto<i64>`
  - 成功则存为 `Integer`
  - 溢出则存为 `String(v.to_string())`
- 输出转换：
  - 支持 `TryFrom<ErrorCode>` / `TryFrom<&ErrorCode>` 到整型（`i8..i128`、`u8..u128`、`isize`、`usize`）
  - 支持 `From<ErrorCode> for String` 与 `From<&ErrorCode> for String`
  - 支持 `Display` / `to_string()` 输出标准文本形态
- 整型提取错误：
  - `ErrorCodeIntError::InvalidIntegerString`
  - `ErrorCodeIntError::OutOfRange`

`AttachmentValue::String` 也使用 `StaticRefStr` 作为内部存储，重复包装同一份 report 时可以减少字符串拷贝。附件 key、payload 名称/media type、全局上下文 key，以及 trace/category 元数据等持久化字符串也遵循同样规则。

### 原因链语义

- `with_display_cause` / `with_display_causes` 接收 `impl Display + Send + Sync + 'static`，并追加到展示原因字符串链（用于渲染与 IR）。
- `with_diag_src_err` 用于显式追加错误对象到**诊断补充链**，参数要求 `impl Error + Send + Sync + 'static`。
- 原生传播链由 `map_err()` 与 `Error::source()` 维护；是否把旧内层错误继续串接到新错误的 `source` 链，由 `ReportOptions` 决定。

### 全局注入 (Global Injection)
用于跨层级自动注入上下文（如 RequestID、SessionID）。
- **注册器**: `register_global_injector(f: fn() -> Option<GlobalContext>)`
- **全局配置**: `set_global_config(config: GlobalConfig)` - 设置应用级别的默认选项
- **注入时机**: 每次创建一个新的 `Report` 实例时自动执行。

| GlobalContext 字段 | 说明 |
| :--- | :--- |
| `context` | `ContextMap` 全局注入的业务上下文 |
| `system` | `ContextMap` 全局注入的系统/运行时上下文 |
| `error` | `Option<GlobalErrorMeta>` 元数据覆盖（`error_code` / `category` / `retryable` / `severity`） |
| `trace`（`trace` feature） | `Option<TraceContext>` 全局注入的 trace 上下文 |

**注意**: `GlobalConfig` 和 `set_global_config` 是独立的全局配置系统，用于设置 `ReportOptions` 的默认值；而 `register_global_injector` 用于注入上下文信息。两者可以配合使用。

`TraceId` / `SpanId` / `ParentSpanId` 为十六进制校验后的标识符。构造方式：
- `TraceId::from_str("32位hex")` / `SpanId::from_str("16位hex")` / `ParentSpanId::from_str("16位hex")`
- `unsafe { TraceId::new_unchecked(...) }` 跳过校验

### 链式配置方法

**API 命名约定**：
- `set_*` 方法总是替换已有值
- `with_*` 方法仅在未设置时才设置值（条件/保留语义）

| 方法 | 参数类型 | 说明 |
| :--- | :--- | :--- |
| `with_ctx` | `(impl Into<StaticRefStr>, impl Into<ContextValue>)` | 添加业务上下文键值对 |
| `set_ctx` | `(ContextMap)` | 替换业务上下文映射 |
| `with_system` | `(impl Into<StaticRefStr>, impl Into<ContextValue>)` | 添加系统上下文键值对 |
| `set_system` | `(ContextMap)` | 替换系统上下文映射 |
| `set_options` | `ReportOptions` | 替换当前报告的选项配置 |
| `set_accumulate_src_chain` | `bool` | 快速设置 `map_err()` 是否累积原生 `source` 链 |
| `attach_note` / `attach_printable` | `impl Display + Send + Sync + 'static` | 添加备注或解决建议 |
| `attach_payload` / `attach_payload` | `(impl Into<StaticRefStr>, Value, Option<impl Into<StaticRefStr>>)` | 附加命名负载 (支持媒体类型) |
| `set_severity` | `Severity` | 设置严重程度 (Debug, Info, Warn, Error, Fatal)，覆盖已有值 |
| `with_severity` | `Severity` | 设置严重程度，仅当未设置时生效 (保留底层诊断信息) |
| `set_error_code` | `impl Into<ErrorCode>` | 设置稳定的错误代码 (如 "E001")，覆盖已有值 |
| `with_error_code` | `impl Into<ErrorCode>` | 设置错误代码，仅当未设置时生效 (保留底层诊断信息) |
| `set_category` | `impl Into<StaticRefStr>` | 设置错误分类 (用于监控指标)，覆盖已有值 |
| `with_category` | `impl Into<StaticRefStr>` | 设置错误分类，仅当未设置时生效 (保留底层诊断信息) |
| `set_retryable` | `bool` | 标记该错误是否建议重试，覆盖已有值 |
| `with_retryable` | `bool` | 标记是否建议重试，仅当未设置时生效 (保留底层诊断信息) |
| `with_display_cause` | `impl Display + Send + Sync + 'static` | 添加单个展示原因字符串 |
| `with_display_causes` | `impl IntoIterator<Item = impl Display + Send + Sync + 'static>` | 批量添加展示原因字符串 |
| `with_diag_src_err` | `impl Error + Send + Sync + 'static` | 添加单个显式错误源对象 |
| `set_stack_trace` | `StackTrace` | 手动关联已存在的堆栈信息，覆盖已有值 |
| `with_stack_trace` | `StackTrace` | 手动关联已存在的堆栈信息，仅当未设置时生效 |
| `set_trace` | `ReportTrace` | 设置追踪信息，覆盖已有值 |
| `with_trace` | `ReportTrace` | 设置追踪信息，仅当未设置时生效 |
| `set_trace_ids` | `(TraceId, SpanId)` | 设置追踪和 Span ID，覆盖已有值 |
| `with_trace_ids` | `(TraceId, SpanId)` | 设置追踪和 Span ID，仅当未设置时生效 |
| `set_parent_span_id` | `ParentSpanId` | 设置父 Span ID，覆盖已有值 |
| `with_parent_span_id` | `ParentSpanId` | 设置父 Span ID，仅当未设置时生效 |
| `set_trace_sampled` | `bool` | 设置是否采样，覆盖已有值 |
| `with_trace_sampled` | `bool` | 设置是否采样，仅当未设置时生效 |
| `set_trace_state` | `impl Into<StaticRefStr>` | 设置 trace state 用于关联元数据，覆盖已有值 |
| `with_trace_state` | `impl Into<StaticRefStr>` | 设置 trace state，仅当未设置时生效 |
| `with_trace_event` | `TraceEvent` | 添加追踪事件到报告 |
| `push_trace_event` | `impl Into<StaticRefStr>` | 追加一个默认字段的 trace 事件 |
| `push_trace_event_with` | `(impl Into<StaticRefStr>, Option<TraceEventLevel>, Option<u64>, impl IntoIterator<Item = TraceEventAttribute>)` | 追加一个完整指定的 trace 事件 |
| `capture_stack_trace` | 无 | (std) 捕获当前堆栈 (若已存在则跳过) |
| `force_capture_stack` | 无 | (std) 强制重新捕获堆栈 |
| `clear_stack_trace` | 无 | 移除已关联的堆栈信息 |

### 快捷渲染入口
| 方法 | 返回类型 | 说明 |
| :--- | :--- | :--- |
| `compact()` | `impl Display` | 仅输出原始错误消息 |
| `pretty()` | `impl Display` | 输出人类友好的分段详细诊断 (默认配置) |
| `json()` | `impl Display` | 输出符合 Schema 的 JSON 字符串 |
| `render(R)` | `impl Display` | 使用指定的渲染器渲染 |

### 用法示例
```rust
use diagweave::prelude::*;
use std::fmt;

#[derive(Debug)]
enum MyError {
    Timeout,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "timeout")
    }
}

impl std::error::Error for MyError {}

let report = Report::new(MyError::Timeout)
    .with_severity(Severity::Fatal)
    .with_ctx(
        "request_id",
        "req-123",
    )
    .attach_note("please check the network connection")
    .with_retryable(true)
    .attach_payload("data", vec![1, 2, 3], Some("application/octet-stream"));
#[cfg(feature = "std")]
let report = report.capture_stack_trace();
```

---

## 2. 渲染与输出 (Rendering)

### 概览
将包含丰富元数据的 `Report` 转换为可展示的字符串或结构化数据。

### 渲染配置 (`ReportRenderOptions`)
| 配置项 | 默认值 | 说明 |
| :--- | :--- | :--- |
| `show_type_name` | `true` | 是否显示错误的 Rust 类型全名 |
| `max_source_depth`| `16` | 递归收集 `source()` 的深度限制 |
| `detect_source_cycle`| `true` | 是否检测并终止循环原因链 |
| `pretty_indent` | `Spaces(2)`| `Pretty` 渲染的缩进风格 (支持 `Tab`) |
| `json_pretty` | `false` | JSON 输出是否带格式化缩进 |
| `show_empty_sections` | `true` | 是否展示没有内容的片段 (如 Trace 为空时) |
| `show_cause_chains_section` | `true` | 是否显示原因链 (Causes) 部分 |
| `show_context_section`| `true` | 是否显示上下文关联词部分 |
| `show_attachments_section`| `true` | 是否显示附件 (Payload/Note) 部分 |
| `show_stack_trace_section`| `true` | 是否显示堆栈轨迹部分 |
| `show_trace_section` | `true` | 是否显示分布式追踪 (TraceID/Event) 部分 |
| `show_trace_event_details` | `true` | 是否在 Pretty/JSON 中显示 trace 事件的 level、timestamp、attributes |
| `stack_trace_max_lines` | `24` | 原始堆栈渲染的最大行数截断 |
| `stack_trace_include_raw` | `true` | 渲染堆栈时是否包含原始堆栈输出 |
| `stack_trace_include_frames` | `true` | 渲染堆栈时是否包含解析后的帧信息 |
| `stack_trace_filter` | `All` | 堆栈帧过滤策略：`All`（全部）、`AppOnly`（过滤标准库帧）、`AppFocused`（额外过滤诊断内部帧） |

预设配置：
| 预设 | 说明 |
| :--- | :--- |
| `ReportRenderOptions::developer()` | 开发模式：完整 trace 事件详情，不过滤堆栈，最多 50 行 |
| `ReportRenderOptions::production()` | 生产排障模式：trace 事件详情，仅应用层帧，最多 15 行 |
| `ReportRenderOptions::minimal()` | 最小模式：隐藏 trace 事件详情，聚焦关键帧，最多 5 行，隐藏空段和类型名 |
| `stack_trace_filter` | `All` | 堆栈帧过滤策略：`All`（全部）、`AppOnly`（过滤标准库帧）、`AppFocused`（额外过滤诊断内部帧） |

预设配置：
| 预设 | 说明 |
| :--- | :--- |
| `ReportRenderOptions::developer()` | 开发模式：完整 trace 事件详情，不过滤堆栈，最多 50 行 |
| `ReportRenderOptions::production()` | 生产排障模式：trace 事件详情，仅应用层帧，最多 15 行 |
| `ReportRenderOptions::minimal()` | 最小模式：隐藏 trace 事件详情，聚焦关键帧，最多 5 行，隐藏空段和类型名 |


### 诊断中间表示 (`DiagnosticIr`)
渲染器不直接处理 `Report`，而是先通过 `to_diagnostic_ir()` 转换为稳定的 IR 结构。该 IR 保留错误节点、元数据、trace 引用、附件、展示原因、原生传播链、诊断补充链，以及附件相关部分的聚合计数。
```rust
use diagweave::render::{
    DiagnosticIrError, DiagnosticIrMetadata,
};
use diagweave::report::{
    Attachment, CauseTraversalState, MissingSeverity, SourceErrorChain,
};
use std::fmt::Display;
use std::sync::Arc;
#[cfg(feature = "trace")]
use diagweave::report::ReportTrace;
#[cfg(feature = "json")]
use diagweave::StaticRefStr;

pub struct DiagnosticIr<'a, State = MissingSeverity> {
    #[cfg(feature = "json")]
    pub schema_version: StaticRefStr,
    pub error: DiagnosticIrError<'a>,
    pub metadata: DiagnosticIrMetadata<'a, State>,
    #[cfg(feature = "trace")]
    pub trace: &'a ReportTrace,
    pub attachments: &'a [Attachment],
    pub display_causes: &'a [Arc<dyn Display + Send + Sync + 'static>],
    pub display_causes_state: CauseTraversalState,
    pub origin_source_errors: Option<SourceErrorChain>,
    pub diagnostic_source_errors: Option<SourceErrorChain>,
}
```

`DiagnosticIrMetadata` 现在将内部字段完全私有化，并通过 `error_code()`、`severity()`、`category()`、`retryable()`、`stack_trace()` 等接口对外暴露只读访问。

逐项访问上下文/note/payload 由 `Report::visit_attachments(...)` 提供。

这样使用：
```rust
use diagweave::render::ReportRenderOptions;

# use diagweave::prelude::{AttachmentValue, Report};
# #[derive(Debug)]
# struct DemoError;
# impl core::fmt::Display for DemoError {
#     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
#         write!(f, "demo error")
#     }
# }
# impl std::error::Error for DemoError {}
# let report = Report::new(DemoError)
#     .with_ctx(
#         "request_id",
#         "req-42",
#     )
#     .attach_printable("note")
#     .attach_payload("body", AttachmentValue::from("ok"), Some("text/plain"))
#     .with_display_cause("retry later")
#     .with_diag_src_err(std::io::Error::other("upstream"));

let ir = report.to_diagnostic_ir();

let context_count = ir.context.len();
let attachment_count = ir.attachments.len();
println!("context_count={context_count}, attachment_count={attachment_count}");
```

`DiagnosticIr` 会保留 `display_causes` 以及两条 source 链作为结构化数据。在 JSON 契约中，`origin_source_errors.type` 与 `diagnostic_source_errors.type` 都是 `string | null`；其中 `origin` 更常见 `null`，因为自然 `Error::source()` 会有信息损耗。
IR 与适配器层采用借用优先策略：错误/type/trace 等字符串投影尽量使用 `RefStr<'a>`，因此 `to_tracing_fields()` 和 `to_otel_envelope_default()` 在热点路径上会减少不必要的 `String` 物化。OTEL 导出被有意限制在 `DiagnosticIr<'a, HasSeverity>` 上。

### 用法示例
```rust
use diagweave::prelude::{Pretty, Report, ReportRenderOptions};
use diagweave::render::PrettyIndent;

let inner = std::io::Error::new(std::io::ErrorKind::Other, "oops");
let report = Report::new(inner);

// 1. 直接打印 Pretty 格式 (Stdout)
println!("{}", report.pretty());

// 2. 自定义 Pretty 布局
println!("{}", report.render(Pretty {
    options: ReportRenderOptions {
        pretty_indent: PrettyIndent::Tab,
        max_source_depth: 5,
        ..Default::default()
    }
}));

// 3. 生成 JSON
#[cfg(feature = "json")]
let json_str = report.json().to_string();
```

---

## 3. 云原生适配 (OpenTelemetry)

### 概览
`diagweave` 提供与 OpenTelemetry (OTel) 规范深度集成的适配器，支持将丰富的诊断数据转换为符合 OTLP 日志数据模型的记录批次。这里需要显式开启 `otel` feature。

### 转换 API
| 方法声明 | 返回类型 | 说明 |
| :--- | :--- | :--- |
| `ir.to_otel_envelope(config)` | `OtelEnvelope<'a>` | 仅在 `DiagnosticIr<'a, HasSeverity>` 上可用；转换为 OTLP 风格的日志/事件记录批次，并支持 `OtelEnvelopeConfig` 的命名空间控制 |
| `ir.to_otel_envelope_default()` | `OtelEnvelope<'a>` | 兼容性快捷入口，使用默认 OTEL 命名行为 |
| `ir.to_tracing_fields()` | `Vec<TracingField<'a>>`| 转换为 KV 形式 of Tracing/Logging 字段 |

### OTel 映射逻辑
1. **记录字段**: 主报告会变成一个日志记录，严重程度、时间戳相关元数据、trace 关联字段和结构化 `body` 错误节点会放在顶层。
2. **属性**: 错误核心字段、重试/分类标记、原因链摘要以及附件/上下文数据会以结构化 OTEL 属性输出。diagweave 自有 key 可通过 `OtelEnvelopeConfig` 统一加 namespace，而 `exception.type` 这类 OTEL 语义约定字段保持不变。
3. **Trace 事件**: `Report` 内部的 `TraceEvent` 会转换成额外的 OTLP 风格日志/事件记录，带各自的时间戳、严重程度和 trace 关联字段。
4. **结构保留**: `exception.stacktrace` and `diagnostic_bag.origin_source_errors / diagnostic_bag.diagnostic_source_errors` 不再被字符串扁平化。

---

## 4. Feature Flags (特性开关)

| Feature | 默认开启 | 说明 |
| :--- | :--- | :--- |
| `std` | 是 | 标准库集成 (捕获堆栈、全局注入器等) |
| `json` | 否 | `Json` 渲染器支持 (依赖 `serde` and `serde_json`) |
| `trace` | 否 | Trace 数据模型 (`ReportTrace` 等)、预校验后的发射 typestate (`PreparedTracingEmission`) 与可插拔导出器 Trait (`TracingExporterTrait`) |
| `otel` | 否 | OTLP envelope 模型 (`OtelEnvelope`、`OtelEvent`、`OtelValue`)、`OtelEnvelopeConfig`，以及 `DiagnosticIr<'_, HasSeverity>` 上的 `to_otel_envelope(config)` / `to_otel_envelope_default()` |
| `tracing` | 否 | 默认 `tracing` 生态集成 (`TracingExporter`、`prepare_tracing`、`emit_tracing`)。会自动开启 `trace`。 |

### 依赖矩阵
- **`no_std`**: 通过关闭默认特性支持。需要 `alloc`。
- **`json`**: 需要 `serde` (含 `derive` 和 `alloc` 特性) 以及 `serde_json` (含 `alloc` 特性)。
- **`trace`**: 无额外外部依赖的 Trace 数据结构。
- **`otel`**: 本身不引入额外依赖，但需要显式开启后才能导出 OTLP envelope。
- **`tracing`**: 依赖 `tracing` crate。
