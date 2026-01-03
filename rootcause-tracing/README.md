# rootcause-tracing

Tracing span support for the [rootcause](https://docs.rs/rootcause) error reporting library.

[![Crates.io](https://img.shields.io/crates/v/rootcause-tracing.svg)](https://crates.io/crates/rootcause-tracing)
[![Documentation](https://docs.rs/rootcause-tracing/badge.svg)](https://docs.rs/rootcause-tracing)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rootcause-rs/rootcause#license)

## Overview

This crate provides automatic tracing span capture for rootcause error reports. When an error occurs, it captures the current tracing span information, helping you understand which operation was being performed when the error occurred.

This is especially useful for:

- Identifying which instrumented function failed
- Debugging errors in async/concurrent code where stack traces are less useful
- Complementing backtraces with high-level operation context

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rootcause = "0.11.1"
rootcause-tracing = "0.11.1"
tracing = "0.1.44"
tracing-subscriber = "0.3.22"
```

**How it works:** rootcause-tracing needs two things:

1. **`RootcauseLayer`** - A tracing layer that quietly captures span field values in the background
2. **`SpanCollector`** (optional) - A rootcause hook that automatically attaches captured spans to all rootcause reports

You add `RootcauseLayer` to your tracing subscriber alongside your existing layers (formatting, filtering, log forwarding, etc.). Each layer operates independently - `RootcauseLayer` captures span data for error reports while your other layers continue their normal work.

Complete example with automatic span capture:

```rust
use rootcause::hooks::Hooks;
use rootcause_tracing::{RootcauseLayer, SpanCollector};
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn main() {
    // 1. Set up tracing with RootcauseLayer (required)
    let subscriber = Registry::default()
        .with(RootcauseLayer)  // Captures span field values for error reports
        .with(tracing_subscriber::fmt::layer());  // Regular tracing output

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set subscriber");

    // 2. Install hook to automatically capture spans for all errors
    Hooks::new()
        .report_creation_hook(SpanCollector::new())
        .install()
        .expect("failed to install hooks");

    // 3. Use your app normally - spans are captured automatically
    if let Err(e) = run_app() {
        eprintln!("{}", e);
    }
}

#[tracing::instrument]
fn run_app() -> Result<(), rootcause::Report> {
    // Your application code
    Ok(())
}
```

**Output:** Errors automatically show span context with field values:

```
 ● Failed to process request
 ├ src/main.rs:45:10
 ╰ Tracing spans
   │ process_request{user_id=42 session="abc-123"}
   ╰─
```

### Alternative: Manual Span Attachment

If you prefer to attach spans selectively, use the `SpanExt` trait:

```rust
use rootcause_tracing::SpanExt;

#[tracing::instrument]
fn operation() -> Result<(), rootcause::Report> {
    Err(rootcause::report!("failed"))
}

let result = operation().attach_span();  // Manually attach span to this error
```

**Note:** You still need `RootcauseLayer` in your subscriber setup (see above).

### Migrating from `tracing_subscriber::fmt::init()`

If you currently use `tracing_subscriber::fmt::init()`, you need to set up your subscriber manually to add `RootcauseLayer`.

**Before:**

```rust
fn main() {
    tracing_subscriber::fmt::init();  // Simple one-line setup
    // ...
}
```

**After:**

```rust
use rootcause_tracing::RootcauseLayer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn main() {
    // Set up subscriber with multiple layers
    let subscriber = Registry::default()
        .with(RootcauseLayer)  // Captures spans for error reports
        .with(tracing_subscriber::fmt::layer());  // Console output (same as before)

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set subscriber");
    // ...
}
```

**What changed:** Instead of `fmt::init()` creating a subscriber for you, you create it yourself with `Registry::default()`. This lets you add multiple layers. Add `RootcauseLayer` alongside your existing layers (formatting, filtering, etc.).

## Nested Spans

With nested instrumented functions, each error captures the full span hierarchy from the active span to the root:

```text
● Request failed
├ Tracing spans
│ │ handle_request{request_id="abc"}
│ ╰─
│
● Auth check failed
├ Tracing spans
│ │ check_auth{user_id=42}
│ │ handle_request{request_id="abc"}
│ ╰─
│
● Database error
╰ Tracing spans
  │ query_db{query="SELECT..."}
  │ check_auth{user_id=42}
  │ handle_request{request_id="abc"}
  ╰─
```

Spans are ordered innermost to outermost. See [`examples/tracing_spans.rs`](../examples/tracing_spans.rs) for a complete example.

## Configuration

### Environment Variables

Control tracing span capture behavior at runtime:

- **`ROOTCAUSE_TRACING`** - Comma-separated options:
  - `leafs` - Only capture tracing spans for leaf errors (errors without children)

Example:

```bash
# Only capture tracing spans for leaf errors
ROOTCAUSE_TRACING=leafs cargo run
```

### Programmatic Configuration

Customize when tracing spans are captured:

```rust
use rootcause::hooks::Hooks;
use rootcause_tracing::SpanCollector;

let collector = SpanCollector {
    capture_span_for_reports_with_children: false,  // Only leaf errors
};

Hooks::new()
    .report_creation_hook(collector)
    .install()
    .expect("failed to install hooks");
```

## Comparison with Backtraces

`rootcause-backtrace` and `rootcause-tracing` serve complementary purposes:

| Feature              | rootcause-backtrace                  | rootcause-tracing                              |
| -------------------- | ------------------------------------ | ---------------------------------------------- |
| **What it captures** | Stack frames / function calls        | Tracing spans / logical operations             |
| **Best for**         | Understanding call paths             | Understanding business logic flow              |
| **Information type** | Technical: file:line, function names | Semantic: operation names, contextual metadata |
| **Overhead**         | Moderate (symbol resolution)         | Low (span metadata already exists)             |

For maximum debugging power, use both together!

## Requirements

- **`RootcauseLayer`** must be added to your tracing subscriber
- A tracing subscriber must be configured (e.g., `tracing-subscriber::Registry`)
- Spans must be entered for context to be captured (use `#[tracing::instrument]` or manual span entry)

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
