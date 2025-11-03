// Batch processing with error collection
//
// IteratorExt::collect_reports() gathers all errors from batch operations
// instead of stopping at the first failure.

use rootcause::{prelude::*, report_collection::ReportCollection};
use std::io;

// Simulates processing that might fail
fn process_file(filename: &str) -> Result<String, Report<io::Error>> {
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

// Collect all errors instead of stopping at first failure
fn process_all_files(files: &[&str]) -> Result<Vec<String>, Report> {
    files
        .iter()
        .map(|&filename| process_file(filename))
        .collect_reports() // Result<Vec<T>, ReportCollection>
        .map_err(|errors| {
            errors
                .context("Failed to process one or more files")
                .into_dyn_any()
        })
}

// Add context to each item before collecting
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

// Partial success: return what succeeded and what failed
fn process_with_partial_results(files: &[&str]) -> (Vec<String>, Option<Report>) {
    let mut successes = Vec::new();
    let mut failures = ReportCollection::new();

    for &filename in files {
        match process_file(filename) {
            Ok(result) => successes.push(result),
            Err(error) => failures.push(error.into_cloneable()),
        }
    }

    let error = if !failures.is_empty() {
        Some(
            failures
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

// Filter and process selected items
fn process_selected_files(files: &[&str]) -> Result<Vec<String>, Report> {
    files
        .iter()
        .filter(|f| !f.starts_with('_')) // Skip underscore-prefixed files
        .map(|&f| process_file(f).attach(format!("Selected file: {f}")))
        .collect_reports()
        .map_err(|errors| {
            errors
                .context("Failed to process selected files")
                .into_dyn_any()
        })
}

fn main() {
    println!("Example 1: Collect all errors\n");
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

    println!("Example 2: Add context per item\n");
    let batch = &["item1.txt", "bad_item2.txt", "item3.txt"];
    match process_batch_with_context(batch, 42) {
        Ok(results) => println!("Batch succeeded: {results:?}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Example 3: Partial success\n");
    let mixed_files = &["good1.txt", "bad_file.dat", "good2.txt", "missing_file.txt"];
    let (successes, error) = process_with_partial_results(mixed_files);
    println!("Successes: {successes:?}");
    if let Some(error) = error {
        eprintln!("\n{error}\n");
    }

    println!("Example 4: Filter before processing\n");
    let all_files = &["_skip.txt", "process.txt", "bad_file.dat", "_ignore.txt"];
    match process_selected_files(all_files) {
        Ok(results) => println!("Filtered results: {results:?}"),
        Err(error) => eprintln!("{error}"),
    }
}
