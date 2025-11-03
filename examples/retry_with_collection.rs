use rootcause::{prelude::*, report_collection::ReportCollection};

/// Example HTTP error type to demonstrate error reporting.
///
/// In a real application, this might come from an HTTP client library,
/// but we define it here to make the example self-contained.
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

/// Simulates fetching a document from the network.
///
/// This always fails with a random HTTP error to demonstrate error handling.
/// In a real application, this would make an actual HTTP request.
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

    // Cycle through errors deterministically for reproducible output
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let index = COUNTER.fetch_add(1, Ordering::Relaxed) % ERRORS.len();

    // ? coerces Report<HttpError> to Report<dyn Any>
    Err(report!(ERRORS[index]))?
}

/// Attempts to fetch a document with retry logic.
///
/// This demonstrates a key rootcause feature: collecting multiple errors
/// using [`ReportCollection`] to show the full history of retry attempts.
///
/// The function:
/// 1. Tries to fetch the document multiple times
/// 2. Attaches attempt number to each error
/// 3. Collects all failures in a ReportCollection
/// 4. Returns a parent error with all attempts as children
fn fetch_document_with_retry(url: &str, retry_count: usize) -> Result<Vec<u8>, Report> {
    // ReportCollection accumulates multiple errors
    let mut errors = ReportCollection::new();

    for attempt in 1..=retry_count {
        // Try to fetch the document, attaching the attempt number
        match fetch_document(url).attach_with(|| format!("Attempt #{attempt}")) {
            Ok(data) => return Ok(data),
            Err(error) => {
                // Convert to cloneable so we can store it in the collection
                errors.push(error.into_cloneable());
            }
        }
    }

    // Return the collection as an error with context
    Err(errors.context(format!("Unable to fetch document {url}")))?
}

fn main() {
    // This intentionally panics to show the formatted error output.
    // The panic message demonstrates:
    // - The parent error message
    // - Each retry attempt as a child error
    // - Attachment showing which attempt failed
    // - The original HTTP error for each attempt
    // - Source code locations where errors were created
    //
    // This output is included in the README to show users what
    // rootcause error reports look like.
    fetch_document_with_retry("http://example.com", 2).unwrap();
}
