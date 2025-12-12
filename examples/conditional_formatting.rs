//! Conditional formatting based on runtime context
//!
//! Use conditional formatting to adapt error output based on runtime conditions:
//! - Hide sensitive data based on environment (production vs development)
//! - Show/hide verbose info based on feature flags or debug settings
//! - Customize visibility based on user permissions or logging levels
//!
//! Pattern: Check runtime state in AttachmentFormatterHook to control
//! placement/visibility

use std::env;

use rootcause::{
    hooks::{Hooks, attachment_formatter::AttachmentFormatterHook},
    markers::Dynamic,
    prelude::*,
    report_attachment::ReportAttachmentRef,
};

// Example 1: Hide sensitive data in production

/// API credentials that should never be exposed in production logs
#[derive(Debug)]
struct ApiCredentials {
    api_key: String,
    secret: String,
}

impl core::fmt::Display for ApiCredentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Show partial data in development for debugging
        write!(
            f,
            "API Key: {}..., Secret: {}...",
            &self.api_key[..8],
            &self.secret[..8]
        )
    }
}

/// Formatter that conditionally hides credentials based on environment
/// The same pattern works for feature flags, user permissions, etc.
struct CredentialsFormatter;

impl AttachmentFormatterHook<ApiCredentials> for CredentialsFormatter {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, Dynamic>,
        _report_formatting_function: rootcause::handlers::FormattingFunction,
    ) -> rootcause::handlers::AttachmentFormattingStyle {
        use rootcause::handlers::{
            AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction,
        };

        if is_production() {
            // Hide when condition is true (production, feature disabled, insufficient
            // permissions, etc.)
            AttachmentFormattingStyle {
                placement: AttachmentFormattingPlacement::Hidden,
                function: FormattingFunction::Display,
                priority: 0,
            }
        } else {
            // Show when condition is false
            AttachmentFormattingStyle {
                placement: AttachmentFormattingPlacement::Inline,
                function: FormattingFunction::Display,
                priority: 0,
            }
        }
    }
}

// Example 2: Show verbose debug info only in development

/// Detailed debugging information that's too verbose for production
#[derive(Debug)]
struct DebugSnapshot {
    memory_mb: usize,
    thread_count: usize,
    active_connections: usize,
    last_gc_ms: u64,
}

impl core::fmt::Display for DebugSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Debug Snapshot:")?;
        writeln!(f, "  Memory: {} MB", self.memory_mb)?;
        writeln!(f, "  Threads: {}", self.thread_count)?;
        writeln!(f, "  Connections: {}", self.active_connections)?;
        write!(f, "  Last GC: {} ms", self.last_gc_ms)
    }
}

/// Formatter that conditionally hides verbose info based on environment
/// Could also check debug flags, log levels, feature toggles, etc.
struct DebugSnapshotFormatter;

impl AttachmentFormatterHook<DebugSnapshot> for DebugSnapshotFormatter {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, Dynamic>,
        _report_formatting_function: rootcause::handlers::FormattingFunction,
    ) -> rootcause::handlers::AttachmentFormattingStyle {
        use rootcause::handlers::{
            AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction,
        };

        if is_production() {
            // Hide when runtime condition is true
            AttachmentFormattingStyle {
                placement: AttachmentFormattingPlacement::Hidden,
                function: FormattingFunction::Display,
                priority: 0,
            }
        } else {
            // Show in appendix when runtime condition is false
            AttachmentFormattingStyle {
                placement: AttachmentFormattingPlacement::Appendix {
                    appendix_name: "Debug Info",
                },
                function: FormattingFunction::Display,
                priority: 0,
            }
        }
    }
}

fn is_production() -> bool {
    env::var("APP_ENV")
        .map(|v| v == "production")
        .unwrap_or(false)
}

fn get_environment_name() -> String {
    env::var("APP_ENV").unwrap_or_else(|_| "development".to_string())
}

fn authenticate_api() -> Result<(), Report> {
    let creds = ApiCredentials {
        api_key: "sk_live_1234567890abcdef".to_string(),
        secret: "secret_1234567890abcdef".to_string(),
    };

    Err(report!(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "API authentication failed",
    ))
    .attach(creds)
    .into_dynamic())
}

fn process_request() -> Result<(), Report> {
    let snapshot = DebugSnapshot {
        memory_mb: 256,
        thread_count: 8,
        active_connections: 42,
        last_gc_ms: 15,
    };

    Err(report!("Request processing failed")
        .attach("Processing step: validation")
        .attach(snapshot)
        .into_dynamic())
}

fn main() {
    println!("=== Conditional Formatting Example ===\n");

    let env = get_environment_name();
    println!("Running in: {} mode", env);
    println!("(Set APP_ENV=production to see different behavior)\n");

    // Install conditional formatting hooks
    Hooks::new()
        .attachment_formatter::<ApiCredentials, _>(CredentialsFormatter)
        .attachment_formatter::<DebugSnapshot, _>(DebugSnapshotFormatter)
        .install()
        .expect("failed to install hooks");

    println!("Example 1: Conditionally hide sensitive data\n");
    match authenticate_api() {
        Ok(()) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            if is_production() {
                println!("Notice: Credentials were hidden based on environment check");
                println!("(Same pattern works for feature flags, permissions, etc.)\n");
            } else {
                println!("Notice: Credentials shown - condition evaluated to false\n");
            }
        }
    }

    println!("Example 2: Conditionally hide verbose info\n");
    match process_request() {
        Ok(()) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            if is_production() {
                println!("Notice: Debug info hidden based on runtime check");
                println!("(Could also check debug flags, log levels, etc.)");
            } else {
                println!("Notice: Debug info shown - use appendix to avoid clutter");
            }
        }
    }
}
