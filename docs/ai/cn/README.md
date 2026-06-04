# 核心开发参考 (面向 AI)

本目录包含 `diagweave` 核心诊断库的详细开发参考。

## 目录大纲

1. [错误定义与转换](./error_definition_and_conversion.md)
   - `set!` 宏
   - `union!` 宏
   - `#[derive(Error)]` 派生宏
   - `Result` 扩展特质 (`Diagnostic` / `ResultReportExt`)
   - 展示原因收集
   - 日志系统集成 (`Tracing`)
   - 高阶模式

2. [诊断报告容器](./diagnostic_report_container.md)
   - `Report<E>` 诊断报告
   - `ReportOptions` 和 `GlobalConfig`
   - `ErrorCode` 设计与转换规则
   - 渲染与输出 (Rendering)
   - 云原生适配 (OpenTelemetry)
   - Feature Flags (特性开关)
