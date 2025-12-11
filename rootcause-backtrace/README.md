# rootcause-backtrace

Stack backtrace support for the [rootcause](https://docs.rs/rootcause) error reporting library.

[![Crates.io](https://img.shields.io/crates/v/rootcause-backtrace.svg)](https://crates.io/crates/rootcause-backtrace)
[![Documentation](https://docs.rs/rootcause-backtrace/badge.svg)](https://docs.rs/rootcause-backtrace)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rootcause-rs/rootcause#license)

## Overview

This crate provides automatic stack trace capture for rootcause error reports. Backtraces help you see the call stack that led to an error, making debugging much easier.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rootcause = "0.11"
rootcause-backtrace = "0.11"
```

### Automatic Capture (All Errors)

Install a hook to automatically attach backtraces to every error:

```rust
use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;

fn main() {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

    // Now all errors automatically include backtraces
    if let Err(e) = run_app() {
        eprintln!("{}", e);
    }
}

fn run_app() -> Result<(), rootcause::Report> {
    // Your application code
    # Ok(())
}
```

### Manual Attachment (Specific Errors)

Use the extension trait to attach backtraces selectively:

```rust
use rootcause::{Report, report};
use rootcause_backtrace::BacktraceExt;

fn risky_operation() -> Result<(), Report> {
    Err(report!("operation failed"))
}

// Attach backtrace only to this specific error
let result = risky_operation().attach_backtrace();
```

## Output Example

When an error with a backtrace is printed:

```
 ● Failed to process request
 ├ src/main.rs:45:10
 ├ Backtrace
 │ │ process_request - src/main.rs:45
 │ │ handle_connection - src/main.rs:32
 │ │ main             - src/main.rs:18
 │ │ note: 15 frame(s) omitted. For a complete backtrace, set RUST_BACKTRACE=full.
 │ ╰─
 │
 ● Database connection lost
 ╰ src/db.rs:89:5
```

## Environment Variables

Control backtrace behavior at runtime:

- **`RUST_BACKTRACE=full`** - Show all frames with full file paths (no filtering)
- **`ROOTCAUSE_BACKTRACE`** - Comma-separated options:
  - `leafs` - Only capture backtraces for leaf errors (errors without children)
  - `full_paths` - Show full file paths instead of shortened versions

Examples:

```bash
# Show complete backtraces with all frames
RUST_BACKTRACE=full cargo run

# Only capture backtraces for leaf errors
ROOTCAUSE_BACKTRACE=leafs cargo run

# Show full file paths
ROOTCAUSE_BACKTRACE=full_paths cargo run
```

## Filtering Frames

Customize which frames appear in backtraces:

```rust
use rootcause_backtrace::{BacktraceCollector, BacktraceFilter};

let collector = BacktraceCollector {
    filter: BacktraceFilter {
        // Skip these crates at the start of the backtrace
        skipped_initial_crates: &["rootcause", "rootcause-backtrace"],
        // Skip these crates in the middle
        skipped_middle_crates: &["tokio", "hyper"],
        // Skip these crates at the end
        skipped_final_crates: &["std"],
        // Limit to 15 frames
        max_entry_count: 15,
        // Show shortened paths (e.g., "src/main.rs" instead of "/home/user/project/src/main.rs")
        show_full_path: false,
    },
    // Only capture backtraces for leaf errors (errors without children)
    capture_backtrace_for_reports_with_children: false,
};
```

## Release Builds

To get useful backtraces in release builds, enable debug symbols in your `Cargo.toml`:

```toml
[profile.release]
strip = false
debug = true  # or "line-tables-only" for smaller binaries
```

## Path Privacy

By default, backtraces show shortened paths for better readability, but they may still expose your filesystem structure. If this is a concern, use rustc's path remapping:

```bash
# Set when building for release
export RUSTFLAGS="--remap-path-prefix=$HOME=/home/user --remap-path-prefix=$PWD=/build"
cargo build --release
```

This remaps absolute paths to generic placeholders.

## Features

- **Smart filtering** - Automatically hides noisy runtime frames while showing your code
- **Readable output** - Shortened paths and formatted display
- **Configurable** - Control capture behavior and filtering per your needs
- **Environment-aware** - Respects `RUST_BACKTRACE` and custom options
- **Opt-in by design** - Only applications that install the hooks pay the capture cost

## API Documentation

For complete API documentation, see [docs.rs/rootcause-backtrace](https://docs.rs/rootcause-backtrace).

## Minimum Supported Rust Version (MSRV)

This crate's MSRV matches rootcause: **1.89.0**

## License

<sup>
Licensed under either of <a href="../LICENSE-APACHE">Apache License, Version 2.0</a> or <a href="../LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
</sub>
