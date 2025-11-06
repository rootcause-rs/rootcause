# RootCause Examples

Demonstrations of rootcause features and patterns.

## Fundamentals

**New to rootcause?** Start with [`basic.rs`](basic.rs).

- [`basic.rs`](basic.rs) - Core concepts: `?` operator, `.context()`, `.attach()`, building error chains
- [`custom_errors.rs`](custom_errors.rs) - Creating errors with `report!()`: string messages, custom types, mixing approaches
- [`lazy_evaluation.rs`](lazy_evaluation.rs) - Lazy evaluation: `.attach_with()` and `.context_with()` for expensive computations
- [`typed_reports.rs`](typed_reports.rs) - Type-safe errors with `Report<C>`, pattern matching for error recovery
- [`error_coercion.rs`](error_coercion.rs) - Understanding automatic type conversions between error types

## Collections

- [`retry_with_collection.rs`](retry_with_collection.rs) - Accumulate multiple errors with `ReportCollection`, retry patterns
- [`batch_processing.rs`](batch_processing.rs) - Batch processing with `IteratorExt::collect_reports()`, partial success

## Inspection & Analysis

- [`inspecting_errors.rs`](inspecting_errors.rs) - Programmatic tree traversal and data extraction: `.iter_reports()`, `.downcast_current_context()`, analytics patterns

## Custom Types & Handlers

- [`thiserror_interop.rs`](thiserror_interop.rs) - Using thiserror errors as contexts, compatibility patterns
- [`custom_attachments.rs`](custom_attachments.rs) - Custom types for structured data you can retrieve and inspect programmatically
- [`custom_handler.rs`](custom_handler.rs) - Per-attachment/context formatting (contrast: formatting hooks are global)

## Hooks & Formatting

- [`formatting_hooks.rs`](formatting_hooks.rs) - Global formatting overrides: placement, priority, custom context display
- [`report_creation_hook.rs`](report_creation_hook.rs) - Automatic attachment on creation: simple collectors vs conditional logic
- [`conditional_formatting.rs`](conditional_formatting.rs) - Conditional formatting based on runtime context (environment, feature flags, etc.)

## Running Examples

```bash
# Run any example
cargo run --example <name>

# For example
cargo run --example basic

# Build all examples
cargo build --examples
```
