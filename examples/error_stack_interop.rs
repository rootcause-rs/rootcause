//! Quick reference for bidirectional error-stack interoperability.
//!
//! This example demonstrates all the ways to convert between rootcause
//! [`Report`]s and [`error_stack::Report`].
//!
//! # Running this Example
//!
//! ```bash
//! cargo run --example error_stack_interop --features compat-error-stack06
//! ```
//!
//! # Conversion Overview
//!
//! ## From error-stack to Rootcause
//! - `.into_rootcause()` - Convert `error_stack::Report<C>` or individual
//!   errors
//!
//! ## From Rootcause to error-stack
//! - `.into_error_stack()` - Convert `Result<T, Report>` or `Report`
//! - `.into()` - Use `From<Report>` for automatic conversion
//! - `?` operator - Automatically converts `Report` to `error_stack::Report` in
//!   error-stack functions

// Import only what we need to avoid conflicting with error-stack's attach trait
use rootcause::{
    Report, bail,
    compat::{IntoRootcause, error_stack06::IntoErrorStack},
};

// ============================================================================
// Example 1: Calling error-stack code from rootcause
// ============================================================================

#[derive(Debug)]
struct ConnectionError;

impl core::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("connection failed")
    }
}

impl core::error::Error for ConnectionError {}

fn some_error_stack_function() -> Result<String, error_stack::Report<ConnectionError>> {
    error_stack::bail!(ConnectionError);
}

fn rootcause_calls_error_stack() -> Result<(), Report> {
    use rootcause::prelude::ResultExt;

    // Use .into_rootcause() to convert error_stack::Report to Result<T, Report>
    // Then use rootcause's .context() to add context
    let value = some_error_stack_function()
        .into_rootcause()
        .context("Failed to get value")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Example 2: Exposing rootcause code to error-stack callers
// ============================================================================

fn some_rootcause_function() -> Result<String, Report> {
    bail!("validation failed");
}

fn error_stack_calls_rootcause() -> Result<(), error_stack::Report<rootcause::compat::ReportAsError>>
{
    use error_stack::ResultExt;

    // Use .into_error_stack() to convert Result<T, Report> to error_stack::Result
    // Then use error-stack's .attach() to add context
    let value = some_rootcause_function()
        .into_error_stack()
        .attach("Failed to process data")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Example 3: Automatic conversion via From trait
// ============================================================================

fn rootcause_function_returning_error_stack()
-> Result<(), error_stack::Report<rootcause::compat::ReportAsError>> {
    // The ? operator automatically converts Report to error_stack::Report
    some_rootcause_function()?;
    Ok(())
}

fn main() -> Result<(), error_stack::Report<rootcause::compat::ReportAsError>> {
    println!("Rootcause ↔ error-stack Interoperability Examples\n");
    println!("===========================================\n");

    println!("=== Example 1: error-stack → Rootcause ===\n");
    if let Err(e) = rootcause_calls_error_stack() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 2: Rootcause → error-stack ===\n");
    if let Err(e) = error_stack_calls_rootcause() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 3: Automatic Conversion with ? ===\n");
    if let Err(e) = rootcause_function_returning_error_stack() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    println!("===========================================");
    println!("\nFor more examples, see:");
    println!("- examples/anyhow_interop.rs - anyhow integration");
    println!("- examples/eyre_interop.rs - eyre integration");

    Ok(())
}
