//! Basic introduction to rootcause error handling.
//!
//! This example demonstrates the fundamental concepts:
//! 1. Creating errors with `report!()`
//! 2. Adding context with `.context()`
//! 3. Attaching extra information with `.attach()`
//! 4. Building error chains through function calls

use rootcause::prelude::*;
use std::fs;

/// Simplest usage: Types implementing `Error` automatically convert to Report.
fn read_config_file_simple(path: &str) -> Result<String, Report> {
    // The ? operator automatically converts types that implement std::error::Error
    let data = fs::read_to_string(path)?;
    Ok(data)
}

/// Adding information: Use .attach() to include debugging details.
fn read_config_file(path: &str) -> Result<String, Report> {
    // .attach() adds information that will be shown in the error output
    let data = fs::read_to_string(path).attach(format!("Config path: {path}"))?;
    Ok(data)
}

/// Building context: Use .context() to explain what you were trying to do.
///
/// Error chains let you show both the low-level cause (e.g., "file not found")
/// and the high-level operation that failed (e.g., "loading user config").
fn load_user_config() -> Result<String, Report> {
    // .context() adds a parent error that explains what this function was doing
    let config = read_config_file("/nonexistent/config.toml")
        .context("Failed to load user configuration")
        .attach("Expected format: TOML")?;
    Ok(config)
}

/// Error chains show the path through your code.
///
/// Each function adds its own context, creating a trail from the top-level
/// operation down to the root cause. This makes debugging much easier.
fn startup() -> Result<(), Report> {
    let _config = load_user_config()
        .context("Application startup failed")
        .attach("Startup phase: Configuration loading")?;

    Ok(())
}

fn main() {
    println!("=== Basic Error Handling ===\n");

    // Example 1: Just the error, no extra information
    println!("Example 1: Simple error (just using ?)");
    if let Err(report) = read_config_file_simple("/nonexistent/config.toml") {
        println!("{report}");
    }
    println!();

    // Example 2: Error chain showing the full context
    // Notice how each function adds a layer, showing the path through your code
    println!("Example 2: Error chain with context and attachments");
    println!("(Shows: startup → load_user_config → read_config_file → io::Error)\n");
    if let Err(report) = startup() {
        println!("{report}");
    }
}
