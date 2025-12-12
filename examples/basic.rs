//! Basic introduction to rootcause error handling.
//!
//! **Run this example:** `cargo run --example basic`
//!
//! This example demonstrates the core error handling workflow through a
//! realistic application startup scenario. Each function builds on the previous
//! one, showing how errors naturally accumulate context as they propagate up
//! the call stack.
//!
//! The concepts demonstrated:
//! 1. The `?` operator - works automatically with any error type
//! 2. `.context()` - add meaning to explain what failed
//! 3. `.attach()` - include debugging information
//! 4. Composition - everything chains together naturally
//!
//! **What's next?** You've learned the basics. Now choose your path:
//! - Need to create your own errors? → `custom_errors.rs`
//! - Want to understand typed reports? → `typed_reports.rs`
//! - Ready to see all the examples? → See `examples/README.md`

use std::fs;

use rootcause::prelude::*;

// The `?` operator automatically converts any error type to Report
fn read_file(path: &str) -> Result<String, Report> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

// Use .context() to explain what operation failed
fn load_config(path: &str) -> Result<String, Report> {
    let content = read_file(path).context("Failed to load application configuration")?;
    Ok(content)
}

// Use .attach() to add debugging data
fn load_config_with_debug_info(path: &str) -> Result<String, Report> {
    let content = load_config(path)
        .attach(format!("Config path: {path}"))
        .attach("Expected format: TOML")?;
    Ok(content)
}

// Everything composes naturally as errors propagate up
fn startup(config_path: &str, environment: &str) -> Result<(), Report> {
    let _config = load_config_with_debug_info(config_path)
        .context("Application startup failed")
        .attach(format!("Environment: {environment}"))?;

    Ok(())
}

fn main() {
    if let Err(report) = startup("/nonexistent/config.toml", "production") {
        println!("{report}");
        println!();
        println!("• Lines with ● are contexts (.context()) - what operation failed");
        println!("• Lines without ● are attachments (.attach()) - debugging data");
    }
}
