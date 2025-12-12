//! Automatic error type conversions with `?`.
//!
//! **Run this example:** `cargo run --example error_coercion`
//!
//! The `?` operator automatically converts between error types:
//! - `C` → `Report<C>`
//! - `C` → `Report<Dynamic>`
//! - `Report<C>` → `Report<Dynamic>`
//!
//! This lets you freely mix error types without manual conversions.
//!
//! **What's next?**
//! - See all examples? → `examples/README.md`

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

// Report<ParseError> accepts both ParseError and Report<ParseError>
fn parse_value(input: &str) -> Result<u32, Report<ParseError>> {
    if input.is_empty() {
        // Raw ParseError → ? converts to Report<ParseError>
        Err(ParseError::InvalidFormat)?;
    }

    if !input.chars().all(|c| c.is_ascii_digit()) {
        // Report<ParseError> → ? passes through
        Err(report!(ParseError::InvalidFormat).attach(format!("Input: {input}")))?;
    }

    let value: u32 = input
        .parse()
        .context(ParseError::MissingField("value".to_string()))?;

    Ok(value)
}

// Report (dynamic) accepts any error type
fn process_file(path: &str) -> Result<u32, Report> {
    // io::Error → Report
    let contents = fs::read_to_string(path).attach(format!("Path: {path}"))?;

    // Report<ParseError> → Report
    let value = parse_value(&contents)?;

    Ok(value)
}

fn main() {
    println!("Typed errors - Report<ParseError>:\n");
    if let Err(report) = parse_value("") {
        eprintln!("{report}\n");
    }

    if let Err(report) = parse_value("abc") {
        eprintln!("{report}\n");
    }

    println!("Dynamic errors - mixing types:\n");
    if let Err(report) = process_file("/nonexistent/config.txt") {
        eprintln!("{report}");
    }
}
