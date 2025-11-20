//! Quick reference for anyhow interoperability
//!
//! This example demonstrates the conversion APIs between rootcause and anyhow.
//! For a complete migration guide, see
//! [`anyhow_migration.rs`](anyhow_migration.rs).
//!
//! # Conversion APIs
//!
//! - **`.into_rootcause()`** - Convert `anyhow::Result` to rootcause `Result`
//! - **`.into_anyhow()`** - Convert rootcause `Result` to `anyhow::Result`
//! - **`From<Report>`** - Automatic conversion to `anyhow::Error`

use rootcause::prelude::*;

// ============================================================================
// Calling anyhow code from rootcause
// ============================================================================

fn some_anyhow_function() -> anyhow::Result<String> {
    Ok("Hello from anyhow".to_string())
}

fn rootcause_calls_anyhow() -> Result<(), Report> {
    // Use .into_rootcause() to convert anyhow::Result to Result<T, Report>
    let value = some_anyhow_function()
        .into_rootcause()
        .context("Failed to get value")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Exposing rootcause code to anyhow callers
// ============================================================================

fn some_rootcause_function() -> Result<String, Report> {
    Ok("Hello from rootcause".to_string())
}

fn anyhow_calls_rootcause() -> anyhow::Result<()> {
    // Use .into_anyhow() to convert Result<T, Report> to anyhow::Result
    let value = some_rootcause_function().into_anyhow()?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Automatic conversion via From trait
// ============================================================================

fn rootcause_function_returning_anyhow() -> anyhow::Result<()> {
    // Report can be automatically converted to anyhow::Error via ?
    some_rootcause_function()?;
    Ok(())
}

// ============================================================================
// Main - demonstrate all patterns
// ============================================================================

fn main() -> anyhow::Result<()> {
    println!("RootCause ↔ Anyhow Interoperability\n");

    println!("1. Calling anyhow code from rootcause:");
    if let Err(e) = rootcause_calls_anyhow() {
        eprintln!("   Error: {}\n", e);
    }

    println!("2. Calling rootcause code from anyhow:");
    anyhow_calls_rootcause()?;
    println!();

    println!("3. Automatic conversion (Report → anyhow::Error):");
    rootcause_function_returning_anyhow()?;
    println!("   ✓ Conversion works seamlessly\n");

    println!("See anyhow_migration.rs for a complete migration guide.");

    Ok(())
}
