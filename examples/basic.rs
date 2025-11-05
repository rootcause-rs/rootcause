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
//! - Want to understand the type system? → `typed_reports.rs`
//! - Ready to see all the examples? → See `examples/README.md`

use std::fs;

use rootcause::prelude::*;

// ============================================================================
// CONCEPT 1: The `?` Operator Just Works
// ============================================================================
// You don't need to change existing error-handling code. The `?` operator
// automatically converts any error type to Report.

/// Reads a file. This is where the actual I/O error will occur.
fn read_file(path: &str) -> Result<String, Report> {
    // The `?` operator automatically converts io::Error → Report
    let content = fs::read_to_string(path)?;
    Ok(content)
}

// ============================================================================
// CONCEPT 2: Adding Context with `.context()`
// ============================================================================
// Raw errors like "No such file or directory" don't explain WHY you needed that
// file. Use `.context()` to add meaning and build a story.

/// Reads the config file and explains what it's for.
fn load_config(path: &str) -> Result<String, Report> {
    // Add context to explain what this file is and why it matters
    let content = read_file(path).context("Failed to load application configuration")?;
    Ok(content)
}

// ============================================================================
// CONCEPT 3: Attaching Debug Information with `.attach()`
// ============================================================================
// Sometimes you need more than just error messages - you need data. Use
// `.attach()` to include debugging information that appears in the error
// output.

/// Loads config and attaches debugging information.
fn load_config_with_debug_info(path: &str) -> Result<String, Report> {
    let content = load_config(path)
        .attach(format!("Config path: {path}"))
        .attach("Expected format: TOML")?;
    Ok(content)
}

// ============================================================================
// CONCEPT 4: Putting It All Together
// ============================================================================
// This shows how everything composes naturally. Each layer adds context and
// attachments, building a complete picture of what went wrong.

/// The top-level startup function that orchestrates the whole process.
fn startup(config_path: &str, environment: &str) -> Result<(), Report> {
    // Load the configuration with full context and debugging info
    let _config = load_config_with_debug_info(config_path)
        .context("Application startup failed")
        .attach(format!("Environment: {environment}"))?;

    // In a real app, you'd parse the config and initialize services
    Ok(())
}

fn main() {
    println!("=== rootcause Basic Tutorial ===\n");

    // Try to start the app with a nonexistent config file
    println!("Attempting to start application...\n");
    if let Err(report) = startup("/nonexistent/config.toml", "production") {
        println!("{report}");
    }

    println!(
        "\n{}\n\
         Notice how the error chain builds from bottom to top:\n\
         \n\
         • Root cause: \"No such file or directory\" (the original I/O error)\n\
         • Low-level context: \"Failed to load application configuration\"\n\
         • Debugging data: Config path and expected format (attached with .attach())\n\
         • High-level context: \"Application startup failed\"\n\
         • More debugging: Environment information\n\
         \n\
         Each function added its piece to tell the complete story of what went wrong.\n\
         \n\
         What's next?\n\
         • Need to create your own errors? → custom_errors.rs\n\
         • Want to understand the type system? → typed_reports.rs\n\
         • See all examples? → examples/README.md\n",
        "=".repeat(70)
    );
}
