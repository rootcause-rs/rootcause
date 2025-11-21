//! Quick reference for bidirectional eyre interoperability.
//!
//! This example demonstrates all the ways to convert between rootcause
//! [`Report`]s and [`eyre::Report`]. For a migration guide from eyre to
//! rootcause, see `examples/anyhow_migration.rs` which covers similar concepts
//! (eyre and anyhow share very similar APIs).
//!
//! # Running this Example
//!
//! ```bash
//! cargo run --example eyre_interop --features eyre
//! ```
//!
//! # Conversion Overview
//!
//! ## From Eyre to Rootcause
//! - `.into_rootcause()` - Convert `eyre::Result<T>` or `eyre::Report`
//!
//! ## From Rootcause to Eyre
//! - `.into_eyre()` - Convert `Result<T, Report>` or `Report`
//! - `.into()` - Use `From<Report>` for automatic conversion
//! - `?` operator - Automatically converts `Report` to `eyre::Report` in eyre
//!   functions

// Import only what we need to avoid conflicting with eyre's WrapErr trait
use rootcause::{
    Report, bail,
    compat::{IntoRootcause, eyre::IntoEyre},
};

// ============================================================================
// Example 1: Calling eyre code from rootcause
// ============================================================================

fn some_eyre_function() -> eyre::Result<String> {
    eyre::bail!("connection failed");
}

fn rootcause_calls_eyre() -> Result<(), Report> {
    use rootcause::prelude::ResultExt;

    // Use .into_rootcause() to convert eyre::Result to Result<T, Report>
    // Then use rootcause's .context() to add context
    let value = some_eyre_function()
        .into_rootcause()
        .context("Failed to get value")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Example 2: Exposing rootcause code to eyre callers
// ============================================================================

fn some_rootcause_function() -> Result<String, Report> {
    bail!("validation failed");
}

fn eyre_calls_rootcause() -> eyre::Result<()> {
    use eyre::WrapErr;

    // Use .into_eyre() to convert Result<T, Report> to eyre::Result
    // Then use eyre's .wrap_err() to add context
    let value = some_rootcause_function()
        .into_eyre()
        .wrap_err("Failed to process data")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Example 3: Automatic conversion via From trait
// ============================================================================

fn rootcause_function_returning_eyre() -> eyre::Result<()> {
    // The ? operator automatically converts Report to eyre::Report
    some_rootcause_function()?;
    Ok(())
}

fn main() -> eyre::Result<()> {
    // Install eyre's default handler
    eyre::set_hook(Box::new(eyre::DefaultHandler::default_with))?;

    println!("Rootcause ↔ Eyre Interoperability Examples\n");
    println!("===========================================\n");

    println!("=== Example 1: Eyre → Rootcause ===\n");
    if let Err(e) = rootcause_calls_eyre() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 2: Rootcause → Eyre ===\n");
    if let Err(e) = eyre_calls_rootcause() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 3: Automatic Conversion with ? ===\n");
    if let Err(e) = rootcause_function_returning_eyre() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("===========================================");
    println!("\nFor more examples, see:");
    println!("- examples/anyhow_migration.rs - Migration guide (similar concepts)");
    println!("- examples/anyhow_interop.rs - anyhow integration");
    println!("- examples/error_stack_interop.rs - error-stack integration");

    Ok(())
}
