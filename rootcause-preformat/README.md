# rootcause-preformat

Preformatting support for the [rootcause](https://docs.rs/rootcause) error reporting library.

[![Crates.io](https://img.shields.io/crates/v/rootcause-preformat.svg)](https://crates.io/crates/rootcause-preformat)
[![Documentation](https://docs.rs/rootcause-preformat/badge.svg)](https://docs.rs/rootcause-preformat)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rootcause-rs/rootcause#license)

## Overview

This crate provides extension traits that turn a rootcause `Report` into a version where every context and attachment has been formatted into a `String`. The result behaves the same way when printed, but the original types have been erased.

Preformatting is useful when you need to:

- **Regain mutability.** A preformatted report is always `Mutable`, even if the input was `Cloneable`.
- **Cross thread boundaries.** Reports containing `!Send`/`!Sync` types (e.g. errors that hold `Rc` or `Cell`) become `Send + Sync` after preformatting.
- **Freeze the rendered output.** The preformatted version always displays the same way, even if the original types or formatting hooks are no longer available.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rootcause = "0.13"
rootcause-preformat = "0.13"
```

Bring the extension traits into scope and call `.preformat()`:

```rust
use rootcause::{markers::{Mutable, SendSync}, prelude::*};
use rootcause_preformat::{PreformatReportExt, PreformattedContext};

let report: Report = report!("database connection failed");
let preformatted: Report<PreformattedContext, Mutable, SendSync> = report.preformat();

// The preformatted report displays identically to the original
assert_eq!(format!("{}", report), format!("{}", preformatted));
```

### Crossing thread boundaries

```rust
use core::cell::Cell;
use rootcause::{markers::{Local, SendSync}, prelude::*};
use rootcause_preformat::{PreformatReportExt, PreformattedContext};

// Cell is !Send and !Sync
#[derive(Debug)]
struct LocalError { counter: Cell<u32> }

impl core::fmt::Display for LocalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Local error: {}", self.counter.get())
    }
}

let local: Report<LocalError, _, Local> = report!(LocalError { counter: Cell::new(42) });

// Preformat to obtain a Send + Sync report
let send_sync: Report<PreformattedContext, _, SendSync> = local.preformat();

std::thread::spawn(move || {
    eprintln!("{send_sync}");
}).join().unwrap();
```

## Provided Extension Traits

- **`PreformatReportExt::preformat`** — preformat an entire report tree. Implemented for `Report`, `ReportRef`, and `ReportMut`.
- **`PreformatAttachmentExt::preformat`** — preformat a single attachment. Implemented for `ReportAttachment`, `ReportAttachmentRef`, and `ReportAttachmentMut`.
- **`PreformatRootExt::preformat_root`** — extract the typed root context and return a preformatted report alongside it. Useful when you need the typed value for processing and the formatted version for display.
- **`ContextTransformNestedExt::context_transform_nested`** — transform the root context while nesting the original report (preformatted) as a child. Implemented for `Report<_, Mutable, _>` and `Result<_, Report<_, Mutable, _>>`.

## API Documentation

For complete API documentation, see [docs.rs/rootcause-preformat](https://docs.rs/rootcause-preformat).

## Minimum Supported Rust Version (MSRV)

This crate's MSRV matches rootcause: **1.89.0**

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
</sub>
