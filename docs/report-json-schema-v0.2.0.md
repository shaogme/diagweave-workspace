# Report JSON Schema v0.2.0

This document defines the machine-consumable JSON contract emitted by `diagweave` when using the `Json` renderer.

- Schema version: `v0.2.0`
- Draft: JSON Schema 2020-12
- Canonical schema file: `diagweave/schemas/report-v0.2.0.schema.json`
- Related OTEL schema: [`docs/report-otel-schema-v0.1.0.md`](docs/report-otel-schema-v0.1.0.md)

## Stable payload fields

- `schema_version: string` (const: `v0.2.0`)
- `error: { message: string, type: string }`
- `metadata: { error_code: string|integer|null, severity: "trace"|"debug"|"info"|"warn"|"error"|"fatal"|null, category: string|null, retryable: boolean|null }`
- `diagnostic_bag: { stack_trace: StackTrace|null, display_causes: DisplayCauseChain|null, origin_source_errors: SourceErrorChain|null, diagnostic_source_errors: SourceErrorChain|null }`
- `trace: { context: TraceContext, events: TraceEvent[] }`
- `context: { entries: Array<{ key: string, value: ContextValue }> }` (business context entries; keys are non-empty strings and may repeat)
- `system: { entries: Array<{ key: string, value: ContextValue }> }` (system context entries; same structure as context)
- `attachments: Array<Note|Payload>`

## StackTrace model

The `stack_trace` object uses a discriminated union based on the `format` field:

### Native format

When `format: "native"`:
- `diagnostic_bag.stack_trace.format: "native"` (required)
- `diagnostic_bag.stack_trace.frames: array` (required)
- `diagnostic_bag.stack_trace.frames[*].symbol: string|null`
- `diagnostic_bag.stack_trace.frames[*].module_path: string|null`
- `diagnostic_bag.stack_trace.frames[*].file: string|null`
- `diagnostic_bag.stack_trace.frames[*].line: integer|null`
- `diagnostic_bag.stack_trace.frames[*].column: integer|null`

### Raw format

When `format: "raw"`:
- `diagnostic_bag.stack_trace.format: "raw"` (required)
- `diagnostic_bag.stack_trace.raw: string` (required)

### Example (native format)

```json
{
  "stack_trace": {
    "format": "native",
    "frames": [
      { "symbol": "main::inner", "file": "src/main.rs", "line": 42 }
    ]
  }
}
```

### Example (raw format)

```json
{
  "stack_trace": {
    "format": "raw",
    "raw": "0: std::backtrace_rs::backtrace::win64::trace\n1: ..."
  }
}
```

## DisplayCauseChain model

- `diagnostic_bag.display_causes.items[*]: string`
- `diagnostic_bag.display_causes.truncated: boolean`
- `diagnostic_bag.display_causes.cycle_detected: boolean`

## SourceErrorChain model

- `diagnostic_bag.origin_source_errors.roots[*]: integer` (node ids of top-level roots)
- `diagnostic_bag.origin_source_errors.nodes[*].message: string`
- `diagnostic_bag.origin_source_errors.nodes[*].type: string|null`
- `diagnostic_bag.origin_source_errors.nodes[*].source_roots[*]: integer` (node ids of children)
- `diagnostic_bag.origin_source_errors.truncated: boolean`
- `diagnostic_bag.origin_source_errors.cycle_detected: boolean`

- `diagnostic_bag.diagnostic_source_errors.roots[*]: integer` (node ids of top-level roots)
- `diagnostic_bag.diagnostic_source_errors.nodes[*].message: string`
- `diagnostic_bag.diagnostic_source_errors.nodes[*].type: string|null`
- `diagnostic_bag.diagnostic_source_errors.nodes[*].source_roots[*]: integer` (node ids of children)
- `diagnostic_bag.diagnostic_source_errors.truncated: boolean`
- `diagnostic_bag.diagnostic_source_errors.cycle_detected: boolean`

## Trace model

- `trace.context.trace_id: string|null` (`string` must match `^[0-9A-Fa-f]{32}$`)
- `trace.context.span_id: string|null` (`string` must match `^[0-9A-Fa-f]{16}$`)
- `trace.context.parent_span_id: string|null` (`string` must match `^[0-9A-Fa-f]{16}$`)
- `trace.context.sampled: boolean|null`
- `trace.context.trace_state: string|null`
- `trace.context.flags: integer|null` (range: `0..=255`)
- `trace.events[*].name: string`
- `trace.events[*].level: "trace"|"debug"|"info"|"warn"|"error"|null`
- `trace.events[*].timestamp_unix_nano: integer|null`
- `trace.events[*].attributes: Array<{ key: string, value: AttachmentValue }>`

### Example `system` payload

```json
{
  "system": {
    "entries": [
      { "key": "service.name", "value": { "kind": "string", "value": "cloud-native-stack" } },
      { "key": "deployment.environment", "value": { "kind": "string", "value": "staging" } },
      { "key": "host.arch", "value": { "kind": "string", "value": "x86_64" } },
      { "key": "request_id", "value": { "kind": "string", "value": "req-20260327-0001" } }
    ]
  }
}
```

## Context model

- `context.entries[*].key` is a non-empty business context key.
- `context.entries[*].value` is a `ContextValue`.
- Repeated keys are preserved by emitting multiple entries with the same `key`.
- Entries are emitted sorted by key; values for the same key retain insertion order.

## System model

- `system.entries[*].key` is a non-empty system context key.
- `system.entries[*].value` is a `ContextValue`.
- Repeated keys are preserved by emitting multiple entries with the same `key`.
- emitters should use namespaced keys (e.g., `service.name`, `deployment.environment`) for organization

## ContextValue

`ContextValue` is a tagged union with these variants:

- `string`
- `integer`
- `unsigned`
- `float`
- `bool`
- `string_array`
- `integer_array`
- `unsigned_array`
- `float_array`
- `bool_array`
- `redacted`

## AttachmentValue

`AttachmentValue` is a tagged recursive union with these variants:

- `null`
- `string`
- `integer`
- `unsigned`
- `float`
- `bool`
- `array`
- `object`
- `bytes`
- `redacted`

## Rust JSON-facing APIs

When `feature = "json"` is enabled, the public JSON-related APIs include:

- `diagweave::render::Json` (renderer)
- `diagweave::render::REPORT_JSON_SCHEMA_VERSION`
- `diagweave::render::REPORT_JSON_SCHEMA_DRAFT`
- `diagweave::render::report_json_schema()`

For typed context modeling in report APIs:

- `diagweave::report::JsonContext`
- `diagweave::report::JsonContextEntry`

Use `report_json_schema()` for strict cross-service validation and compatibility checks.

See also the OpenTelemetry envelope schema in [`docs/report-otel-schema-v0.1.0.md`](docs/report-otel-schema-v0.1.0.md).
