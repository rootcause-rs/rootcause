// Retry logic with error collection: why each attempt failed
//
// ReportCollection gathers multiple errors so you can see the full history
// of retry attempts instead of just the last failure.

use rootcause::{prelude::*, report_collection::ReportCollection};

#[derive(Copy, Clone, Debug)]
struct HttpError {
    code: usize,
    msg: &'static str,
}

impl core::fmt::Display for HttpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HTTP error: {} {}", self.code, self.msg)
    }
}

// Simulates network fetch (always fails for demonstration)
fn fetch_document(_url: &str) -> Result<Vec<u8>, Report> {
    const ERRORS: [HttpError; 3] = [
        HttpError {
            code: 500,
            msg: "Internal server error",
        },
        HttpError {
            code: 404,
            msg: "Not found",
        },
        HttpError {
            code: 400,
            msg: "Bad Request: Could not parse JSON payload",
        },
    ];

    // Cycle through errors deterministically
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let index = COUNTER.fetch_add(1, Ordering::Relaxed) % ERRORS.len();

    Err(report!(ERRORS[index]))? // HttpError → Report<HttpError> → Report<Dynamic>
}

// Retry logic that preserves all failure information
fn fetch_document_with_retry(url: &str, retry_count: usize) -> Result<Vec<u8>, Report> {
    let mut errors = ReportCollection::new(); // Accumulate all failures

    for attempt in 1..=retry_count {
        match fetch_document(url).attach_with(|| format!("Attempt #{attempt}")) {
            Ok(data) => return Ok(data),
            Err(error) => {
                errors.push(error.into_cloneable()); // Store this attempt's failure
            }
        }
    }

    // Return all failures as children of a parent error
    Err(errors.context(format!("Unable to fetch document {url}")))?
}

fn main() {
    // Shows error with full retry history:
    // - Parent: "Unable to fetch document"
    // - Child 1: "Attempt #1" → HTTP 500
    // - Child 2: "Attempt #2" → HTTP 404
    if let Err(report) = fetch_document_with_retry("http://example.com", 2) {
        eprintln!("{report}");
    }
}
