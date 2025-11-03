//! Basic introduction to rootcause error handling.
//!
//! This example demonstrates the fundamental concepts:
//! 1. Creating errors with `report!()`
//! 2. Adding context with `.context()`
//! 3. Attaching extra information with `.attach()`
//! 4. Building error chains through function calls

use rootcause::prelude::*;
use std::fs;

/// Attempts to read a configuration file (simple version).
///
/// Demonstrates direct coercion of `io::Error` to `Report` using just `?`.
/// This works because `io::Error` implements `std::error::Error`, and there's a
/// `From<C>` impl for any `C: Error` that converts to `Report<dyn Any>`.
fn read_config_file_simple(path: &str) -> Result<String, Report> {
    // No .attach() or .context() needed - just let ? coerce io::Error to Report
    let data = fs::read_to_string(path)?;
    Ok(data)
}

/// Attempts to read a configuration file.
///
/// Demonstrates using `.attach()` from ResultExt to convert stdlib errors
/// to Reports while adding contextual information.
fn read_config_file(path: &str) -> Result<String, Report> {
    // .attach() calls .into_report() internally, converting Result<T, E> to Result<T, Report<E>>
    // The ? operator coerces Report<io::Error> to Report (dyn Any)
    let data = fs::read_to_string(path).attach(format!("Config path: {path}"))?;
    Ok(data)
}

/// Loads and validates user configuration.
///
/// Demonstrates using `.context()` to add a parent error to the chain.
fn load_user_config() -> Result<String, Report> {
    // .context() creates a parent error, .attach() adds info to it
    let config = read_config_file("/nonexistent/config.toml")
        .context("Failed to load user configuration")
        .attach("Expected format: TOML")?;
    Ok(config)
}

/// Application startup function.
///
/// This adds another layer of context, showing how errors propagate
/// through the call stack with each function adding relevant information.
fn startup() -> Result<(), Report> {
    let _config = load_user_config()
        .context("Application startup failed")
        .attach("Startup phase: Configuration loading")?;

    Ok(())
}

fn main() {
    println!("=== Basic Error Handling ===\n");

    // Example 1: Direct coercion with ? (no attachments)
    println!("Example 1: Direct coercion");
    if let Err(report) = read_config_file_simple("/nonexistent/config.toml") {
        println!("{report}");
    }
    println!();

    // Example 2: Demonstrate error chain with attachments
    println!("Example 2: Error chain with context");
    if let Err(report) = startup() {
        println!("{report}");
    }
}
