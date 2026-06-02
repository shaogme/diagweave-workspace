# Core Development Reference (AI-Oriented)

This directory contains the detailed developer reference for the `diagweave` core diagnostics library.

## Table of Contents

1. [Error Definition and Conversion](./error_definition_and_conversion.md)
   - `set!` Macro
   - `union!` Macro
   - `#[derive(Error)]` Derive Macro
   - `Result` Extension Traits (`Diagnostic` / `ResultReportExt` / `InspectReportExt`)
   - Display Cause Collection
   - Log System Integration (`Tracing`)
   - Advanced Patterns

2. [Diagnostic Report Container](./diagnostic_report_container.md)
   - `Report<E>` Diagnostic Report
   - `ReportOptions` and `GlobalConfig`
   - `ErrorCode` Design and Conversions
   - Rendering and Output
   - Cloud-Native Adaptation (OpenTelemetry)
   - Feature Flags
