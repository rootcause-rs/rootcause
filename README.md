# rootcause

A flexible, ergonomic, and inspectable error reporting library for Rust.

[![Build Status](https://github.com/rootcause-rs/rootcause/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/rootcause-rs/rootcause/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/rootcause.svg)](https://crates.io/crates/rootcause)
[![Documentation](https://docs.rs/rootcause/badge.svg)](https://docs.rs/rootcause)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rootcause-rs/rootcause#license)

<img src="https://github.com/rootcause-rs/rootcause/raw/main/rootcause.png" width="192">

## Overview

This crate provides a structured way to represent and work with errors and their context. The main goal is to enable you to build rich, structured error reports that automatically capture not just what went wrong, but also the context and supporting data at each step.

It allows printing pretty, tree-structured reports like this one:

```
 ● Unable to fetch document http://example.com
 ├ examples/readme-example.rs:45:21
 │
 ├─ ● HTTP error: 400 Bad Request: Could not parse JSON payload
 │  ├ examples/readme-example.rs:32:9
 │  ╰ Attempt #1
 │
 ╰─ ● HTTP error: 500 Internal server error
    ├ examples/readme-example.rs:32:9
    ╰ Attempt #2
```

## Project Goals

- **Ergonomic**: The `?` operator should work with most error types, even ones not designed for this library
- **Fast happy path**: A `Result<(), Report>` should never be larger than a `usize`
- **Typable**: Users should be able to (optionally) specify the type of the context in the root node
- **Inspectable**: The objects in a Report should not be glorified strings. Inspecting and interacting with them should be easy
- **Cloneable**: It should be possible to clone a `Report` when you need to
- **Mergeable**: It should be possible to merge multiple `Report`s into a single one
- **Customizable**: It should be possible to customize what data gets collected, or how reports are formatted
- **Rich**: Reports should automatically capture information (like backtraces) that might be useful in debugging
- **Beautiful**: The default formatting of a Report should look pleasant

## Core Concepts

At a high level, you can think of the library as a way to build a tree of error reports, where each node in the tree represents a step in the error's history.

You can think of a Report as roughly implementing this data structure:

```rust
struct Report<C: ?Sized> {
    root: triomphe::Arc<Node<C>>,
}

struct Node<C: ?Sized> {
    attachments: Vec<Attachment>,
    children: Vec<Report<dyn Any>>,
    context: C,
}

struct Attachment(Box<dyn Any>);
```

The actual implementation follows the same basic structure, but has a few more complications.
There are multiple reasons for this, including performance, ergonomics and being able to support
the features we want.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rootcause = "0.4.2"
```

### Basic Usage

```rust
use rootcause::prelude::*;

fn might_fail() -> Result<(), Report> {
    std::fs::read("/tmp/nonexistent")
        .attach("additional context")?;
    Ok(())
}

fn main() {
    if let Err(report) = might_fail() {
        println!("{report}");
    }
}
```

### With Custom Context Types

```rust
use rootcause::prelude::*;

// You might also want to implement `std::fmt::Display`,
// as otherwise this example will print out:
//   ● Context of type `example::MyError`
//   ╰ src/main.rs:19:9
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

## Report Variants

The `Report` type is generic over three parameters to support different use cases:

1. **Context Type**: The type of the context in the root node (default: `dyn Any`)
2. **Ownership**: The ownership and cloning behavior (default: `Mutable`)
3. **Thread Safety**: Whether the report is `Send + Sync` (default: `SendSync`)

### Context Variants

| Variant                     | Root Context      | Internal Contexts | Use Case                 |
| --------------------------- | ----------------- | ----------------- | ------------------------ |
| `Report<SomeContextType>`   | `SomeContextType` | Can be anything   | Similar to `error-stack` |
| `Report<dyn Any>` (default) | Can be anything   | Can be anything   | Similar to `anyhow`      |

### Ownership Variants

| Variant                        | `Clone` | Mutation  | Description                                           |
| ------------------------------ | ------- | --------- | ----------------------------------------------------- |
| `Report<*, Mutable>` (default) | ❌      | Root only | Root allocated with `UniqueArc`, internals with `Arc` |
| `Report<*, Cloneable>`         | ✅      | ❌        | All nodes allocated with `Arc`                        |

### Thread Safety Variants

| Variant                            | `Send + Sync` | Allows `!Send/!Sync` objects | Description                       |
| ---------------------------------- | ------------- | ---------------------------- | --------------------------------- |
| `Report<*, *, SendSync>` (default) | ✅            | ❌                           | All objects must be `Send + Sync` |
| `Report<*, *, Local>`              | ❌            | ✅                           | Allows non-thread-safe objects    |

## Converting Between Variants

You can convert between report variants using the `From` trait or specific methods:

```rust
use rootcause::prelude::*;

// Convert to more general variants (always possible)
let report: Report<MyError> = report!(MyError { /* ... */ });
let general: Report = report.into_dyn_any();
let cloneable: Report<dyn Any, Cloneable> = general.into_cloneable();

// Convert to more specific variants (conditional)
let specific: Result<Report<MyError>, _> = general.downcast_report();
let mutable: Result<Report<dyn Any, Mutable>, _> = cloneable.try_into_mutable();
```

## Features

- **`std`** (default): Enable standard library support
- **`backtrace`**: Automatic backtrace capture on error creation

## Comparison with Other Libraries

### vs. `anyhow`

- ✅ More structured and inspectable errors
- ✅ Better support for custom context types
- ✅ Richer attachment system
- ❌ Slightly more complex API

### vs. `error-stack`

- ✅ More flexible ownership models
- ✅ Better ergonomics with the `?` operator
- ✅ More customizable formatting
- ❌ Different API surface

### vs. `thiserror`

- ✅ Runtime error composition vs. compile-time
- ✅ Automatic context capture
- ✅ Rich attachment system
- ❌ More overhead for simple cases

## Architecture

The library consists of two main crates:

- **`rootcause`**: The main user-facing API
- **`rootcause-internals`**: Low-level implementation details and most of the unsafe code

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
