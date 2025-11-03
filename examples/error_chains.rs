//! Demonstrates various patterns for building error chains.
//!
//! Key concepts:
//! - Chaining operations with different error types
//! - `.attach_with()` for lazy evaluation (only computed on error)
//! - Building multi-level context with `.attach()` and `.context()`

use std::{fs, io};

use rootcause::prelude::*;

/// Reads a file and parses its contents as a number.
///
/// This demonstrates chaining multiple fallible operations where each
/// can fail with different error types.
fn parse_config_value(path: &str) -> Result<i32, Report> {
    // First operation: read the file (can fail with io::Error)
    let contents = fs::read_to_string(path).attach(format!("Reading config file: {path}"))?;

    // Second operation: parse as number (can fail with ParseIntError)
    let value: i32 = contents
        .trim()
        .parse()
        .context("Failed to parse configuration value")
        .attach_with(|| format!("File contents: {:?}", contents.trim()))?;

    Ok(value)
}

/// Demonstrates using `.attach_with()` for lazy evaluation.
///
/// This is useful when the attachment string is expensive to compute
/// or when you want to capture values from the surrounding scope.
fn process_items(items: &[&str]) -> Result<(), Report> {
    for (index, item) in items.iter().enumerate() {
        validate_item(item)
            // attach_with() only evaluates the closure if an error occurs
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

/// Shows how to add multiple pieces of context at different levels.
fn open_and_read(path: &str) -> Result<String, Report> {
    Ok(fs::File::open(path)
        .attach("Operation: Opening file")
        .attach_with(|| format!("Path: {path}"))
        .and_then(|mut file| {
            let mut contents = String::new();
            use io::Read;
            file.read_to_string(&mut contents)
                .attach("Operation: Reading file contents")
                .map(|_| contents)
        })
        .context("Failed to read configuration file")?)
}

fn main() {
    println!("=== Example 1: Chained operations with different error types ===\n");
    match parse_config_value("/tmp/nonexistent_config.txt") {
        Ok(value) => println!("Config value: {value}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 2: Lazy attachments with attach_with() ===\n");
    let items = vec!["valid", "", "also valid", "x".repeat(150).leak()];
    match process_items(&items) {
        Ok(()) => println!("All items processed successfully"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 3: Multiple context layers ===\n");
    match open_and_read("/etc/nonexistent") {
        Ok(contents) => println!("File contents: {contents}"),
        Err(error) => eprintln!("{error}"),
    }
}
