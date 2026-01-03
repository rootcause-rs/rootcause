# RootCause Examples

Demonstrations of rootcause features and patterns.

## Fundamentals

**New to rootcause?** Start with [`basic.rs`](basic.rs).

- [`basic.rs`](basic.rs) - Core concepts: `?` operator, `.context()`, `.attach()`, building error chains
- [`custom_errors.rs`](custom_errors.rs) - Creating errors with `report!()`: string messages, custom types, mixing approaches
- [`lazy_evaluation.rs`](lazy_evaluation.rs) - Lazy evaluation: `.attach_with()` and `.context_with()` for expensive computations
- [`typed_reports.rs`](typed_reports.rs) - Type-safe errors with `Report<C>`, pattern matching for error recovery
- [`error_coercion.rs`](error_coercion.rs) - How `?` automatically converts between error types - mixing typed and dynamic
- [`context_methods.rs`](context_methods.rs) - Comparing context transformation methods: `context()`, `context_transform()`, `context_transform_nested()`, `context_to()`

## Collections

- [`retry_with_collection.rs`](retry_with_collection.rs) - Accumulate multiple errors with `ReportCollection`, retry patterns
- [`batch_processing.rs`](batch_processing.rs) - Three error collection strategies: standard `.collect()` vs `.collect_reports()` vs manual loop for partial success

## Inspection & Analysis

- [`inspecting_errors.rs`](inspecting_errors.rs) - Programmatic tree traversal and data extraction: `.iter_reports()`, `.downcast_current_context()`, analytics patterns

## Integration & Migration

**Bidirectional conversion** with other error libraries:

- [`anyhow_interop.rs`](anyhow_interop.rs) - Quick reference for anyhow conversion APIs: `.into_rootcause()`, `.into_anyhow()`, `From<Report>`
- [`eyre_interop.rs`](eyre_interop.rs) - Quick reference for eyre conversion APIs: `.into_rootcause()`, `.into_eyre()`
- [`error_stack_interop.rs`](error_stack_interop.rs) - Quick reference for error-stack conversion APIs: `.into_rootcause()`, `.into_error_stack()`
- [`boxed_error_interop.rs`](boxed_error_interop.rs) - Quick reference for boxed error conversion APIs: `.into_rootcause()`, `.into_boxed_error()`, preserving thread safety

**Migration guides:**

- [`anyhow_migration.rs`](anyhow_migration.rs) - Gradual migration from anyhow: 5 stages showing top-down adoption strategy

**Using derive macro errors with rootcause:**

- [`thiserror_interop.rs`](thiserror_interop.rs) - Using thiserror-generated errors: pattern matching on `Report<E>`, comparison of `#[from]` nesting vs `.context()` chains
- [`derive_more_interop.rs`](derive_more_interop.rs) - Using derive_more-generated errors: same patterns as thiserror but with `#[display]` instead of `#[error]`

## Custom Types & Handlers

- [`custom_attachments.rs`](custom_attachments.rs) - Custom types for structured data you can retrieve and inspect programmatically
- [`custom_handler.rs`](custom_handler.rs) - Per-attachment/context formatting (contrast: formatting hooks are global)

## Hooks & Formatting

- [`formatting_hooks.rs`](formatting_hooks.rs) - Global formatting overrides: placement, priority, custom context display
- [`report_creation_hook.rs`](report_creation_hook.rs) - Automatic attachment on creation: simple collectors vs conditional logic
- [`conditional_formatting.rs`](conditional_formatting.rs) - Conditional formatting based on runtime context (environment, feature flags, etc.)

## Tracing Integration

- [`tracing_spans.rs`](tracing_spans.rs) - Automatic span capture with full hierarchy and field values

## Running Examples

```bash
# Run any example
cargo run --example <name>

# For example
cargo run --example basic

# Build all examples
cargo build --examples
```
