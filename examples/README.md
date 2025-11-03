# RootCause Examples

Demonstrations of rootcause features and patterns.

## Fundamentals

- **basic.rs** - Core concepts: error coercion with `?`, `.context()`, `.attach()`, building chains
- **typed_reports.rs** - `Report<C>` vs `Report<dyn Any>`, pattern matching for error recovery
- **error_coercion.rs** - Automatic coercion between `C`, `Report<C>`, and `Report<dyn Any>`
- **error_chains.rs** - Chaining operations with different error types, lazy evaluation with `.attach_with()`

## Collections

- **retry_with_collection.rs** - Accumulate multiple errors with `ReportCollection`, retry patterns
- **batch_processing.rs** - Batch processing with `IteratorExt::collect_reports()`, partial success

## Custom Types & Handlers

- **thiserror_interop.rs** - Using thiserror errors as contexts, compatibility patterns
- **custom_attachments.rs** - Custom types for structured data you can retrieve and inspect programmatically
- **custom_handler.rs** - Per-attachment/context formatting (contrast: formatting hooks are global)

## Hooks & Formatting

- **formatting_hooks.rs** - Global formatting overrides: placement, priority, custom context display
- **report_creation_hook.rs** - Automatic attachment on creation: simple collectors vs conditional logic
- **conditional_formatting.rs** - Conditional formatting based on runtime context (environment, feature flags, etc.)

## Running Examples

```bash
# Run any example
cargo run --example <name>

# For example
cargo run --example basic

# Build all examples
cargo build --examples
```
