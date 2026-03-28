---
description: "Instructions specific to rootcause documentation"
applyTo: "**/*.rs"
---

# Documentation Style Guide for Rootcause

This document establishes consistent standards for documentation across the rootcause library.

## Overall Tone and Voice

- **Technical but approachable**: Assume readers are Rust developers but may be new to advanced error handling concepts
- **Clear and direct**: Avoid unnecessary jargon while being precise about technical details
- **Helpful and encouraging**: Guide users toward success rather than just documenting what exists

## Documentation Depth by Visibility

Different visibility levels require different levels of documentation detail:

### Public Items (`pub`)

**These are the user-facing API and require comprehensive documentation:**

- Complete documentation with multiple examples showing different use cases
- Clear explanations of when and why to use the item
- Full cross-references to related functionality
- Panics, errors, and safety sections when applicable
- Multiple examples demonstrating different scenarios
- Target audience: Library users
- **Required** for all public items

### Internal Items (`pub(crate)`, private, or private modules)

**These are implementation details for maintainers:**

- Brief, concise documentation explaining what it does and why it exists
- No need for extensive examples or multiple scenarios
- Focus on implementation details rather than user-facing usage
- Target audience: Library developers and contributors
- **Optional** - add documentation when it meaningfully helps readers understand the implementation, but it's not required for all internal items

**Example:**

````rust
/// Creates a report with the given context.
///
/// This is the primary way to create reports. Use the `report!()` macro
/// for more convenient report creation with format string support.
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
/// let report: Report<&str> = Report::new("error message");
/// ```
pub fn new(context: C) -> Report { /* ... */ }

/// Internal helper to construct a report without triggering hooks.
///
/// Used by preformat() to avoid infinite recursion.
pub(crate) fn from_parts_unhooked(/* ... */) -> Report { /* ... */ }

// Helper to validate report structure invariants.
fn check_invariants(&self) -> bool { /* ... */ }
````

## Structure Patterns

### Module-Level Documentation (`//!`)

1. **Hook line** (1-2 sentences): What this module/crate does
2. **Overview section**: Broader context and main concepts (2-3 paragraphs)
3. **Core concepts section** (if complex): Break down key ideas with examples
4. **Usage examples**: Show common patterns
5. **Cross-references**: Link to related modules/types

### Item-Level Documentation (`///`)

1. **Summary line**: One sentence describing what the item does
2. **Detailed explanation** (if needed): How it works, when to use it
3. **Examples**: Practical usage demonstration
4. **Errors/Panics/Safety** (if applicable): Important behavioral notes
5. **See also**: Cross-references to related items

## Language Conventions

### Terminology Consistency

- **"Report"** (capitalized) when referring to the type
- **"report"** (lowercase) when referring to an instance
- **"context"** for the root node's data
- **"attachment"** for additional data added to nodes
- **"attachment data"** for the actual data stored in attachments
- **"handler"** for types that process contexts/attachments
- **"hook"** for customization points in the reporting process

### Common Phrases

- "This allows you to..." (not "This lets you...")
- "You can use this to..." (for explaining purpose)
- "Returns a new..." (for constructors)
- "Converts this..." (for transformation methods)
- "Note that..." (for important caveats)

### Code References

- **Use intra-doc links for types**: [`Report`], [`Error`] (not plain `Report` or `Error`)
- **Use intra-doc links for methods**: [`Report::new`], [`into_dyn_any`] or [`into_dyn_any()`]
- **Use intra-doc links for modules**: [`crate::handlers`]
- **Use full paths for external crates**: [`std::error::Error`]
- **Especially important for internal references**: Always use [`ReportRef`], [`ReportMut`], [`Cloneable`], etc. rather than plain backticks
- **Exception for well-known standard library types**: Don't use intra-doc links for `String`, `Vec`, or other ubiquitous standard library types.

**Link syntax variants**: When linking, prefer keeping the identifier itself in backticks (e.g., [`Debug`] rather than [Debug handler] with reference-style links), unless the prose-style version flows significantly better in context.

- **Rationale**: Intra-doc links enable IDE navigation and rustdoc verification of link validity

## Example Standards

### Code Example Requirements

- **Always include type annotations**: Use explicit types on let bindings to help readers understand what they're working with. Only leave out the type annotations when they are truly obvious from context.
- **Use imports**: Prefer `use` statements over full type paths in examples
- **Prefer `report!()` macro**: Use `report!()` instead of `Report::new()` unless specifically demonstrating the constructor
- **Include informative type parameters**: Only show type parameters that help the reader understand the example
- **Use `'_` for lifetimes**: When lifetime parameters are needed, use `'_` unless the specific lifetime is important
- **Use `std` in examples**: While this is a `no_std` crate, documentation examples run as normal Rust. Prefer `std::` imports (e.g., `std::error::Error`, `std::fmt`) over `core::` or `alloc::` in examples, as they are more familiar and easier for readers to understand. The actual library code should still use `core::` and `alloc::` appropriately.

### Good Example Structure

````rust
/// Creates a new report with the given context.
///
/// This allocates a new root node containing the provided context.
/// The report starts with no children or attachments.
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
///
/// let report: Report<&str> = report!("Something went wrong");
/// println!("{}", report);
/// ```
///
/// For formatted messages:
///
/// ```
/// use rootcause::prelude::*;
///
/// let error_code = 404;
/// let report: Report = report!("Error {}: Not found", error_code);
/// println!("{}", report);
/// ```
///
/// # See Also
///
/// - [`Report::context`] for adding context to existing reports
/// - [`IntoReport::into_report`] for converting from other error types
pub fn new(context: C) -> Report<C, Mutable, SendSync> {
    // implementation
}
````

### `report!()` Macro Usage

The `report!()` macro has two forms and should be used appropriately:

**Format string form** (returns `Report<Dynamic, Mutable, SendSync>`):

```rust
use rootcause::prelude::*;

let error_code = 500;
let report: Report = report!("Server error: {}", error_code);
```

**Expression form** (returns `Report<C, Mutable, T>` where `C` and `T` are inferred):

```rust
use rootcause::prelude::*;

let custom_error = MyError::new("database connection failed");
let report: Report<MyError> = report!(custom_error);
```

### Type Parameter Guidelines

- **Context type (`C`)**: Include when it helps understanding (e.g., `Report<MyError>`, `Report<&str>`)
- **Ownership marker**: Usually omit `Mutable` unless comparing with `Cloneable`
- **Thread safety marker**: Usually omit `SendSync` unless comparing with `Local` or when `Local` is used
- **`Dynamic`**: Usually omit (it's the default) unless explicitly demonstrating type erasure or comparing with typed reports

**Good examples:**

```rust
let report: Report = report!("error message");
let report: Report<MyError, Cloneable> = report!(my_error).into_cloneable();
let report: Report<Dynamic, Mutable, Local> = report!(non_send_error).into_local();
```

**Avoid unless necessary:**

```rust
// Too verbose for most examples
let report: Report<&str, Mutable, SendSync> = report!("error");
```

### Table Formatting

Use consistent table formatting with proper alignment:

```markdown
| Variant                | Feature A | Feature B | Description                                     |
| ---------------------- | --------- | --------- | ----------------------------------------------- |
| `Type<Param1, Param2>` | ✅        | ❌        | Clear, concise description of what this enables |
| `Type<Param3, Param4>` | ❌        | ✅        | Another clear description                       |
```

## Section Naming

### Standard Section Headers

- **Overview** - High-level introduction
- **Core Concepts** - Key ideas users need to understand
- **Usage Examples** - Common patterns and use cases
- **Variants** or **Configuration** - Different ways to use the API
- **Converting Between Types** - Transformation patterns
- **Performance Notes** - When relevant to user decisions
- **Limitations** - Important constraints or trade-offs
- **See Also** - Cross-references and related functionality

### Method Documentation Sections

- **Examples** - Always include when helpful
- **Panics** - When the method can panic
- **Errors** - For fallible operations
- **Safety** - For unsafe methods
- **Performance** - When non-obvious performance characteristics exist

## Cross-Reference Patterns

- Link to types on first mention in a section: [`Report`]
- Link to methods with their parent type: [`Report::new`]
- Link to external crates with full URLs on first mention
- Use relative links for internal modules: [`crate::handlers`]
- Group related links in "See Also" sections

## Code Example Guidelines

- **Always compile**: Use `# ` for hidden setup code if needed
- **Show realistic usage**: Avoid trivial examples unless demonstrating basic syntax
- **Include error handling**: Show how errors propagate in examples
- **Use consistent imports**: Prefer `use rootcause::prelude::*;` or specific imports
- **Keep examples focused**: One concept per example
- **Demonstrate the `report!()` macro**: Use it as the primary way to create reports
- **Show type inference**: Let readers see how types flow through the API
- **Prefer `std` in examples**: Documentation examples should use `std::` types and imports (e.g., `std::error::Error`, `std::fmt::Display`) rather than `core::` or `alloc::`, as readers are more familiar with the standard library. The library implementation itself remains `no_std` compatible.

## Import Patterns

### Preferred Import Styles

**For most examples:**

```rust
use rootcause::prelude::*;
```

**For specific functionality:**

```rust
use rootcause::{Report, report};
use rootcause::handlers::Display;
```

**Avoid in examples unless necessary:**

```rust
// Too verbose for examples
rootcause::Report::<&str, rootcause::markers::Mutable, rootcause::markers::SendSync>
```

## Related Guidelines

This document focuses on documentation standards. For Rust coding conventions and API design, see [`rust.instructions.md`](rust.instructions.md).

## Avoiding Common Issues

- **Don't assume prior knowledge** of error handling libraries
- **Explain the "why"** not just the "what" for complex features
- **Use active voice** when possible
- **Break up long paragraphs** with subheadings or lists
- **Test code examples** - they should actually compile and run
- **Keep line lengths reasonable** in documentation (80-100 chars)
- **Show type annotations** to help readers understand the API
- **Prefer macros over constructors** in examples unless specifically teaching about constructors

## Testing Documentation

When building or checking documentation, always use `--all-features` to ensure intra-doc links work correctly:

```bash
cargo doc --all-features --no-deps
```

Without `--all-features`, some cross-references between feature-gated items may not resolve properly, causing broken links in the documentation.

## Review Checklist

For each documentation update, verify:

- [ ] Summary sentence clearly explains the purpose
- [ ] Technical terms are defined on first use
- [ ] Examples compile and demonstrate real usage
- [ ] Cross-references use correct syntax and resolve properly
- [ ] Tables are properly formatted and aligned
- [ ] Tone is consistent with established voice
- [ ] No typos or grammatical errors
- [ ] Links to external resources are current and accurate
- [ ] Examples use type annotations on let bindings
- [ ] Examples prefer `report!()` macro over `Report::new()`
- [ ] Type parameters shown are informative and not overwhelming
- [ ] Import statements are included and follow preferred patterns
- [ ] Documentation builds successfully with `cargo doc --all-features --no-deps`
