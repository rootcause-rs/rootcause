//! Quick reference for error-stack interoperability
//!
//! This example demonstrates the conversion APIs between rootcause and
//! error-stack.
//!
//! # Conversion APIs
//!
//! - **`.into_rootcause()`** - Convert `error_stack::Report` to rootcause `Report`
//! - **`.into_error_stack()`** - Convert rootcause `Report` to `error_stack::Report`
//! - **`From<Report>`** - Automatic conversion to `error_stack::Report`

use rootcause::compat::error_stack::IntoErrorStack;
use rootcause::prelude::*;
use std::io;

// ============================================================================
// Calling error-stack code from rootcause
// ============================================================================

fn some_error_stack_function() -> Result<String, error_stack::Report<io::Error>> {
    Ok("Hello from error-stack".to_string())
}

fn rootcause_calls_error_stack() -> Result<(), Report> {
    // Use .into_rootcause() to convert error_stack::Report to Report
    let value = some_error_stack_function()
        .into_rootcause()
        .context("Failed to get value")?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Exposing rootcause code to error-stack callers
// ============================================================================

fn some_rootcause_function() -> Result<String, Report> {
    Ok("Hello from rootcause".to_string())
}

fn error_stack_calls_rootcause()
-> Result<(), error_stack::Report<rootcause::compat::ReportAsError<dyn core::any::Any>>> {
    // Use .into_error_stack() to convert Result<T, Report> to error_stack result
    let value = some_rootcause_function().into_error_stack()?;

    println!("Got value: {}", value);
    Ok(())
}

// ============================================================================
// Automatic conversion via From trait
// ============================================================================

fn rootcause_function_returning_error_stack()
-> Result<(), error_stack::Report<rootcause::compat::ReportAsError<dyn core::any::Any>>> {
    // Report can be automatically converted to error_stack::Report via ?
    some_rootcause_function()?;
    Ok(())
}

// ============================================================================
// Converting individual reports
// ============================================================================

fn demonstrate_report_conversion() {
    // Convert a rootcause Report to error-stack Report using .into_error_stack()
    let rootcause_report = report!("something failed").attach("Additional context");

    let es_report: error_stack::Report<_> = rootcause_report.into_error_stack();
    println!("Using .into_error_stack(): {}", es_report);

    // Convert using the From trait
    let rootcause_report = report!("another error");
    let es_report: error_stack::Report<_> = rootcause_report.into();
    println!("Using From trait: {}", es_report);

    // Convert an error-stack Report to rootcause Report
    use error_stack::IntoReport;
    let es_report = io::Error::from(io::ErrorKind::NotFound).into_report();
    let rootcause_report: Report<_> = es_report.into_rootcause();
    println!("error-stack to rootcause: {}", rootcause_report);
}

// ============================================================================
// Main - demonstrate all patterns
// ============================================================================

fn main() -> Result<(), error_stack::Report<rootcause::compat::ReportAsError<dyn core::any::Any>>> {
    println!("RootCause ↔ error-stack Interoperability\n");

    println!("1. Calling error-stack code from rootcause:");
    if let Err(e) = rootcause_calls_error_stack() {
        eprintln!("   Error: {}\n", e);
    }

    println!("2. Calling rootcause code from error-stack:");
    error_stack_calls_rootcause()?;
    println!();

    println!("3. Automatic conversion (Report → error_stack::Report):");
    rootcause_function_returning_error_stack()?;
    println!("   ✓ Conversion works seamlessly\n");

    println!("4. Converting individual reports:");
    demonstrate_report_conversion();
    println!();

    println!("✓ All conversions work seamlessly");

    Ok(())
}
