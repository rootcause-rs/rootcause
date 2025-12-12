//! Programmatic error inspection and analysis.
//!
//! **Run this example:** `cargo run --example inspecting_errors`
//!
//! Traverse error trees and extract structured data for analytics and monitoring:
//! - `.iter_reports()` - traverse all nodes
//! - `.downcast_current_context::<T>()` - extract typed context
//! - `.downcast_inner::<T>()` - extract typed attachments
//!
//! **What's next?**
//! - See all examples? → `examples/README.md`

use rootcause::{prelude::*, report_collection::ReportCollection};

#[derive(Debug, Clone)]
struct HttpError {
    code: u16,
    message: &'static str,
}

impl core::fmt::Display for HttpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HTTP {}: {}", self.code, self.message)
    }
}

impl core::error::Error for HttpError {}

#[derive(Debug, Clone)]
struct RetryMetadata {
    attempt: u32,
    delay_ms: u64,
}

impl core::fmt::Display for RetryMetadata {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Attempt {} (waited {}ms)", self.attempt, self.delay_ms)
    }
}

// Simulates network requests that fail with different HTTP codes
fn fetch_data(attempt: u32) -> Result<String, Report<HttpError>> {
    let error = match attempt {
        1 => HttpError {
            code: 500,
            message: "Internal Server Error",
        },
        2 => HttpError {
            code: 503,
            message: "Service Unavailable",
        },
        _ => HttpError {
            code: 504,
            message: "Gateway Timeout",
        },
    };

    Err(report!(error).attach(RetryMetadata {
        attempt,
        delay_ms: attempt as u64 * 100,
    }))
}

// Retry logic that collects all failures
fn fetch_with_retries(url: &str) -> Result<String, Report> {
    let mut errors = ReportCollection::new();

    for attempt in 1..=3 {
        match fetch_data(attempt) {
            Ok(data) => return Ok(data),
            Err(err) => errors.push(err.into_cloneable()),
        }
    }

    Err(errors.context(format!("Failed to fetch {url}")))?
}

// Extract all HTTP status codes from the error tree
fn extract_status_codes(report: &Report) -> Vec<u16> {
    report
        .iter_reports()
        .filter_map(|node| node.downcast_current_context::<HttpError>())
        .map(|err| err.code)
        .collect()
}

// Calculate total retry time from metadata attachments
fn calculate_total_retry_time(report: &Report) -> u64 {
    report
        .iter_reports()
        .flat_map(|node| node.attachments().iter())
        .filter_map(|att| att.downcast_inner::<RetryMetadata>())
        .map(|meta| meta.delay_ms)
        .sum()
}

// Combine context and attachment data from each node
fn pair_errors_with_delays(report: &Report) -> Vec<(u16, u64)> {
    report
        .iter_reports()
        .filter_map(|node| {
            let status = node.downcast_current_context::<HttpError>()?.code;
            let delay = node
                .attachments()
                .iter()
                .find_map(|att| att.downcast_inner::<RetryMetadata>())?
                .delay_ms;
            Some((status, delay))
        })
        .collect()
}

fn main() {
    if let Err(report) = fetch_with_retries("https://api.example.com/data") {
        eprintln!("Error tree:\n\n{report}\n");

        println!("Programmatic inspection:\n");

        let codes = extract_status_codes(&report);
        println!("HTTP status codes: {:?}", codes);

        let total_time = calculate_total_retry_time(&report);
        println!("Total retry time: {}ms", total_time);

        let paired = pair_errors_with_delays(&report);
        println!("Error/delay pairs: {:?}", paired);
    }
}
