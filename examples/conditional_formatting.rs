//! Demonstrates conditional formatting based on environment.
//!
//! This example shows:
//! 1. Hiding sensitive data in production
//! 2. Adding extra debug info in development
//! 3. Using environment variables to control formatting
//! 4. Conditional attachment collection hooks

use rootcause::{
    hooks::{
        formatting_overrides::attachment::{
            AttachmentFormattingOverride, register_attachment_hook,
        },
        report_creation::{AttachmentCollectorHook, register_attachment_collector_hook},
    },
    prelude::*,
    report_attachment::ReportAttachmentRef,
};
use std::env;

// ============================================================================
// Example 1: Conditional sensitive data hiding
// ============================================================================

/// API credentials that should be hidden in production.
#[derive(Debug)]
struct ApiCredentials {
    api_key: String,
    secret: String,
}

impl core::fmt::Display for ApiCredentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if is_production() {
            write!(f, "[API credentials hidden in production]")
        } else {
            write!(
                f,
                "API Key: {}..., Secret: {}...",
                &self.api_key[..8],
                &self.secret[..8]
            )
        }
    }
}

/// Formatting hook that hides credentials completely in production.
struct CredentialsFormatter;

impl AttachmentFormattingOverride<ApiCredentials> for CredentialsFormatter {
    fn display(
        &self,
        attachment: ReportAttachmentRef<'_, ApiCredentials>,
        _parent: Option<rootcause::hooks::formatting_overrides::attachment::AttachmentParent<'_>>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        if is_production() {
            write!(f, "[REDACTED: API credentials not shown in production]")
        } else {
            let creds = attachment.inner();
            writeln!(f, "API Credentials (dev mode):")?;
            writeln!(f, "  API Key: {}", creds.api_key)?;
            write!(f, "  Secret: {}", creds.secret)
        }
    }

    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, dyn std::any::Any>,
        _report_formatting_function: rootcause::handlers::FormattingFunction,
    ) -> rootcause::handlers::AttachmentFormattingStyle {
        use rootcause::handlers::{
            AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction,
        };

        if is_production() {
            // Hide in production
            AttachmentFormattingStyle {
                placement: AttachmentFormattingPlacement::Hidden,
                function: FormattingFunction::Display,
                priority: 0,
            }
        } else {
            // Show with header in development
            AttachmentFormattingStyle {
                placement: AttachmentFormattingPlacement::InlineWithHeader {
                    header: "Debug: API Credentials",
                },
                function: FormattingFunction::Display,
                priority: 50,
            }
        }
    }
}

// ============================================================================
// Example 2: Conditional debug info collection
// ============================================================================

/// Debug information only collected in development.
#[derive(Debug)]
struct DebugMetrics {
    memory_mb: usize,
    uptime_secs: u64,
    thread_count: usize,
}

impl core::fmt::Display for DebugMetrics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Debug Metrics:")?;
        writeln!(f, "  Memory: {} MB", self.memory_mb)?;
        writeln!(f, "  Uptime: {} seconds", self.uptime_secs)?;
        write!(f, "  Threads: {}", self.thread_count)
    }
}

/// Collector that only gathers metrics in development.
struct ConditionalMetricsCollector;

impl AttachmentCollectorHook<String> for ConditionalMetricsCollector {
    type Handler = handlers::Display;

    fn collect(&self) -> String {
        if is_production() {
            // Don't collect in production - return empty string
            String::new()
        } else {
            // Collect debug metrics in development
            let metrics = DebugMetrics {
                memory_mb: 256, // Simulated
                uptime_secs: 3600,
                thread_count: 4,
            };
            format!("{}", metrics)
        }
    }
}

// ============================================================================
// Example 3: Environment-specific context
// ============================================================================

/// Context information that varies by environment.
#[derive(Debug)]
struct EnvironmentContext {
    mode: String,
    log_level: String,
    features_enabled: Vec<&'static str>,
}

impl core::fmt::Display for EnvironmentContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Environment: {}", self.mode)?;
        writeln!(f, "Log Level: {}", self.log_level)?;
        write!(f, "Features: {}", self.features_enabled.join(", "))
    }
}

struct EnvironmentContextCollector;

impl AttachmentCollectorHook<EnvironmentContext> for EnvironmentContextCollector {
    type Handler = handlers::Display;

    fn collect(&self) -> EnvironmentContext {
        EnvironmentContext {
            mode: get_environment_name(),
            log_level: if is_production() {
                "WARN".to_string()
            } else {
                "DEBUG".to_string()
            },
            features_enabled: if is_production() {
                vec!["production-features"]
            } else {
                vec!["dev-features", "debug-mode", "verbose-logging"]
            },
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn is_production() -> bool {
    env::var("APP_ENV")
        .map(|v| v == "production")
        .unwrap_or(false)
}

fn get_environment_name() -> String {
    env::var("APP_ENV").unwrap_or_else(|_| "development".to_string())
}

// ============================================================================
// Demo functions
// ============================================================================

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
    .into_dyn_any())
}

fn process_request() -> Result<(), Report> {
    Err(report!("Request processing failed")
        .attach("Processing step: validation")
        .into_dyn_any())
}

fn main() {
    println!("=== Conditional Formatting Example ===\n");

    let env = get_environment_name();
    println!("Running in: {} mode", env);
    println!("(Set APP_ENV=production to see production behavior)\n");

    // Register conditional hooks
    register_attachment_hook::<ApiCredentials, _>(CredentialsFormatter);
    register_attachment_collector_hook(ConditionalMetricsCollector);
    register_attachment_collector_hook(EnvironmentContextCollector);

    println!("=== Example 1: Sensitive data handling ===\n");
    match authenticate_api() {
        Ok(()) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            if is_production() {
                println!("Notice: Credentials were hidden in production mode\n");
            } else {
                println!("Notice: Credentials shown in development mode\n");
            }
        }
    }

    println!("=== Example 2: Automatic context collection ===\n");
    match process_request() {
        Ok(()) => println!("Success"),
        Err(error) => {
            eprintln!("{error}\n");
            if is_production() {
                println!("Notice: Debug metrics not collected in production");
            } else {
                println!("Notice: Debug metrics and environment info automatically attached");
            }
        }
    }
}
