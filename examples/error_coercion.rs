//! Demonstrates automatic error coercion with the `?` operator.
//!
//! This example shows:
//! 1. Using `C` (raw errors) in functions returning `Result<T, Report<C>>`
//! 2. Using `Report<C>` in functions returning `Result<T, Report<C>>`
//! 3. Using `C`, `Report<C>`, and `Report<dyn Any>` in functions returning `Result<T, Report>`
//! 4. How the `?` operator automatically coerces between these types
//!
//! This is one of rootcause's most powerful features - you can freely mix
//! different error representations and let `?` handle the conversions.

use rootcause::prelude::*;
use std::fs;

/// Custom error type for parsing operations.
#[derive(Debug, Clone)]
enum ParseError {
    InvalidFormat,
    MissingField(String),
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid format"),
            Self::MissingField(field) => write!(f, "Missing required field: {field}"),
        }
    }
}

impl core::error::Error for ParseError {}

// ============================================================================
// Example 1: Functions returning Result<T, Report<C>>
// ============================================================================

/// Demonstrates using both `C` and `Report<C>` in a typed error function.
///
/// When returning `Result<T, Report<C>>`, the `?` operator accepts:
/// - `C` (the raw error type) - converted to `Report<C>`
/// - `Report<C>` (already a report) - passed through as-is
fn parse_typed_example(input: &str) -> Result<u32, Report<ParseError>> {
    // Scenario 1: Return raw ParseError
    if input.is_empty() {
        // ? converts ParseError to Report<ParseError>
        Err(ParseError::InvalidFormat)?;
    }

    // Scenario 2: Return Report<ParseError> with attachments
    if !input.chars().all(|c| c.is_ascii_digit()) {
        // report!() creates Report<ParseError>, ? passes it through
        Err(report!(ParseError::InvalidFormat).attach(format!("Input: {input}")))?;
    }

    // Scenario 3: Chain with .context() which returns Report<ParseError>
    let value: u32 = input
        .parse()
        .into_report() // converts to Report<ParseIntError>
        .context(ParseError::MissingField("value".to_string()))?; // creates Report<ParseError>

    Ok(value)
}

// ============================================================================
// Example 2: Functions returning Result<T, Report<dyn Any>>
// ============================================================================

/// Demonstrates mixing `C`, `Report<C>`, and `Report<dyn Any>` in a dynamic function.
///
/// When returning `Result<T, Report>` (= `Report<dyn Any>`), the `?` operator accepts:
/// - Any `C: Error` - converted to `Report<dyn Any>`
/// - `Report<C>` for any C - coerced to `Report<dyn Any>`
/// - `Report<dyn Any>` - passed through as-is
fn mixed_errors_example(path: &str) -> Result<u32, Report> {
    // Scenario 1: io::Error from stdlib
    // ? converts io::Error to Report<dyn Any>
    let contents = fs::read_to_string(path)?;

    // Scenario 2: Report<io::Error> with attachments
    // ? coerces Report<io::Error> to Report<dyn Any>
    let _metadata = fs::metadata(path)
        .into_report()
        .attach(format!("Path: {path}"))?;

    // Scenario 3: Report<ParseError> from another function
    // ? coerces Report<ParseError> to Report<dyn Any>
    let value = parse_typed_example(&contents)?;

    // Scenario 4: Report<dyn Any> with string context
    // ? passes Report<dyn Any> through as-is
    if value == 0 {
        Err(report!("Invalid value").attach("Value must be non-zero"))?;
    }

    Ok(value)
}

// ============================================================================
// Example 3: Coercion chains
// ============================================================================

/// Demonstrates how coercion works through multiple layers.
///
/// This shows that you can build complex error chains where each layer
/// might use different error types, and `?` handles all the conversions.
fn coercion_chain(path: &str) -> Result<String, Report> {
    // Layer 1: io::Error -> Report<dyn Any>
    let raw_data = fs::read_to_string(path).attach(format!("Reading file: {path}"))?;

    // Layer 2: ParseError -> Report<ParseError> -> Report<dyn Any>
    let number = parse_typed_example(&raw_data).context("Failed to parse configuration")?;

    // Layer 3: String error -> Report<String> -> Report<dyn Any>
    if number > 100 {
        return Err(
            report!(format!("Value too large: {number}")).attach("Maximum allowed value: 100")
        )?;
    }

    Ok(format!("Processed value: {number}"))
}

// ============================================================================
// Example 4: Practical patterns
// ============================================================================

/// Helper function that returns a typed error.
fn validate_config(value: u32) -> Result<(), Report<ParseError>> {
    if value == 0 {
        Err(report!(ParseError::MissingField("value".to_string())))?;
    }
    Ok(())
}

/// Helper function that returns a dynamic error.
fn save_config(value: u32, path: &str) -> Result<(), Report> {
    fs::write(path, value.to_string())
        .into_report()
        .attach(format!("Writing to: {path}"))?;
    Ok(())
}

/// Main processing function that combines both helpers.
///
/// This is a realistic pattern: validation returns typed errors
/// (for pattern matching), while I/O returns dynamic errors.
fn process_and_save(input: &str, output: &str) -> Result<(), Report> {
    // Parse with typed errors
    let value = parse_typed_example(input)?; // Report<ParseError> -> Report<dyn Any>

    // Validate with typed errors
    validate_config(value)?; // Report<ParseError> -> Report<dyn Any>

    // Save with dynamic errors
    save_config(value, output)?; // Report<dyn Any> -> Report<dyn Any>

    Ok(())
}

fn main() {
    println!("=== Error Coercion Examples ===\n");

    // Example 1: Typed errors
    println!("Example 1: Functions returning Report<C>");
    println!("Raw error:");
    if let Err(e) = parse_typed_example("") {
        println!("{e}\n");
    }

    println!("Report with attachments:");
    if let Err(e) = parse_typed_example("abc") {
        println!("{e}\n");
    }

    // Example 2: Mixed errors
    println!("Example 2: Mixing different error types");
    if let Err(e) = mixed_errors_example("/nonexistent/config.txt") {
        println!("{e}\n");
    }

    // Example 3: Coercion chains
    println!("Example 3: Error coercion through multiple layers");
    if let Err(e) = coercion_chain("/nonexistent/data.txt") {
        println!("{e}\n");
    }

    // Example 4: Practical pattern
    println!("Example 4: Combining typed and dynamic errors");
    if let Err(e) = process_and_save("42", "/tmp/config.txt") {
        println!("{e}");
    } else {
        println!("Successfully processed and saved configuration");
    }
}
