//! Batch processing with error collection
//!
//! Three approaches to handling errors in batch operations:
//! - Standard `.collect()` - stops at first error
//! - `.collect_reports()` - collects all errors
//! - Manual loop - allows partial success

use std::io;

use rootcause::{prelude::*, report_collection::ReportCollection};

fn process_file(filename: &str) -> Result<String, Report<io::Error>> {
    if filename.contains("bad") {
        Err(report!(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid file format",
        ))
        .attach(format!("File: {filename}")))
    } else if filename.contains("missing") {
        Err(
            report!(io::Error::new(io::ErrorKind::NotFound, "File not found"))
                .attach(format!("File: {filename}")),
        )
    } else {
        Ok(format!("Processed: {filename}"))
    }
}

// Standard Rust: stops at first error
fn process_standard_collect(files: &[&str]) -> Result<Vec<String>, Report> {
    let results = files
        .iter()
        .map(|&filename| process_file(filename))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(results)
}

// IteratorExt: processes all items, collects all errors
fn process_collect_reports(files: &[&str]) -> Result<Vec<String>, Report> {
    let results = files
        .iter()
        .map(|&filename| process_file(filename))
        .collect_reports()
        .context("Failed to process one or more files")?;
    Ok(results)
}

// Manual loop: processes all items, allows partial success
fn process_with_partial_success(files: &[&str]) -> (Vec<String>, Option<Report>) {
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
                .into_dynamic(),
        )
    } else {
        None
    };

    (successes, error)
}

fn main() {
    let files = &[
        "data.txt",
        "bad_file.dat",
        "config.json",
        "missing_file.txt",
    ];

    println!("Approach 1: Standard .collect() (stops at first error)\n");
    match process_standard_collect(files) {
        Ok(results) => println!("All succeeded: {results:?}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Approach 2: .collect_reports() (collects all errors)\n");
    match process_collect_reports(files) {
        Ok(results) => println!("All succeeded: {results:?}"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Approach 3: Manual loop (partial success)\n");
    let (successes, error) = process_with_partial_success(files);
    println!("Successes: {successes:?}");
    if let Some(error) = error {
        eprintln!("\n{error}");
    }
}
