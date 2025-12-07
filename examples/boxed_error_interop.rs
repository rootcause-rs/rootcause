//! Quick reference for bidirectional boxed error interoperability.
//!
//! This example demonstrates all the ways to convert between rootcause
//! [`Report`]s and boxed error trait objects (`Box<dyn Error>`). This is
//! useful for integrating with APIs that expect standard Rust error types.
//!
//! # Running this Example
//!
//! ```bash
//! cargo run --example boxed_error_interop
//! ```
//!
//! # Conversion Overview
//!
//! ## From Boxed Errors to Rootcause
//! - `.into_rootcause()` - Convert `Result<T, Box<dyn Error>>` or `Box<dyn
//!   Error>`
//!
//! ## From Rootcause to Boxed Errors
//! - `.into_boxed_error()` - Convert `Result<T, Report>` or `Report`
//! - `.into()` - Use `From<Report>` for automatic conversion
//! - `?` operator - Automatically converts `Report` to `Box<dyn Error>` in
//!   functions returning boxed errors
//!
//! # Thread Safety
//! - `SendSync` reports → `Box<dyn Error + Send + Sync>`
//! - `Local` reports → `Box<dyn Error>`

use std::{error::Error, rc::Rc};

use rootcause::{
    Report, bail,
    compat::{IntoRootcause, boxed_error::IntoBoxedError},
    markers::{Dynamic, Local, SendSync},
    prelude::*,
};

// ============================================================================
// Example 1: Calling boxed error code from rootcause
// ============================================================================

fn some_boxed_error_function() -> Result<String, Box<dyn Error + Send + Sync>> {
    Err("connection timeout".into())
}

fn another_boxed_error_function() -> Result<i32, Box<dyn Error>> {
    Err("parsing failed".into())
}

fn rootcause_calls_boxed_error() -> Result<(), Report> {
    // Use .into_rootcause() to convert boxed error Result to Result<T, Report>
    // Then use rootcause's .context() to add context
    let _value = some_boxed_error_function()
        .into_rootcause()
        .context("Failed to get network data")?;

    println!("Got network value");
    Ok(())
}

fn rootcause_calls_local_boxed_error() -> Result<(), Box<dyn Error>> {
    // Convert local boxed error to rootcause, then back to boxed error for example
    let _value = another_boxed_error_function()
        .into_rootcause()
        .into_boxed_error()
        .map_err(|e| format!("Failed to parse local data: {}", e))?;

    println!("Got local value");
    Ok(())
}

// ============================================================================
// Example 2: Exposing rootcause code to boxed error callers
// ============================================================================

fn some_rootcause_function() -> Result<String, Report> {
    bail!("validation failed");
}

fn some_local_rootcause_function() -> Result<String, Report<Dynamic, markers::Mutable, Local>> {
    let local_data = Rc::new("sensitive data");
    let report = report!("local validation failed")
        .into_local()
        .attach(local_data);
    Err(report)
}

fn boxed_error_calls_rootcause() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Use .into_boxed_error() to convert Result<T, Report> to
    // Result<T, Box<dyn Error + Send + Sync>>
    let _value = some_rootcause_function().into_boxed_error()?;

    println!("Got rootcause value");
    Ok(())
}

fn local_boxed_error_calls_rootcause() -> Result<(), Box<dyn Error>> {
    // Convert local report to local boxed error
    let _value = some_local_rootcause_function().into_boxed_error()?;

    println!("Got local rootcause value");
    Ok(())
}

// ============================================================================
// Example 3: Automatic conversion via From trait
// ============================================================================

fn rootcause_function_returning_boxed_error() -> Result<(), Box<dyn Error + Send + Sync>> {
    // The ? operator automatically converts Report to Box<dyn Error + Send + Sync>
    some_rootcause_function()?;
    Ok(())
}

fn rootcause_function_returning_local_boxed_error() -> Result<(), Box<dyn Error>> {
    // The ? operator automatically converts local Report to Box<dyn Error>
    some_local_rootcause_function()?;
    Ok(())
}

// ============================================================================
// Example 4: Working with different thread safety levels
// ============================================================================

fn demonstrate_thread_safety() {
    println!("=== Thread Safety Demonstration ===\n");

    // SendSync report becomes Box<dyn Error + Send + Sync>
    let send_sync_report: Report<_, _, SendSync> = report!("network error");
    let send_sync_boxed: Box<dyn Error + Send + Sync> = send_sync_report.into_boxed_error();
    println!("SendSync report → Box<dyn Error + Send + Sync>");
    println!("Error: {}\n", send_sync_boxed);

    // Local report becomes Box<dyn Error>
    let local_report: Report<_, _, Local> = report!("local error")
        .into_local()
        .attach(Rc::new("local data"));
    let local_boxed: Box<dyn Error> = local_report.into_boxed_error();
    println!("Local report → Box<dyn Error>");
    println!("Error: {}\n", local_boxed);
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Rootcause ↔ Boxed Error Interoperability Examples\n");
    println!("===============================================\n");

    println!("=== Example 1: Boxed Error → Rootcause ===\n");
    if let Err(e) = rootcause_calls_boxed_error() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    if let Err(e) = rootcause_calls_local_boxed_error() {
        println!("Local error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 2: Rootcause → Boxed Error ===\n");
    if let Err(e) = boxed_error_calls_rootcause() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    if let Err(e) = local_boxed_error_calls_rootcause() {
        println!("Local error occurred:");
        println!("{}\n", e);
    }

    println!("=== Example 3: Automatic Conversion with ? ===\n");
    if let Err(e) = rootcause_function_returning_boxed_error() {
        println!("Error occurred:");
        println!("{}\n", e);
    }

    if let Err(e) = rootcause_function_returning_local_boxed_error() {
        println!("Local error occurred:");
        println!("{}\n", e);
    }

    demonstrate_thread_safety();

    println!("===============================================");
    println!("\nKey Benefits:");
    println!("- Seamless integration with standard Rust error handling");
    println!("- Preserves thread safety constraints (SendSync vs Local)");
    println!("- Works with existing APIs expecting Box<dyn Error>");
    println!("- Automatic conversion via From trait and ? operator");

    Ok(())
}
