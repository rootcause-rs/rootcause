//! Demonstrates creating custom attachment types with their own formatting.
//!
//! This example shows:
//! 1. Creating custom attachment types with Display and Debug
//! 2. Using custom handlers for specialized formatting
//! 3. Attaching structured data to errors
//! 4. Different ways to work with custom types

use rootcause::prelude::*;
use std::time::SystemTime;

/// Custom attachment type representing HTTP request information.
///
/// This is useful when reporting errors from web services - you can attach
/// details about the request that failed.
#[derive(Debug)]
struct RequestInfo {
    method: &'static str,
    path: String,
    status_code: u16,
    duration_ms: u64,
}

impl core::fmt::Display for RequestInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} {} returned {} (took {}ms)",
            self.method, self.path, self.status_code, self.duration_ms
        )
    }
}

/// Custom attachment for server metrics.
///
/// When errors occur, it's often helpful to know the server state at the time.
#[derive(Debug)]
struct ServerMetrics {
    active_connections: usize,
    memory_mb: usize,
    cpu_percent: f32,
}

impl core::fmt::Display for ServerMetrics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Server State:")?;
        writeln!(f, "  Active connections: {}", self.active_connections)?;
        writeln!(f, "  Memory usage: {} MB", self.memory_mb)?;
        write!(f, "  CPU usage: {:.1}%", self.cpu_percent)
    }
}

/// Simulates an HTTP request that fails.
fn make_request(path: &str) -> Result<String, Report> {
    let start = SystemTime::now();

    // Simulate request failure
    let status_code = 503;
    let duration_ms = start.elapsed().unwrap().as_millis() as u64;

    let request_info = RequestInfo {
        method: "GET",
        path: path.to_string(),
        status_code,
        duration_ms,
    };

    Err(report!("Request failed")
        .attach(request_info)
        .attach("Reason: Service temporarily unavailable"))
}

/// Simulates a server error with system state attached.
fn handle_connection() -> Result<(), Report> {
    let metrics = ServerMetrics {
        active_connections: 1542,
        memory_mb: 4096,
        cpu_percent: 87.5,
    };

    let _ = make_request("/api/users")
        .context("Connection handler failed")
        .attach(metrics)?;

    Ok(())
}

/// Demonstrates attaching different types of data.
fn process_with_metadata() -> Result<(), Report> {
    // You can attach any type that implements Display + Debug

    // Simple types
    let user_id = 12345;

    // Use standard library for timestamps (no external dependencies needed)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Err(report!("Database query failed")
        .attach(format!("User ID: {user_id}"))
        .attach(format!("Timestamp: {timestamp}"))
        .attach("Query: SELECT * FROM users WHERE id = ?"))
}

fn main() {
    println!("=== Example 1: Custom RequestInfo attachment ===\n");
    match make_request("/api/data") {
        Ok(_) => println!("Request succeeded"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 2: Custom ServerMetrics attachment ===\n");
    match handle_connection() {
        Ok(()) => println!("Connection handled"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 3: Multiple formatted attachments ===\n");
    match process_with_metadata() {
        Ok(()) => println!("Processing complete"),
        Err(error) => eprintln!("{error}"),
    }
}
