// Custom attachment types for structured error context
//
// Custom types give you structured data you can retrieve and use programmatically,
// not just display. Implement Display + Debug to attach any type.

use rootcause::prelude::*;

// Custom attachment: single-line Display
#[derive(Debug)]
struct RequestInfo {
    method: &'static str,
    path: String,
    status_code: u16,
}

impl core::fmt::Display for RequestInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {} → {}", self.method, self.path, self.status_code)
    }
}

// Custom attachment: multiline Display
#[derive(Debug)]
struct ServerMetrics {
    active_connections: usize,
    memory_mb: usize,
}

impl core::fmt::Display for ServerMetrics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Server state:")?;
        writeln!(f, "  Connections: {}", self.active_connections)?;
        write!(f, "  Memory: {} MB", self.memory_mb)
    }
}

// Attach custom types to errors
fn make_request(path: &str, metrics: &ServerMetrics) -> Result<String, Report> {
    let request_info = RequestInfo {
        method: "GET",
        path: path.to_string(),
        status_code: 503,
    };

    Err(report!("Request failed")
        .attach(request_info)
        .attach(format!("{}", metrics))) // Both custom types in one error
}

// Retrieve custom attachments programmatically to make decisions
fn handle_request_with_retry(path: &str) -> Result<String, Report> {
    let metrics = ServerMetrics {
        active_connections: 1542,
        memory_mb: 4096,
    };

    match make_request(path, &metrics) {
        Ok(result) => Ok(result),
        Err(error) => {
            // Inspect RequestInfo to decide whether to retry
            let should_retry = error
                .attachments()
                .iter()
                .find_map(|att| att.downcast_inner::<RequestInfo>())
                .map(|info| info.status_code >= 500) // Retry on 5xx errors
                .unwrap_or(false);

            if should_retry {
                println!("5xx error detected - retry logic would run here");
            }

            Err(error.context("Request handling failed").into_dyn_any())
        }
    }
}

fn main() {
    println!("Example 1: Attach and format custom types\n");
    let metrics = ServerMetrics {
        active_connections: 1542,
        memory_mb: 4096,
    };
    match make_request("/api/data", &metrics) {
        Ok(_) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Example 2: Retrieve attachments to make decisions\n");
    match handle_request_with_retry("/api/users") {
        Ok(_) => println!("Success"),
        Err(error) => eprintln!("{error}"),
    }
}
