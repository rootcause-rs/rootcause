use rootcause::{prelude::*, report_collection::ReportCollection};

// Fake error to show the
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

fn fetch_document(_s: &str) -> Result<Vec<u8>, Report<HttpError>> {
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

    Err(report!(ERRORS[rand::random_range(0..ERRORS.len())]))
}

fn fetch_document_with_retry(s: &str, retry_count: usize) -> Result<Vec<u8>, Report> {
    let mut errors = ReportCollection::new();
    for i in 1..=retry_count {
        match fetch_document(s).attach_with(|| format!("Attempt #{i}")) {
            Ok(v) => return Ok(v),
            Err(e) => {
                errors.push(e.into_cloneable());
            }
        }
    }
    let mut error = report!("Unable to fetch document {s}");
    error.children_mut().extend(errors);
    Err(error)
}

fn main() {
    fetch_document_with_retry("http://example.com", 2).unwrap();
}
