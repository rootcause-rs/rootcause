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

Here's a simple example (from [`examples/basic.rs`](examples/basic.rs)) showing how errors build up context as they propagate through your call stack:

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

## Core Concepts

At a high level, rootcause helps you build a tree of error reports. Each node in the tree represents a step in the error's history - you start with a root error, then add context and attachments as it propagates up through your code.

Most error reports are linear chains (just like anyhow), but the tree structure lets you collect multiple related errors when needed.

## Project Goals

- **Ergonomic**: The `?` operator should work with most error types, even ones not designed for this library
- **Multi-failure tracking**: When operations fail multiple times (retry attempts, batch processing, parallel execution), all failures should be captured and preserved in a single report
- **Inspectable**: The objects in a Report should not be glorified strings. Inspecting and interacting with them should be easy
- **Optionally typed**: Users should be able to (optionally) specify the type of the context in the root node
- **Beautiful**: The default formatting should look pleasant—and if it doesn't match your style, the hook system lets you customize it
- **Cloneable**: It should be possible to clone a `Report` when you need to
- **Self-documenting**: Reports should automatically capture information (like backtraces and locations) that might be useful in debugging
- **Customizable**: It should be possible to customize what data gets collected, or how reports are formatted
- **Lightweight**: `Report` has a pointer-sized representation, keeping `Result<T, Report>` small and fast

## Why rootcause?

### Collecting Multiple Errors

When operations fail multiple times (retries, batch processing, parallel tasks), rootcause lets you gather all the failures into a single tree structure with readable output (from [`examples/retry_with_collection.rs`](examples/retry_with_collection.rs)):

```rust
use rootcause::{prelude::*, report_collection::ReportCollection};

fn fetch_document_with_retry(url: &str, retry_count: usize) -> Result<Vec<u8>, Report> {
    let mut errors = ReportCollection::new();

    for attempt in 1..=retry_count {
        match fetch_document(url).attach_with(|| format!("Attempt #{attempt}")) {
            Ok(data) => return Ok(data),
            // Make error cloneable so we can store it (see "Supporting..." section below)
            Err(error) => errors.push(error.into_cloneable()),
        }
    }

    Err(errors.context(format!("Unable to fetch document {url}")))?
}
```

The output from the above function will be a tree with data associated to each node:

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

For more tree examples, see [`retry_with_collection.rs`](examples/retry_with_collection.rs) and [`batch_processing.rs`](examples/batch_processing.rs).

### Inspecting Error Trees

Errors aren't just formatted strings—they're structured objects you can traverse and analyze programmatically. This enables analytics, custom error handling, and automated debugging (from [`examples/inspecting_errors.rs`](examples/inspecting_errors.rs)):

```rust
use rootcause::prelude::*;

// Analyze retry failures to extract structured data
fn analyze_retry_failures(report: &Report) -> Vec<(u32, u16)> {
    let mut attempts = Vec::new();

    // Traverse all nodes in the error tree
    for node in report.iter_reports() {
        // Try to downcast the context to NetworkError
        if let Some(network_err) = node.downcast_current_context::<NetworkError>() {
            // Search through attachments for RetryMetadata
            for attachment in node.attachments().iter() {
                if let Some(retry_meta) = attachment.downcast_inner::<RetryMetadata>() {
                    attempts.push((retry_meta.attempt, network_err.status_code));
                }
            }
        }
    }

    attempts
}
```

This lets you extract retry statistics, categorize errors, or build custom monitoring—not just display them. See [`inspecting_errors.rs`](examples/inspecting_errors.rs) for complete examples.

### Supporting Advanced Use Cases

When you need to use the same error in multiple places—like sending to a logging backend and displaying to a user, potentially on different threads—you can make errors cloneable:

```rust
use rootcause::prelude::*;

// Make the error cloneable so we can use it multiple times
let error = fetch_data().unwrap_err().into_cloneable();

// Send to background logging service
let log_error = error.clone();
tokio::spawn(async move {
    log_to_backend(log_error).await;
});

// Also display to user
display_error(error);
```

For the niche case where you're working with `!Send` errors from other libraries or need to attach thread-local data:

```rust
// Include thread-local data in error reports
let report: Report<_, _, Local> = report!(MyError)
    .attach(Rc::new(expensive_data));
```

Most code uses the defaults, but these type parameters are available when you need them. See [Report Type Parameters](https://docs.rs/rootcause/latest/rootcause/#report-type-parameters) for details.

### Type-Safe Error Handling

Libraries often need to preserve specific error types so callers can handle errors programmatically. Use `Report<YourError>` to enable pattern matching without runtime type checks (simplified from [`examples/typed_reports.rs`](examples/typed_reports.rs)):

```rust
use rootcause::prelude::*;

// Library function returns typed error
fn query_database(id: u32) -> Result<Data, Report<DatabaseError>> {
    // ... can fail with specific error types
}

// Caller can pattern match to handle errors intelligently
fn process_with_retry(id: u32) -> Result<Data, Report> {
    match query_database(id) {
        Ok(data) => Ok(data),
        Err(report) => {
            // Pattern match on the typed context
            match report.current_context() {
                DatabaseError::ConnectionLost | DatabaseError::QueryTimeout => {
                    // Retry transient errors
                    query_database(id).map_err(|e| e.into_dynamic())
                }
                DatabaseError::ConstraintViolation { .. } => {
                    // Don't retry permanent errors
                    Err(report.into_dynamic())
                }
            }
        }
    }
}
```

See [`typed_reports.rs`](examples/typed_reports.rs) for a complete example with retry logic.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rootcause = "0.11.0"
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

## Ecosystem

rootcause is designed to be lightweight and extensible. The core library provides essential error handling functionality, while optional companion crates add specialized capabilities:

- **[`rootcause-backtrace`](https://docs.rs/rootcause-backtrace)** - Automatic stack trace capture for debugging. Install hooks to attach backtraces to all errors, or use the extension trait to add them selectively.
- **[`rootcause-tracing`](https://docs.rs/rootcause-tracing)** - Tracing span capture for error reports. Automatically capture and display the active tracing spans when errors occur, providing operation context especially useful in async code.

## Next Steps

- **New to rootcause?** See [`examples/basic.rs`](examples/basic.rs) for a hands-on introduction
- **More examples:** Browse the [`examples/`](examples/) directory for common patterns
- **Full API documentation:** [docs.rs/rootcause](https://docs.rs/rootcause)

## Features

- **`std`** (default): Enable standard library support

## Coming from other libraries?

| Feature                    | anyhow              | error-stack                 | rootcause                     |
| -------------------------- | ------------------- | --------------------------- | ----------------------------- |
| **Error structure**        | Linear chain        | Opaque attachment model     | Explicit tree                 |
| **Type safety**            | No                  | Required                    | Optional                      |
| **Adding context**         | `.context()`        | `.change_context()`         | `.context()`                  |
| **Structured attachments** | No                  | Yes (`.attach_printable()`) | Yes (`.attach()`)             |
| **Tree navigation**        | Linear (`.chain()`) | Limited iterators           | Full tree access              |
| **Cloneable errors**       | No                  | No                          | Yes (`Report<_, Cloneable>`)  |
| **Thread-local errors**    | No                  | No                          | Yes (`Report<_, _, Local>`)   |
| **Location tracking**      | Single backtrace    | ?                           | Multiple backtraces supported |
| **Customization hooks**    | No                  | Formatting only             | Creation + formatting         |

See the retry example in ["Why rootcause?"](#why-rootcause) for how the tree structure enables collecting multiple related errors with `ReportCollection`.

## Advanced Features

Once you're comfortable with the basics, rootcause offers powerful features for complex scenarios. See the [examples directory](examples/) for patterns including:

- [`retry_with_collection.rs`](examples/retry_with_collection.rs) - Collecting multiple retry attempts
- [`batch_processing.rs`](examples/batch_processing.rs) - Gathering errors from parallel operations
- [`inspecting_errors.rs`](examples/inspecting_errors.rs) - Programmatic tree traversal and data extraction for analytics
- [`custom_handler.rs`](examples/custom_handler.rs) - Customizing error formatting and data collection
- [`formatting_hooks.rs`](examples/formatting_hooks.rs) - Advanced formatting customization

## Architecture

The rootcause ecosystem consists of multiple crates:

**Core:**
- **`rootcause`** - The main user-facing API with type-safe abstractions. Uses type markers to ensure the underlying data structures are used safely.
- **`rootcause-internals`** - Low-level data structures and memory management. Handles the actual report storage, allocations, and pointer manipulation.

**Extensions:**
- **`rootcause-backtrace`** - Optional backtrace capture support. Provides hooks for automatic stack trace collection.
- **`rootcause-tracing`** - Optional tracing span capture. Provides hooks to attach active tracing spans to error reports.

The split between `rootcause` and `rootcause-internals` provides a clean API boundary: internals define how data is stored, while the main crate ensures that storage is accessed safely through Rust's type system. This makes it easy to understand the underlying representation while keeping the safe API ergonomic. Extensions integrate via the hook system without requiring changes to core.

## Stability and Roadmap

**Current status:** Pre-1.0 (v0.11.0)

rootcause follows semantic versioning. As a 0.x library, breaking changes may occur in minor version bumps (0.x → 0.x+1). We're actively refining the API based on real-world usage and focused on reaching 1.0.

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
