//! Quick reference for bidirectional anyhow interoperability.
//!
//! This example demonstrates all the ways to convert between rootcause
//! [`Report`]s and [`anyhow::Error`]. For a complete migration guide, see
//! [`anyhow_migration.rs`](anyhow_migration.rs).
//!
//! # Running this Example
//!
//! ```bash
//! cargo run --example anyhow_interop --features anyhow
//! ```
//!
//! # Conversion Overview
//!
//! ## From Anyhow to Rootcause
//! - `.into_rootcause()` - Convert `anyhow::Result<T>` or `anyhow::Error`
//!
//! ## From Rootcause to Anyhow
//! - `.into_anyhow()` - Convert `Result<T, Report>` or `Report`
//! - `.into()` - Use `From<Report>` for automatic conversion
//! - `?` operator - Automatically converts `Report` to `anyhow::Error` in
//!   anyhow functions

// Import only what we need to avoid conflicting with anyhow's Context trait
use rootcause::{
    Report, bail,
    compat::{IntoRootcause, anyhow::IntoAnyhow},
};

// ============================================================================
// Example 1: Calling anyhow code from rootcause
// ============================================================================

fn some_anyhow_function() -> anyhow::Result<String> {
    anyhow::bail!("connection failed");
}

fn rootcause_calls_anyhow() -> Result<(), Report> {
    use rootcause::prelude::ResultExt;

    // Use .into_rootcause() to convert anyhow::Result to Result<T, Report>
    // Then use rootcause's .context() to add context
    let value = some_anyhow_function()
        .into_rootcause()
        .context("Failed to get value")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Example 2: Exposing rootcause code to anyhow callers
// ============================================================================

fn some_rootcause_function() -> Result<String, Report> {
    bail!("validation failed");
}

fn anyhow_calls_rootcause() -> anyhow::Result<()> {
    use anyhow::Context;

    // Use .into_anyhow() to convert Result<T, Report> to anyhow::Result
    // Then use anyhow's .context() to add context
    let value = some_rootcause_function()
        .into_anyhow()
        .context("Failed to process data")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Example 3: Automatic conversion via From trait
// ============================================================================

fn rootcause_function_returning_anyhow() -> anyhow::Result<()> {
    // The ? operator automatically converts Report to anyhow::Error
    some_rootcause_function()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    println!("Rootcause ↔ Anyhow Interoperability Examples\n");
    println!("===========================================\n");

    println!("=== Example 1: Anyhow → Rootcause ===\n");
    if let Err(e) = rootcause_calls_anyhow() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 2: Rootcause → Anyhow ===\n");
    if let Err(e) = anyhow_calls_rootcause() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 3: Automatic Conversion with ? ===\n");
    if let Err(e) = rootcause_function_returning_anyhow() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("===========================================");
    println!("\nFor more examples, see:");
    println!("- examples/anyhow_migration.rs - Complete migration guide");

    Ok(())
}
