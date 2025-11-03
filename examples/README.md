# RootCause Examples

Demonstrations of rootcause features and patterns.

## Fundamentals

- **basic.rs** - Core concepts: using `?` for coercion, `.context()`, `.attach()`, and building error chains
- **typed_reports.rs** - When to use `Report<C>` vs `Report<dyn Any>`, pattern matching for intelligent error recovery
- **error_coercion.rs** - How `?` automatically coerces between `C`, `Report<C>`, and `Report<dyn Any>`
- **error_chains.rs** - Chaining operations with different error types, `.attach_with()` for lazy evaluation

## Collections

- **retry_with_collection.rs** - Using `ReportCollection` to accumulate multiple errors, HTTP retry pattern
- **batch_processing.rs** - Batch processing with `IteratorExt::collect_reports()`, partial success handling

## Custom Types & Handlers

- **thiserror_interop.rs** - Using thiserror-generated errors as Report contexts, compatibility patterns
- **custom_attachments.rs** - Creating custom attachment types for structured data that can be retrieved and inspected programmatically
- **custom_handler.rs** - Custom handlers for per-attachment/per-context formatting (contrast with global formatting hooks)

## Hooks & Formatting

- **formatting_hooks.rs** - Global formatting overrides: control attachment placement (appendix vs inline), priority ordering, and custom context formatting
- **report_creation_hook.rs** - Automatically attaching context when errors are created with `ReportCreationHook`
- **conditional_formatting.rs** - Environment-based formatting: hiding sensitive data, conditional metrics

## Running Examples

```bash
# Run any example
cargo run --example <name>

# For example
cargo run --example basic

# Build all examples
cargo build --examples
```
