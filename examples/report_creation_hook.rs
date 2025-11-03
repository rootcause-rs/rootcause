//! Demonstrates using report creation hooks to automatically add context.
//!
//! This example shows:
//! 1. Registering ReportCreationHook for automatic context
//! 2. Using AttachmentCollectorHook for specific data
//! 3. Using closures for simple attachment collection
//! 4. Real-world patterns (request ID, timestamps, environment)

use rootcause::{
    ReportMut,
    hooks::report_creation::{
        AttachmentCollectorHook, ReportCreationHook, register_attachment_collector_hook,
        register_report_creation_hook,
    },
    markers::{Local, SendSync},
    prelude::*,
};
use std::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Example 1: Simple attachment collector with closure
// ============================================================================

fn setup_timestamp_collector() {
    // Register a closure that adds a timestamp to every error
    register_attachment_collector_hook(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
}

// ============================================================================
// Example 2: Attachment collector for request tracking
// ============================================================================

/// Global request ID counter for demonstration.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Request ID that we want to automatically attach to errors.
#[derive(Debug)]
struct RequestId(u64);

impl core::fmt::Display for RequestId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Request ID: {}", self.0)
    }
}

/// Collector that automatically gets the current request ID.
struct RequestIdCollector;

impl AttachmentCollectorHook<RequestId> for RequestIdCollector {
    type Handler = handlers::Display;

    fn collect(&self) -> RequestId {
        // In a real app, this might get from thread-local storage
        RequestId(REQUEST_COUNTER.load(Ordering::Relaxed))
    }
}

// ============================================================================
// Example 3: Custom report creation hook for environment info
// ============================================================================

/// Environment information we want on every error.
#[derive(Debug)]
struct EnvironmentInfo {
    mode: &'static str,
    version: &'static str,
}

impl core::fmt::Display for EnvironmentInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Environment: {} (v{})", self.mode, self.version)
    }
}

/// Hook that adds environment info to all reports.
struct EnvironmentHook;

impl ReportCreationHook for EnvironmentHook {
    fn on_local_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, Local>) {
        // Only add attachment to leaf reports (reports without children)
        if report.children().is_empty() {
            let env = EnvironmentInfo {
                mode: "development",
                version: "0.1.0",
            };
            report
                .attachments_mut()
                .push(report_attachment!(env).into());
        }
    }

    fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, SendSync>) {
        // Only add attachment to leaf reports (reports without children)
        if report.children().is_empty() {
            let env = EnvironmentInfo {
                mode: "development",
                version: "0.1.0",
            };
            report
                .attachments_mut()
                .push(report_attachment!(env).into());
        }
    }
}

// ============================================================================
// Example 4: Conditional attachment based on report type
// ============================================================================

/// Hook that adds debug info only for certain error types.
struct DebugInfoHook;

impl ReportCreationHook for DebugInfoHook {
    fn on_local_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, Local>) {
        // Only add to leaf reports (reports without children)
        if report.children().is_empty() {
            report
                .attachments_mut()
                .push(report_attachment!("Debug: Local report created").into());
        }
    }

    fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, SendSync>) {
        // Only add to leaf reports (reports without children)
        if report.children().is_empty() {
            report
                .attachments_mut()
                .push(report_attachment!("Debug: SendSync report created").into());
        }
    }
}

// ============================================================================
// Demo functions
// ============================================================================

fn simulate_api_request(endpoint: &str) -> Result<String, Report> {
    // Set current request ID
    REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);

    // Simulate an error
    Err(report!(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        format!("Failed to connect to {endpoint}"),
    ))
    .into_dyn_any())
}

fn process_data() -> Result<(), Report> {
    Err(report!("Data processing failed")
        .attach("Processing step: validation")
        .into_dyn_any())
}

fn nested_operation() -> Result<(), Report> {
    process_data()
        .context("Nested operation failed")
        .map_err(|e| e.into_dyn_any())
}

fn main() {
    println!("=== Setting up hooks ===\n");

    // Register all hooks
    setup_timestamp_collector();
    register_attachment_collector_hook(RequestIdCollector);
    register_report_creation_hook(EnvironmentHook);
    register_report_creation_hook(DebugInfoHook);

    println!("Registered hooks:");
    println!("  - Timestamp collector (closure)");
    println!("  - Request ID collector");
    println!("  - Environment info hook");
    println!("  - Debug info hook\n");

    println!("=== Example 1: API request with auto-attached context ===\n");
    match simulate_api_request("/api/users") {
        Ok(_) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Timestamp, Request ID, Environment, and Debug info were");
            println!("automatically attached by the hooks!\n");
        }
    }

    println!("=== Example 2: Another request with different ID ===\n");
    match simulate_api_request("/api/posts") {
        Ok(_) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Request ID incremented automatically\n");
        }
    }

    println!("=== Example 3: Nested operation with multiple reports ===\n");
    match nested_operation() {
        Ok(()) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Each report in the chain has hook-added context");
        }
    }
}
