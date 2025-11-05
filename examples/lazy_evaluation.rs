//! Lazy evaluation with `.attach_with()` and `.context_with()`.
//!
//! **Run this example:** `cargo run --example lazy_evaluation`
//!
//! This is an **optional optimization** - use it when you have performance
//! concerns. The `_with()` variants work exactly like `.context()` and
//! `.attach()`, but only compute their arguments when an error actually occurs.
//!
//! Use the `_with()` variants when:
//! - The string is expensive to compute (e.g., formatting large data
//!   structures)
//! - You need to capture values from the surrounding scope
//! - The computation should only happen if an error actually occurs
//!
//! Key concepts:
//! - `.attach_with(|| ...)` - Lazy attachments (debug info)
//! - `.context_with(|| ...)` - Lazy context (error messages)
//! - Capturing variables from the surrounding scope
//! - Performance: computation only happens on error path

use std::fs;

use rootcause::prelude::*;

// ============================================================================
// CONCEPT 1: .attach_with() - Lazy Debug Information
// ============================================================================
// Use .attach_with() when the attachment string is expensive to compute or
// needs to capture values from the surrounding scope.

/// Demonstrates lazy attachments that capture loop variables.
fn process_items(items: &[&str]) -> Result<(), Report> {
    for (index, item) in items.iter().enumerate() {
        validate_item(item)
            // attach_with() captures 'index' and 'item' from the loop
            // The closure only runs if validate_item returns Err
            .attach_with(|| format!("Processing item {index}: {item}"))
            .context("Batch processing failed")?;
    }
    Ok(())
}

fn validate_item(item: &str) -> Result<(), Report> {
    if item.is_empty() {
        return Err(report!("Item cannot be empty"));
    }
    if item.len() > 100 {
        return Err(report!("Item too long")
            .attach(format!("Length: {} characters", item.len()))
            .attach("Maximum allowed: 100 characters"));
    }
    Ok(())
}

// ============================================================================
// CONCEPT 2: .context_with() - Lazy Context Messages
// ============================================================================
// Use .context_with() when the context message itself is expensive to generate.
// Common when you need to format complex state or call functions.

/// Simulates an expensive operation that should only run on error.
fn get_diagnostic_info() -> String {
    // In a real app, this might query databases, format large structures, etc.
    format!("System state: [diagnostic data would be expensive to compute]")
}

/// Demonstrates lazy context with expensive computation.
fn process_with_diagnostics(should_fail: bool) -> Result<(), Report> {
    if should_fail {
        // context_with() means get_diagnostic_info() ONLY runs if we hit this error
        // path
        return Err(report!("Operation failed"))
            .context_with(|| format!("Failed to process. {}", get_diagnostic_info()))
            .map_err(|e| e.into());
    }
    Ok(())
}

// ============================================================================
// CONCEPT 3: Combining Both - Lazy Context and Attachments
// ============================================================================
// You can use both together for maximum flexibility.

/// Reads and parses a config file with lazy evaluation for both context and
/// attachments.
fn parse_config_value(path: &str) -> Result<i32, Report> {
    // Read the file
    let contents =
        fs::read_to_string(path).attach_with(|| format!("Reading config file: {path}"))?;

    // Parse as number
    let value: i32 = contents
        .trim()
        .parse()
        .context_with(|| {
            // This expensive message only formats if parsing fails
            format!("Failed to parse configuration value as integer")
        })
        .attach_with(|| {
            // Capture 'contents' from surrounding scope
            // Only evaluated if parsing fails
            format!("File contents: {:?}", contents.trim())
        })?;

    Ok(value)
}

fn main() {
    println!("=== Lazy Evaluation Tutorial ===\n");
    println!("When to use _with() variants vs regular methods:\n");

    println!("=== Example 1: .attach_with() - Capturing Loop Variables ===\n");
    let items = vec!["valid", "", "also valid", "x".repeat(150).leak()];
    match process_items(&items) {
        Ok(()) => println!("All items processed successfully"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("{}\n", "=".repeat(70));
    println!("=== Example 2: .context_with() - Expensive Diagnostic Info ===\n");
    match process_with_diagnostics(true) {
        Ok(()) => println!("Operation succeeded"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("{}\n", "=".repeat(70));
    println!("=== Example 3: Combined - Parsing Config with Lazy Context and Attachments ===\n");
    match parse_config_value("/nonexistent/config.txt") {
        Ok(value) => println!("Config value: {value}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("{}\n", "=".repeat(70));
    println!(
        "When NOT to use lazy evaluation:\n\
         \n\
         ❌ DON'T do this for simple strings:\n\
         \n\
            Err(report!(\"Invalid value\"))\n\
                .context_with(|| \"Value must be positive\")  // Unnecessary closure!\n\
         \n\
         ✅ DO this instead:\n\
         \n\
            Err(report!(\"Invalid value\"))\n\
                .context(\"Value must be positive\")          // Simple and clear\n\
         \n\
         The _with() variants are for EXPENSIVE operations like:\n\
         • Formatting large data structures (Vec, HashMap, etc.)\n\
         • Calling functions that do work (database queries, file I/O)\n\
         • Capturing many variables from the surrounding scope\n\
         \n\
         For simple string literals or cheap format!() calls, use the regular methods.\n\
         \n{}\n",
        "=".repeat(70)
    );
    println!(
        "Key takeaways:\n\
         \n\
         • Use .attach_with() when the attachment is expensive or captures variables\n\
         • Use .context_with() when the context message itself is costly to generate\n\
         • Use regular .attach() and .context() for simple, cheap strings\n\
         • Lazy evaluation only happens on the error path - zero cost on success\n\
         \n\
         Next steps:\n\
         • See typed_reports.rs to learn about preserving custom error types\n\
         • See custom_attachments.rs for structured attachments you can query\n"
    );
}
