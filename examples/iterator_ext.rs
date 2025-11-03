//! Demonstrates using IteratorExt to collect errors from multiple operations.
//!
//! This example shows:
//! 1. Using collect_reports() to gather errors from an iterator
//! 2. Handling mixed success/failure scenarios
//! 3. Converting iterator results into ReportCollection
//! 4. Practical patterns for batch operations

use rootcause::{prelude::*, report_collection::ReportCollection};
use std::io;

/// Simulates processing a file that might fail.
fn process_file(filename: &str) -> Result<String, Report<io::Error>> {
    // Simulate failures for certain files
    if filename.contains("bad") {
        Err(report!(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid file format",
        ))
        .attach(format!("File: {filename}")))
    } else if filename.contains("missing") {
        Err(
            report!(io::Error::new(io::ErrorKind::NotFound, "File not found",))
                .attach(format!("File: {filename}")),
        )
    } else {
        Ok(format!("Processed: {filename}"))
    }
}

/// Example 1: Using collect_reports() to gather all errors.
///
/// This is the most common pattern - try to process all items, and if any
/// fail, collect all the errors together.
fn process_all_files(files: &[&str]) -> Result<Vec<String>, Report> {
    // Map over files, creating an iterator of Results
    // Then use collect_reports() to convert to Result<Vec<T>, ReportCollection>
    files
        .iter()
        .map(|&filename| process_file(filename))
        .collect_reports()
        .map_err(|errors| {
            errors
                .context("Failed to process one or more files")
                .into_dyn_any()
        })
}

/// Example 2: Collecting with additional context per item.
///
/// Shows how to add context to each operation before collecting.
fn process_batch_with_context(items: &[&str], batch_id: u32) -> Result<Vec<String>, Report> {
    items
        .iter()
        .enumerate()
        .map(|(index, &item)| {
            process_file(item)
                .attach(format!("Batch ID: {batch_id}"))
                .attach(format!("Item index: {index}"))
        })
        .collect_reports()
        .map_err(|errors| {
            errors
                .context(format!("Batch {batch_id} processing failed"))
                .into_dyn_any()
        })
}

/// Example 3: Partial success handling.
///
/// Sometimes you want to know what succeeded even if some items failed.
/// This uses partition to separate successes from failures, then uses
/// collect_reports on just the failures.
fn process_with_partial_results(files: &[&str]) -> (Vec<String>, Option<Report>) {
    // Process all files
    let results: Vec<_> = files
        .iter()
        .map(|&filename| process_file(filename))
        .collect();

    // Partition into successes and failures
    let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

    // Extract the values
    let successes: Vec<_> = successes.into_iter().map(Result::unwrap).collect();
    let failures: Vec<_> = failures.into_iter().map(Result::unwrap_err).collect();

    let error = if !failures.is_empty() {
        // Convert failures to cloneable reports and collect
        let mut collection = ReportCollection::new();
        for failure in failures {
            collection.push(failure.into_cloneable());
        }
        Some(
            collection
                .context(format!(
                    "Processed {}/{} files successfully",
                    successes.len(),
                    files.len()
                ))
                .into_dyn_any(),
        )
    } else {
        None
    };

    (successes, error)
}

/// Example 4: Using collect_reports with filter_map.
///
/// Shows a more complex pattern where you filter and map in one step.
fn process_selected_files(files: &[&str]) -> Result<Vec<String>, Report> {
    files
        .iter()
        .filter(|f| !f.starts_with('_')) // Skip files starting with underscore
        .map(|&f| process_file(f).attach(format!("Selected file: {f}")))
        .collect_reports()
        .map_err(|errors| {
            errors
                .context("Failed to process selected files")
                .into_dyn_any()
        })
}

fn main() {
    println!("=== Example 1: Basic collect_reports() ===\n");
    let files = &[
        "data.txt",
        "bad_file.dat",
        "config.json",
        "missing_file.txt",
    ];
    match process_all_files(files) {
        Ok(results) => println!("All succeeded: {results:?}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 2: Batch processing with context ===\n");
    let batch = &["item1.txt", "bad_item2.txt", "item3.txt"];
    match process_batch_with_context(batch, 42) {
        Ok(results) => println!("Batch succeeded: {results:?}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 3: Partial success handling ===\n");
    let mixed_files = &["good1.txt", "bad_file.dat", "good2.txt", "missing_file.txt"];
    let (successes, error) = process_with_partial_results(mixed_files);
    println!("Successes: {successes:?}");
    if let Some(error) = error {
        eprintln!("\n{error}\n");
    }

    println!("=== Example 4: Filtered processing ===\n");
    let all_files = &["_skip.txt", "process.txt", "bad_file.dat", "_ignore.txt"];
    match process_selected_files(all_files) {
        Ok(results) => println!("Filtered results: {results:?}"),
        Err(error) => eprintln!("{error}"),
    }
}
