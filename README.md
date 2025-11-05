# rootcause

A flexible, ergonomic, and inspectable error reporting library for Rust.

[![Build Status](https://github.com/rootcause-rs/rootcause/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/rootcause-rs/rootcause/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/rootcause.svg)](https://crates.io/crates/rootcause)
[![Documentation](https://docs.rs/rootcause/badge.svg)](https://docs.rs/rootcause)
[![Discord](https://img.shields.io/discord/1430547172159651860.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Hs6ezQ6Y4U)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rootcause-rs/rootcause#license)

<img src="https://github.com/rootcause-rs/rootcause/raw/main/rootcause.png" width="192">

## Overview

rootcause helps you build rich, structured error reports that capture not just what went wrong, but the full context and history.

Here's a simple example showing how errors build up context as they propagate through your call stack:

```rust
use rootcause::prelude::*;

fn read_file(path: &str) -> Result<String, Report> {
    // The ? operator automatically converts io::Error to Report
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

fn load_config(path: &str) -> Result<String, Report> {
    // Add context to explain what this file is for
    let content = read_file(path)
        .context("Failed to load application configuration")?;
    Ok(content)
}

fn load_config_with_debug_info(path: &str) -> Result<String, Report> {
    // Attach debugging information
    let content = load_config(path)
        .attach(format!("Config path: {path}"))
        .attach("Expected format: TOML")?;
    Ok(content)
}

fn startup(config_path: &str, environment: &str) -> Result<(), Report> {
    let _config = load_config_with_debug_info(config_path)
        .context("Application startup failed")
        .attach(format!("Environment: {environment}"))?;
    Ok(())
}
```

When `startup()` fails, you get a chain showing the full story:

```
 ● Application startup failed
 ├ examples/basic.rs:76:10
 ├ Environment: production
 │
 ● Failed to load application configuration
 ├ examples/basic.rs:47:35
 ├ Config path: /nonexistent/config.toml
 ├ Expected format: TOML
 │
 ● No such file or directory (os error 2)
 ╰ examples/basic.rs:34:19
```

Each layer adds context and debugging information, building a trail from the high-level operation down to the root cause.

### Why rootcause instead of anyhow?

**Programmatic error inspection.** Unlike anyhow's linear error chains, rootcause exposes the full error tree structure for navigation and inspection:

```rust
use rootcause::prelude::*;

#[derive(Debug)]
enum ValidationError {
    Timeout,
    InvalidData,
}

fn process_batch(items: &[Item]) -> Result<(), Report> {
    match validate_and_save(items) {
        Ok(()) => Ok(()),
        Err(report) => {
            // Navigate the error tree to make intelligent decisions
            for attachment in report.attachments() {
                if let Some(err) = attachment.downcast_ref::<ValidationError>() {
                    match err {
                        ValidationError::Timeout => {
                            // Retry timeouts
                            return retry_with_backoff(items);
                        }
                        ValidationError::InvalidData => {
                            // Don't retry bad data
                            return Err(report);
                        }
                    }
                }
            }
            Err(report)
        }
    }
}
```

This tree navigation enables retry logic, recovery strategies, and error aggregation patterns that are impossible with anyhow's opaque error chains.

## Project Goals

- **Ergonomic**: The `?` operator should work with most error types, even ones not designed for this library
- **Fast happy path**: `Report` has a pointer-sized representation, keeping `Result<T, Report>` small and fast
- **Typable**: Users should be able to (optionally) specify the type of the context in the root node
- **Inspectable**: The objects in a Report should not be glorified strings. Inspecting and interacting with them should be easy
- **Cloneable**: It should be possible to clone a `Report` when you need to
- **Mergeable**: It should be possible to merge multiple `Report`s into a single one
- **Customizable**: It should be possible to customize what data gets collected, or how reports are formatted
- **Rich**: Reports should automatically capture information (like backtraces) that might be useful in debugging
- **Beautiful**: The default formatting should look pleasant—and if it doesn't match your style, the hook system lets you customize it

## Core Concepts

At a high level, rootcause helps you build a tree of error reports. Each node in the tree represents a step in the error's history - you start with a root error, then add context and attachments as it propagates up through your code.

Think of it like this: when a file read fails deep in your code, you can wrap it with context like "failed to load config", then wrap that with "app initialization failed". The result is a tree showing the full story of what went wrong.

**Why a tree?** Most of the time your error reports will be linear chains (just like anyhow), but the tree structure becomes useful when you need to:

- Collect multiple errors (e.g., validation errors from different fields) - see [Collecting Multiple Errors](#collecting-multiple-errors)
- Show retry attempts with different failures
- Represent parallel operations that each failed in different ways

If you're coming from anyhow and only need linear error chains, that's totally fine - rootcause handles that case efficiently.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rootcause = "0.6.0"
```

Use `Report` as your error type:

```rust
use rootcause::prelude::*;

fn your_function() -> Result<(), Report> {
    // Your existing code with ? already works
    std::fs::read_to_string("/path/to/file")?;
    Ok(())
}
```

That's it! The `?` operator automatically converts any error type to `Report`.

**Ready to learn more?** See [`examples/basic.rs`](examples/basic.rs) for a hands-on tutorial covering `.context()`, `.attach()`, and composing error chains.

## Next Steps

- **New to rootcause?** See [`examples/basic.rs`](examples/basic.rs) for a hands-on introduction
- **More examples:** Browse the [`examples/`](examples/) directory for common patterns
- **Full API documentation:** [docs.rs/rootcause](https://docs.rs/rootcause)

## Features

- **`std`** (default): Enable standard library support
- **`backtrace`**: Automatic backtrace capture on error creation

## Coming from other libraries?

### From `anyhow`

**Most of the API will feel familiar** - `.context()` works the same way and `?` just works.

**The key differences:**

1. **Structured attachments**: In anyhow, `.context()` adds another layer to the error chain. rootcause has both `.context()` (adds a new layer) AND `.attach()` (adds metadata to the _current_ layer). This lets you add debugging information without creating new tree levels.

2. **Full tree navigation**: anyhow's `.chain()` gives you a linear iterator over contexts. rootcause exposes the complete tree - you can iterate over children, inspect attachments at each level, and navigate the full structure programmatically.

3. **Cloneable errors**: anyhow errors cannot be cloned. rootcause supports `Report<dyn Any, Cloneable>` when you need to clone errors (e.g., for retry logic or caching).

4. **Thread-local errors**: anyhow requires all errors to be `Send + Sync`. rootcause supports `Report<dyn Any, Mutable, Local>` for thread-local data like `Rc` or `Cell`.

5. **Automatic location tracking**: By default, rootcause captures file locations (and optionally backtraces) every time `.context()` or `.attach()` is called, showing you exactly where each layer was added. anyhow only captures a single backtrace at error creation.

6. **Customization hooks**: Beyond location tracking, rootcause provides hooks to customize report creation (what data to capture) and formatting (how to display). anyhow's behavior is fixed.

7. **Optional type safety**: Use `Report<YourErrorType>` when you want compile-time guarantees about the root error type.

**When to switch:**

- ✅ You need `.attach()` to add metadata without creating new context layers
- ✅ You need to navigate complex error trees (not just linear chains)
- ✅ You need cloneable errors or thread-local error data
- ✅ You need to customize backtrace capture or error formatting
- ✅ You want optional type safety on error contexts
- ⏸️ Stick with anyhow if linear error chains meet your needs

**Migration cost:** Medium - `.context()` works the same, but tree navigation, attachments, and hooks are new concepts.

### From `error-stack`

rootcause takes inspiration from error-stack's typed error approach, with some key differences:

**Similarities:**

- Both support structured attachments via `.attach()`
- Both allow type-safe error contexts
- Both provide customization hooks

**Differences:**

1. **Optional typing**: `Report` (without type parameter) gives you anyhow-like ergonomics when you don't need type safety. error-stack requires every Report to have a specific context type.

2. **Explicit tree model**: rootcause is very clear about the underlying data structure - it's a tree where each node has context, attachments, and children. You can navigate this structure directly. error-stack's internal model is more opaque, exposing only high-level iteration methods.

3. **Hook flexibility**: rootcause's hook system is more flexible and fully documented, with clear explanations of when hooks fire and what data they receive. error-stack has hooks but they're less documented and more constrained.

4. **Additional type parameters**: Beyond context type, rootcause adds ownership (`Mutable`/`Cloneable`) and thread-safety (`SendSync`/`Local`) parameters for fine-grained control.

**When to switch:**

- ✅ You want the option to use untyped errors (like anyhow) when appropriate
- ✅ You need direct access to the error tree structure
- ✅ You need more flexible customization hooks
- ⏸️ Stick with error-stack if you prefer its more abstract API

**API naming differences:** `.context()` vs `.change_context()`, `.attach()` vs `.attach_printable()`, etc.

## Using Type Parameters

### Custom Context Types

If you want compile-time guarantees about the error type at the root of your Report, you can use a type parameter:

```rust
use rootcause::prelude::*;

#[derive(Debug)]
struct MyError {
    code: u32,
    message: String,
}

fn typed_error() -> Result<(), Report<MyError>> {
    let error = MyError {
        code: 404,
        message: "Resource not found".to_string(),
    };

    Err(report!(error))
}

fn main() {
    if let Err(report) = typed_error() {
        println!("{report}");
    }
}
```

Note: You might also want to implement `std::fmt::Display`, as otherwise the report will print:

```
● Context of type `example::MyError`
╰ src/main.rs:19:9
```

### Other Variants

_Most users can use just `Report` with the defaults. This section explains additional type parameters if you need cloning, thread-local errors, or other specialized behavior - come back to this when you encounter those needs._

The `Report` type is generic over three parameters: `Report<Context, Ownership, ThreadSafety>`.

**For type safety**, use `Report<YourErrorType>` instead of plain `Report`. This guarantees the root error is your specific type (shown above).

**For cloning**, use `Report<dyn Any, Cloneable>`. The default `Report` can't be cloned because it allows efficient mutation of the root node. (Note: `dyn Any` is the default context type - you can use any context type with `Cloneable`.)

**For thread-local data**, use `Report<dyn Any, Mutable, Local>` to store `!Send` or `!Sync` objects like `Rc` or `Cell`. The default `Report` requires all errors to be `Send + Sync`.

You can convert between variants:

```rust
use rootcause::prelude::*;

// Make a Report cloneable
let report: Report = report!("error");
let cloneable: Report<dyn Any, Cloneable> = report.into_cloneable();

// Downcast to a specific type
let typed: Result<Report<MyError>, _> = cloneable.downcast_report();
```

See the [full API documentation](https://docs.rs/rootcause) for all variants and conversions.

## Advanced Features

Once you're comfortable with the basics, rootcause offers powerful features for complex scenarios.

### Collecting Multiple Errors

Remember the [tree structure](#core-concepts)? This is where it shines. Use `ReportCollection` to gather multiple failures and show them all as branches:

```rust
use rootcause::{prelude::*, report_collection::ReportCollection};

fn fetch_document_with_retry(url: &str, retry_count: usize) -> Result<Vec<u8>, Report> {
    let mut errors = ReportCollection::new();

    for attempt in 1..=retry_count {
        match fetch_document(url).attach_with(|| format!("Attempt #{attempt}")) {
            Ok(data) => return Ok(data),
            Err(error) => errors.push(error.into_cloneable()),
        }
    }

    Err(errors.context(format!("Unable to fetch document {url}")))?
}
```

This creates a tree structure with all retry attempts:

```
 ● Unable to fetch document http://example.com
 ├ examples/retry_with_collection.rs:59:16
 │
 ├─ ● HTTP error: 500 Internal server error
 │  ├ examples/retry_with_collection.rs:42:9
 │  ╰ Attempt #1
 │
 ╰─ ● HTTP error: 404 Not found
    ├ examples/retry_with_collection.rs:42:9
    ╰ Attempt #2
```

See [`retry_with_collection.rs`](examples/retry_with_collection.rs) and [`batch_processing.rs`](examples/batch_processing.rs) for more examples of collecting multiple errors.

## Architecture

The library consists of two main crates:

- **`rootcause`**: The main user-facing API. Contains safe abstractions and some unsafe code for type parameter handling.
- **`rootcause-internals`**: Low-level implementation details. Contains the majority of unsafe code, isolated from user-facing APIs.

This separation ensures that most unsafe operations are contained in a single, auditable crate. The public API in `rootcause` uses these primitives to provide safe, ergonomic error handling.

## Stability and Roadmap

**Current status:** Pre-1.0 (v0.6.0)

rootcause follows semantic versioning. As a 0.x library, breaking changes may occur in minor version bumps (0.x → 0.x+1). We're actively refining the API based on real-world usage and focused on reaching 1.0.

**Post-1.0:** API stability with only breaking changes in major versions.

**When to adopt:**

- ✅ **Now**: If you value the inspection capabilities and can tolerate occasional breaking changes during 0.x
- ⏳ **Wait for 1.0**: If you need long-term API stability guarantees

We're committed to reaching 1.0, but we want to get the API right first.

## Minimum Supported Rust Version (MSRV)

Our current Minimum Supported Rust Version is 1.89.0. When adding features, we will follow these guidelines:

- Our goal is to support at least five minor Rust versions. This gives you a 6 month window to upgrade your compiler.
- Any change to the MSRV will be accompanied with a minor version bump.

## Acknowledgements

This library was inspired by and draws ideas from several existing error handling libraries in the Rust ecosystem, including [`anyhow`](https://docs.rs/anyhow), [`thiserror`](https://docs.rs/thiserror), and [`error-stack`](https://docs.rs/error-stack).

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
