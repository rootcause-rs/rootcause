# RootCause Examples

Demonstrations of rootcause features and patterns.

## Fundamentals

### basic.rs

Core concepts: using `?` for coercion, `.context()`, `.attach()`, and building error chains.

**Run:** `cargo run --example basic`

### typed_reports.rs

When to use `Report<C>` vs `Report<dyn Any>`. Pattern matching on typed reports for intelligent error recovery.

**Run:** `cargo run --example typed_reports`

### error_coercion.rs

How `?` automatically coerces between `C`, `Report<C>`, and `Report<dyn Any>`. Mixing different error types in one function.

**Run:** `cargo run --example error_coercion`

### error_chains.rs

Advanced error handling: chaining operations with different error types, `.attach_with()` for lazy evaluation.

**Run:** `cargo run --example error_chains`

## Collections

### retry_with_collection.rs

Using `ReportCollection` to accumulate multiple errors. HTTP retry pattern showing all attempts.

**Run:** `cargo run --example retry_with_collection`

### iterator_ext.rs

Using `IteratorExt::collect_reports()` for batch processing, partial success handling, and filtered error collection.

**Run:** `cargo run --example iterator_ext`

## Custom Types & Handlers

### thiserror_interop.rs

Using thiserror-generated errors as Report contexts. Shows compatibility and recommended patterns for mixing thiserror with rootcause.

**Run:** `cargo run --example thiserror_interop`

### custom_attachments.rs

Creating custom attachment types with `Display` and `Debug` implementations.

**Run:** `cargo run --example custom_attachments`

### custom_handler.rs

Implementing `AttachmentHandler` for specialized formatting (hexdump, tables, JSON).

**Run:** `cargo run --example custom_handler`

## Hooks & Formatting

### formatting_hooks.rs

Using `AttachmentFormattingOverride` to control placement: inline, appendix, or hidden.

**Run:** `cargo run --example formatting_hooks`

### report_creation_hook.rs

Automatically attaching context when errors are created using `ReportCreationHook` and `AttachmentCollectorHook`.

**Run:** `cargo run --example report_creation_hook`

### conditional_formatting.rs

Environment-based formatting: hiding sensitive data in production, conditional metrics.

**Run:** `cargo run --example conditional_formatting`

## Quick Commands

```bash
# Build all examples
cargo build --examples

# Run a specific example
cargo run --example basic
```
