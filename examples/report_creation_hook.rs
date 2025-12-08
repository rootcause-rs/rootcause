// Report creation hooks - automatically attach context when errors are created
//
// Report creation hooks vs formatting hooks:
// - Creation hooks (this example): Automatically attach data when reports are
//   created
// - Formatting hooks (see formatting_hooks.rs): Control how
//   attachments/contexts are displayed
//
// Two types of creation hooks:
// - AttachmentCollectorHook: Simple - always collects and attaches data
// - ReportCreationHook: Advanced - conditional logic based on report state

use std::sync::atomic::{AtomicU64, Ordering};

use rootcause::{
    ReportMut,
    hooks::{
        Hooks,
        report_creation::{AttachmentCollector, ReportCreationHook},
    },
    markers::{Dynamic, Local, SendSync},
    prelude::*,
};

// Example 1: AttachmentCollectorHook - automatic request tracking

/// Global request ID counter for demonstration
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Request ID that we want to automatically attach to all errors
#[derive(Debug)]
struct RequestId(u64);

impl core::fmt::Display for RequestId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Request ID: {}", self.0)
    }
}

/// Collector that automatically gets the current request ID
/// This is called for EVERY report created - use for lightweight data
struct RequestIdCollector;

impl AttachmentCollector<RequestId> for RequestIdCollector {
    type Handler = handlers::Display;

    fn collect(&self) -> RequestId {
        // In a real app, this might get from thread-local storage
        RequestId(REQUEST_COUNTER.load(Ordering::Relaxed))
    }
}

// Example 2: ReportCreationHook - conditional attachment based on error type

/// Hint about whether an operation can be retried
struct RetryHint(&'static str);

impl core::fmt::Display for RetryHint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "💡 {}", self.0)
    }
}

impl core::fmt::Debug for RetryHint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

/// Hook that inspects the error type and adds retry hints for transient errors
/// Use ReportCreationHook when you need to inspect the report to decide what to
/// attach
struct RetryHintHook;

impl ReportCreationHook for RetryHintHook {
    fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, Local>) {
        // Inspect the error to see if it's a transient network error
        if let Some(io_error) = report.downcast_current_context::<std::io::Error>() {
            let hint = match io_error.kind() {
                std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::ConnectionReset => {
                    Some(RetryHint("This is a transient error - retry may succeed"))
                }
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
                    Some(RetryHint("This error is permanent - retrying won't help"))
                }
                _ => None,
            };

            if let Some(hint) = hint {
                report
                    .attachments_mut()
                    .push(report_attachment!(hint).into());
            }
        }
    }

    fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, SendSync>) {
        // Same logic for SendSync errors
        if let Some(io_error) = report.downcast_current_context::<std::io::Error>() {
            let hint = match io_error.kind() {
                std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::ConnectionReset => {
                    Some(RetryHint("This is a transient error - retry may succeed"))
                }
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
                    Some(RetryHint("This error is permanent - retrying won't help"))
                }
                _ => None,
            };

            if let Some(hint) = hint {
                report
                    .attachments_mut()
                    .push(report_attachment!(hint).into());
            }
        }
    }
}

fn simulate_api_request(request_id: u64) -> Result<String, Report> {
    // Set current request ID for tracking
    REQUEST_COUNTER.store(request_id, Ordering::Relaxed);

    // Simulate an error - notice we don't manually attach the request ID
    Err(report!(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "Failed to connect to API",
    ))
    .into_dynamic())
}

fn file_operation(exists: bool) -> Result<(), Report> {
    // Simulate different error types
    if exists {
        Err(report!(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Access denied to file",
        ))
        .into_dynamic())
    } else {
        Err(report!(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ))
        .into_dynamic())
    }
}

fn main() {
    println!("Example 1: AttachmentCollectorHook - automatic request tracking\n");

    // Install the request ID collector hook
    Hooks::new()
        .attachment_collector(RequestIdCollector)
        .install()
        .expect("failed to install hooks");

    println!("Making first API request...");
    match simulate_api_request(1001) {
        Ok(_) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Request ID was automatically attached!\n");
        }
    }

    println!("Making second API request...");
    match simulate_api_request(1002) {
        Ok(_) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Different request ID, automatically tracked\n");
        }
    }

    println!("\nExample 2: ReportCreationHook - conditional retry hints\n");

    // Install retry hint hook (replaces previous hooks)
    Hooks::new().report_creation_hook(RetryHintHook).replace();

    println!("Attempting transient network error...");
    match simulate_api_request(1003) {
        Ok(_) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Retry hint was added based on error type (ConnectionRefused)\n");
        }
    }

    println!("Attempting permanent file error...");
    match file_operation(false) {
        Ok(()) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            println!("Notice: Different retry hint for permanent error (NotFound)");
        }
    }
}
