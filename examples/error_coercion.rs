//! Demonstrates automatic error coercion with the `?` operator.
//!
//! Key insight: The `?` operator automatically converts between error types:
//! - `C` → `Report<C>`
//! - `C` → `Report<dyn Any>`
//! - `Report<C>` → `Report<dyn Any>`
//!
//! This lets you freely mix error types and let `?` handle conversions.

use std::fs;

use rootcause::prelude::*;

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

// Example 1: Typed errors - Report<C> accepts both C and Report<C>

/// Functions returning Report<C> can use ? with both raw errors and reports.
fn parse_typed_example(input: &str) -> Result<u32, Report<ParseError>> {
    // Scenario 1: Raw ParseError → ? converts to Report<ParseError>
    if input.is_empty() {
        Err(ParseError::InvalidFormat)?;
    }

    // Scenario 2: Report<ParseError> → ? passes through as-is
    if !input.chars().all(|c| c.is_ascii_digit()) {
        Err(report!(ParseError::InvalidFormat).attach(format!("Input: {input}")))?;
    }

    // Scenario 3: .context() returns Report<ParseError> → ? passes through
    let value: u32 = input
        .parse()
        .context(ParseError::MissingField("value".to_string()))?;

    Ok(value)
}

// Example 2: Dynamic errors - Report<dyn Any> accepts any error type

/// Functions returning Report<dyn Any> can freely mix different error types.
fn mixed_errors_example(path: &str) -> Result<u32, Report> {
    // io::Error → ? converts to Report<dyn Any>
    let contents = fs::read_to_string(path)?;

    // Report<io::Error> → ? coerces to Report<dyn Any>
    let _metadata = fs::metadata(path).attach(format!("Path: {path}"))?;

    // Report<ParseError> → ? coerces to Report<dyn Any>
    let value = parse_typed_example(&contents)?;

    // Report<&str> → ? coerces to Report<dyn Any>
    if value == 0 {
        Err(report!("Invalid value").attach("Value must be non-zero"))?;
    }

    Ok(value)
}

// Example 3: Coercion chains

/// Multiple layers of different error types all coerce to Report<dyn Any>.
fn coercion_chain(path: &str) -> Result<String, Report> {
    // io::Error → Report<dyn Any>
    let raw_data = fs::read_to_string(path).attach(format!("Reading file: {path}"))?;

    // Report<ParseError> → Report<dyn Any>
    let number = parse_typed_example(&raw_data).context("Failed to parse configuration")?;

    // Report<String> → Report<dyn Any>
    if number > 100 {
        Err(report!("Value too large: {number}").attach("Maximum allowed value: 100"))?;
    }

    Ok(format!("Processed value: {number}"))
}

// Example 4: Practical pattern combining typed and dynamic errors

fn validate_config(value: u32) -> Result<(), Report<ParseError>> {
    if value == 0 {
        Err(report!(ParseError::MissingField("value".to_string())))?;
    }
    Ok(())
}

fn save_config(value: u32, path: &str) -> Result<(), Report> {
    fs::write(path, value.to_string()).attach(format!("Writing to: {path}"))?;
    Ok(())
}

/// Realistic pattern: validation uses typed errors, I/O uses dynamic errors.
fn process_and_save(input: &str, output: &str) -> Result<(), Report> {
    let value = parse_typed_example(input)?; // Report<ParseError> → Report<dyn Any>
    validate_config(value)?; // Report<ParseError> → Report<dyn Any>
    save_config(value, output)?; // Report<dyn Any> → Report<dyn Any>
    Ok(())
}

fn main() {
    println!("=== Error Coercion Examples ===\n");

    println!("Example 1: Report<C> accepts C and Report<C>\n");

    println!("Raw error (C):");
    if let Err(e) = parse_typed_example("") {
        println!("{e}\n");
    }

    println!("Report with attachments (Report<C>):");
    if let Err(e) = parse_typed_example("abc") {
        println!("{e}\n");
    }

    println!("Example 2: Report<dyn Any> accepts any error type");
    if let Err(e) = mixed_errors_example("/nonexistent/config.txt") {
        println!("{e}\n");
    }

    println!("Example 3: Coercion through multiple layers");
    if let Err(e) = coercion_chain("/nonexistent/data.txt") {
        println!("{e}\n");
    }

    println!("Example 4: Combining typed and dynamic errors");
    if let Err(e) = process_and_save("42", "/tmp/config.txt") {
        println!("{e}");
    } else {
        println!("Successfully processed and saved configuration");
    }
}
