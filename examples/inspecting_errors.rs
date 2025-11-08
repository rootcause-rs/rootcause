//! Programmatic error inspection and analysis.
//!
//! **Run this example:** `cargo run --example inspecting_errors`
//!
//! This example demonstrates how to programmatically inspect error trees to
//! extract structured data. Unlike just formatting errors for display, you can
//! traverse the tree and downcast to specific types for analytics, monitoring,
//! or intelligent error handling.
//!
//! Key concepts:
//! - `.iter_reports()` - Traverse all nodes in the error tree
//! - `.downcast_current_context::<T>()` - Check and extract typed context at
//!   each node
//! - `.attachments().iter()` - Access attachments at each node
//! - `.downcast_inner::<T>()` - Extract typed attachment data
//!
//! This shows why rootcause errors are "not glorified strings" - they contain
//! structured, inspectable objects that you can programmatically analyze.
//!
//! **What's next?**
//! - Want to collect multiple errors? → `retry_with_collection.rs`
//! - See all examples? → `examples/README.md`

use std::{error::Error, fmt};

use rootcause::prelude::*;

// ============================================================================
// Domain Types - The structured data we want to extract
// ============================================================================

/// Metadata about a retry attempt (attached to error reports)
#[derive(Debug, Clone)]
struct RetryMetadata {
    attempt: u32,
    delay_ms: u64,
    status_code: u16, // Include the status code in the attachment for easy extraction
}

impl fmt::Display for RetryMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Retry attempt {} (after {}ms, status {})",
            self.attempt, self.delay_ms, self.status_code
        )
    }
}

/// A network error with status code (used as error context)
#[derive(Debug, Clone)]
struct NetworkError {
    status_code: u16,
    message: String,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP {} - {}", self.status_code, self.message)
    }
}

impl Error for NetworkError {}

/// Database connection error (another error context type)
#[derive(Debug, Clone)]
struct DatabaseError {
    code: String,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Database error: {}", self.code)
    }
}

impl Error for DatabaseError {}

// ============================================================================
// Simulated Operations - Create errors with structured data
// ============================================================================

fn simulate_network_request(attempt: u32) -> Result<String, Report<NetworkError>> {
    let status_code = if attempt == 1 { 500 } else { 503 };
    let error = NetworkError {
        status_code,
        message: "Service temporarily unavailable".to_string(),
    };

    let delay_ms = attempt as u64 * 100;

    Err(report!(error)
        .attach(RetryMetadata {
            attempt,
            delay_ms,
            status_code,
        })
        .attach("URL: https://api.example.com/data"))
}

fn complex_operation_with_retries() -> Result<String, Report> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 1..=MAX_RETRIES {
        match simulate_network_request(attempt) {
            Ok(data) => return Ok(data),
            Err(err) => {
                if attempt == MAX_RETRIES {
                    return Err(err.context("All retry attempts exhausted").into_dyn_any());
                }
            }
        }
    }

    unreachable!()
}

// ============================================================================
// Programmatic Inspection - Extract structured data from error tree
// ============================================================================

/// Extract all retry attempts with their HTTP status codes for analytics.
///
/// This demonstrates programmatic error inspection:
/// 1. Traverse the entire error tree with `.iter_reports()`
/// 2. Downcast each node's context to check if it's a NetworkError
/// 3. Search attachments and downcast to specific types
/// 4. Extract structured data (not just strings!)
fn analyze_retry_failures(report: &Report) -> Vec<(u32, u16)> {
    let mut attempts = Vec::new();

    // Traverse all nodes in the error tree
    for node in report.iter_reports() {
        // Try to downcast the context to NetworkError
        if let Some(network_err) = node.downcast_current_context::<NetworkError>() {
            // Search through attachments for RetryMetadata
            for attachment in node.attachments().iter() {
                // Downcast the attachment to get typed access
                if let Some(retry_meta) = attachment.downcast_inner::<RetryMetadata>() {
                    // Extract the structured data
                    attempts.push((retry_meta.attempt, network_err.status_code));
                }
            }
        }
    }

    attempts
}

/// Count how many nodes in the error tree have a specific error type.
///
/// Useful for analytics: "How many NetworkErrors vs DatabaseErrors in this
/// failure?"
fn count_error_types(report: &Report) -> (usize, usize) {
    let mut network_errors = 0;
    let mut database_errors = 0;

    for node in report.iter_reports() {
        if node.downcast_current_context::<NetworkError>().is_some() {
            network_errors += 1;
        }
        if node.downcast_current_context::<DatabaseError>().is_some() {
            database_errors += 1;
        }
    }

    (network_errors, database_errors)
}

/// Find the maximum retry delay across all attempts.
///
/// Demonstrates searching attachments across the entire tree.
fn find_max_retry_delay(report: &Report) -> Option<u64> {
    let mut max_delay = None;

    for node in report.iter_reports() {
        for attachment in node.attachments().iter() {
            if let Some(retry_meta) = attachment.downcast_inner::<RetryMetadata>() {
                max_delay = Some(max_delay.unwrap_or(0).max(retry_meta.delay_ms));
            }
        }
    }

    max_delay
}

// ============================================================================
// Main - Demonstrate programmatic inspection
// ============================================================================

fn main() {
    println!("=== Programmatic Error Inspection Example ===\n");

    // Simulate an operation that fails with retries
    let result = complex_operation_with_retries();

    match result {
        Ok(_) => println!("Operation succeeded (unexpected in this example)"),
        Err(report) => {
            println!("Operation failed. Here's the formatted error:\n");
            println!("{}\n", report);

            println!("{}\n", "=".repeat(70));
            println!("Now let's inspect the error programmatically:\n");

            // Extract retry attempts and status codes
            let retry_stats = analyze_retry_failures(&report);
            println!("Retry attempts with status codes:");
            for (attempt, status) in &retry_stats {
                println!("  Attempt {}: HTTP {}", attempt, status);
            }

            // Count error types
            let (network_count, database_count) = count_error_types(&report);
            println!("\nError type breakdown:");
            println!("  NetworkError nodes: {}", network_count);
            println!("  DatabaseError nodes: {}", database_count);

            // Find maximum retry delay
            if let Some(max_delay) = find_max_retry_delay(&report) {
                println!("\nMaximum retry delay: {}ms", max_delay);
            }

            println!("\n{}\n", "=".repeat(70));
            println!("Key takeaway:");
            println!("• Errors aren't just strings - they're structured objects");
            println!("• You can traverse the tree with .iter_reports()");
            println!("• You can downcast contexts with .downcast_current_context::<T>()");
            println!("• You can downcast attachments with .downcast_inner::<T>()");
            println!("• This enables analytics, monitoring, and intelligent handling");
            println!("\nThis is what makes rootcause errors \"inspectable\"!");
            println!("\nNote: For type-safe access without downcasting, use Report<NetworkError>");
            println!("instead of Report<dyn Any>. See typed_reports.rs for examples.");
        }
    }
}
