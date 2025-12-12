//! Retry logic with error collection: why each attempt failed
//!
//! ReportCollection gathers multiple errors so you can see the full history
//! of retry attempts instead of just the last failure.

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

fn fetch_document(attempt: usize) -> Result<Vec<u8>, Report> {
    let error = match attempt {
        1 => HttpError {
            code: 500,
            msg: "Internal server error",
        },
        2 => HttpError {
            code: 404,
            msg: "Not found",
        },
        _ => HttpError {
            code: 503,
            msg: "Service unavailable",
        },
    };

    Err(report!(error))?
}

fn fetch_document_with_retry(url: &str, retry_count: usize) -> Result<Vec<u8>, Report> {
    let mut errors = ReportCollection::new();

    for attempt in 1..=retry_count {
        match fetch_document(attempt).attach_with(|| format!("Attempt #{attempt}")) {
            Ok(data) => return Ok(data),
            Err(error) => errors.push(error.into_cloneable()),
        }
    }

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
