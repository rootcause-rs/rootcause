---
description: "Instructions specific to rootcause documentation"
applyTo: "**/*.rs"
---

# Documentation Style Guide for Rootcause

This document establishes consistent standards for documentation across the rootcause library.

## Core Philosophy

Documentation length tracks the amount of non-obvious information, not the item's visibility. `Vec::push` in std is two lines plus an example; the `Vec` type-level doc is an essay. Follow the same shape here: rich narrative at the crate/module/type level, lean contracts at the method level.

The guiding principles:

1. **Every sentence must beat the signature.** A doc sentence earns its place by telling the reader something they cannot see from the item's name and types. "Converts the error into a [`Report`]" on `fn into_report(self) -> Report<E>` fails this test. If nothing non-obvious remains, a single crisp summary line is a complete docstring.
2. **Document the contract, not the paraphrase.** Method prose is for what the signature can't say: allocation and cloning semantics (Arc refcount bump vs deep copy), whether hooks run, `#[track_caller]` location capture, effect on children and attachments, formatting interplay, panics.
3. **Push shared explanation up; keep siblings thin.** A method family (e.g. the `attach*` methods, the [`OptionExt`] methods, the `into_*` conversions) gets one thorough trait- or module-level explanation with when-to-use-which guidance. Each sibling then gets a one-liner plus a link. Never stamp a doc template across siblings: visible repetition trains readers to skip docs.
4. **Every example must assert something.** See [Example Standards](#example-standards).
5. **Right layer, no repetition.** Crate docs teach the problem and the mental model; module docs explain the subsystem's role and lifecycle; item docs state the contract. Don't restate a higher layer in a lower one.
6. **State behavior positively.** Say what the code does. Drop clauses that justify what it doesn't do ("...rather than exposing it via the source chain"). Don't add `compile_fail` doctests to assert negative type contracts; positive type contracts belong in compile-time unit tests (e.g. an `assert_send_sync` helper). In conceptual docs such as the [`markers`] module, a compile-fail example is acceptable only where the failure itself is the lesson being taught.
7. **Direct sentences, no filler.** Write "Allocates a new root node containing the context.", not "This allows you to...", "You can use this to...", or similar scaffolding.

## Documentation Depth by Visibility

### Public Items (`pub`)

- **Required** for all public items.
- A summary line stating the contract; further prose only where the contract has non-obvious parts (see principle 2).
- An example that asserts concrete behavior (see [Example Standards](#example-standards)).
- Panics, errors, and safety sections when applicable.
- Cross-references to the family-level explanation and closely related items.
- Target audience: library users.

### Internal Items (`pub(crate)`, private, or private modules)

- **Optional**: add documentation when it meaningfully helps readers understand the implementation.
- Brief and concise: what it does and why it exists.
- No examples required.
- Target audience: library developers and contributors.

## Structure Patterns

### Crate- and Module-Level Documentation (`//!`)

1. **Hook line** (1-2 sentences): what this module/crate does
2. **Overview**: the mental model and how this subsystem fits into the whole
3. **Core concepts** (if complex): key ideas, decision guidance (when to use which variant), lifecycle (e.g. when hooks run and what they see)
4. **Usage examples**: realistic multi-step scenarios (this is where the rich examples live, not on individual methods)
5. **Cross-references**: link to related modules/types

### Item-Level Documentation (`///`)

1. **Summary line**: one sentence stating the contract
2. **Detailed explanation** (only if the contract has non-obvious parts)
3. **Example**: a concrete behavioral assertion
4. **Errors/Panics/Safety** (if applicable)
5. **See also**: link to the family-level explanation where one exists

## Language Conventions

### Terminology Consistency

- **"Report"** (capitalized) when referring to the type
- **"report"** (lowercase) when referring to an instance
- **"context"** for the root node's data
- **"attachment"** for additional data added to nodes
- **"attachment data"** for the actual data stored in attachments
- **"handler"** for types that process contexts/attachments
- **"hook"** for customization points in the reporting process

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

Every public item carries an example, enforced in CI by the `rustdoc::missing_doc_code_examples` lint. Doctests are executable contracts: they pin down observable behavior and CI keeps them honest.

### What an Example Must Show

An example must **assert concrete behavior**: an input→output assertion or rendered output, ideally the behavior that distinguishes this method from its siblings. A bare invocation proves nothing:

```rust
// Bad: only proves the method can be called
let value: Option<String> = None;
let result = value.ok_or_report();
assert!(result.is_err());
```

Since error rendering is this crate's product, the rootcause analog of std's `assert_eq!(v.len(), 3)` is usually an assertion on rendered output or report structure:

```rust
// Good: pins down what the reader can't guess from the signature
let value: Option<String> = None;
let report = value.ok_or_report().unwrap_err();
assert!(format!("{report}").contains("String"));
```

Sibling methods each keep their own example, and the examples differ where the behavior differs (as `Option::is_some`/`is_none` do in std).

If an item genuinely has nothing to assert (e.g. a marker unit struct), skip the example with an explicit per-item opt-out:

```rust
#[cfg_attr(nightly_extra_checks, allow(rustdoc::missing_doc_code_examples))]
```

The `cfg_attr` gate is required: the lint is unstable, so a bare `#[allow(...)]` triggers an `unknown_lints` warning on stable rustdoc. The opt-out is visible in diffs and greppable, so skipping stays a deliberate, reviewed decision.

### Example Mechanics

- **Method examples are 2-5 visible lines**: use `# ` hidden lines for setup so the visible code is only the point being made. Richer multi-step scenarios belong in type- or module-level docs.
- **Always compile**: examples run as doctests.
- **Always include type annotations**: use explicit types on let bindings to help readers understand what they're working with. Only leave out the type annotations when they are truly obvious from context.
- **Use imports**: prefer `use` statements over full type paths; `use rootcause::prelude::*;` for most examples.
- **Prefer `report!()` macro**: use `report!()` instead of `Report::new()` unless specifically demonstrating the constructor.
- **Use `'_` for lifetimes**: when lifetime parameters are needed, use `'_` unless the specific lifetime is important.
- **Use `std` in examples**: while this is a `no_std` crate, documentation examples run as normal Rust. Prefer `std::` imports (e.g., `std::error::Error`, `std::fmt`) over `core::` or `alloc::` in examples. The actual library code should still use `core::` and `alloc::` appropriately.

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

- **Context type (`C`)**: include when it helps understanding (e.g., `Report<MyError>`, `Report<&str>`)
- **Ownership marker**: usually omit `Mutable` unless comparing with `Cloneable`
- **Thread safety marker**: usually omit `SendSync` unless comparing with `Local` or when `Local` is used
- **`Dynamic`**: usually omit (it's the default) unless explicitly demonstrating type erasure or comparing with typed reports

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

## Cross-Reference Patterns

- Link to types on first mention in a section: [`Report`]
- Link to methods with their parent type: [`Report::new`]
- Link to external crates with full URLs on first mention
- Use relative links for internal modules: [`crate::handlers`]
- Group related links in "See Also" sections

## Related Guidelines

This document focuses on documentation standards. For Rust coding conventions and API design, see [`rust.instructions.md`](rust.instructions.md).

## Testing Documentation

When building or checking documentation, always use `--all-features` to ensure intra-doc links work correctly:

```bash
cargo doc --all-features --no-deps
```

Without `--all-features`, some cross-references between feature-gated items may not resolve properly, causing broken links in the documentation.

## Review Checklist

For each documentation update, verify:

- [ ] Every sentence says something the signature doesn't
- [ ] The contract is stated where applicable: cloning/allocation semantics, hook behavior, panics, safety
- [ ] No filler phrasing ("This allows you to...") and no template stamped across sibling items
- [ ] Method families are explained once at trait/module level; siblings link to it
- [ ] Examples assert concrete behavior (input→output or rendered output), not bare invocation
- [ ] Method examples are 2-5 visible lines with `# ` hidden setup; rich scenarios live at type/module level
- [ ] Examples use type annotations and prefer `report!()` over `Report::new()`
- [ ] Behavior is stated positively; no justifications for what the code doesn't do
- [ ] Cross-references use intra-doc links and resolve properly
- [ ] Documentation builds successfully with `cargo doc --all-features --no-deps`
