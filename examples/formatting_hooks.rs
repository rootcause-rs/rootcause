//! Demonstrates using formatting hooks to customize error output.
//!
//! This example shows:
//! 1. Registering custom formatting hooks for attachment types
//! 2. Using AttachmentFormattingPlacement to control where attachments appear
//! 3. Setting priority for attachment ordering
//! 4. Using InlineWithHeader for structured data
//! 5. Hiding sensitive data conditionally

use rootcause::{
    handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
    hooks::formatting_overrides::attachment::{
        AttachmentFormattingOverride, AttachmentParent, register_attachment_hook,
    },
    prelude::*,
    report_attachment::ReportAttachmentRef,
};

// ============================================================================
// Example 1: Custom formatting with InlineWithHeader
// ============================================================================

/// API error information that we want to format nicely.
#[derive(Debug)]
struct ApiError {
    code: u32,
    message: String,
    endpoint: String,
}

impl core::fmt::Display for ApiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "API Error {}: {}", self.code, self.message)
    }
}

/// Custom formatter that adds a header and high priority.
struct ApiErrorFormatter;

impl AttachmentFormattingOverride<ApiError> for ApiErrorFormatter {
    fn display(
        &self,
        attachment: ReportAttachmentRef<'_, ApiError>,
        _parent: Option<AttachmentParent<'_>>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        let err = attachment.inner();
        writeln!(f, "Code: {}", err.code)?;
        writeln!(f, "Message: {}", err.message)?;
        write!(f, "Endpoint: {}", err.endpoint)
    }

    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, dyn core::any::Any>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::InlineWithHeader {
                header: "API Error Details",
            },
            function: FormattingFunction::Display,
            priority: 100, // High priority - shows first
        }
    }
}

// ============================================================================
// Example 2: Verbose data in appendix
// ============================================================================

/// Stack trace data that's too verbose to show inline.
#[derive(Debug)]
struct StackTrace {
    frames: Vec<String>,
}

impl core::fmt::Display for StackTrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, frame) in self.frames.iter().enumerate() {
            writeln!(f, "  {}: {}", i, frame)?;
        }
        Ok(())
    }
}

/// Formatter that puts stack traces in an appendix.
struct StackTraceFormatter;

impl AttachmentFormattingOverride<StackTrace> for StackTraceFormatter {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, dyn core::any::Any>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Appendix {
                appendix_name: "Full Stack Trace",
            },
            function: FormattingFunction::Display,
            priority: 0, // Low priority - goes to appendix anyway
        }
    }
}

// ============================================================================
// Example 3: Hiding sensitive data
// ============================================================================

/// Sensitive credentials that should never appear in logs.
#[derive(Debug)]
struct Credentials {
    username: String,
    #[allow(dead_code)]
    password: String,
}

impl core::fmt::Display for Credentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Username: {}, Password: ********", self.username)
    }
}

/// Formatter that hides credentials completely.
struct HideCredentialsFormatter;

impl AttachmentFormattingOverride<Credentials> for HideCredentialsFormatter {
    fn display(
        &self,
        _attachment: ReportAttachmentRef<'_, Credentials>,
        _parent: Option<AttachmentParent<'_>>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        write!(f, "[Credentials hidden for security]")
    }

    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, dyn core::any::Any>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Hidden, // Don't show at all
            function: FormattingFunction::Display,
            priority: 0,
        }
    }
}

// ============================================================================
// Example 4: Priority-based ordering
// ============================================================================

/// Important action the user must take.
struct ActionRequired(String);

impl core::fmt::Display for ActionRequired {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "⚠️  ACTION REQUIRED: {}", self.0)
    }
}

impl core::fmt::Debug for ActionRequired {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Formatter with very high priority to show this first.
struct ActionRequiredFormatter;

impl AttachmentFormattingOverride<ActionRequired> for ActionRequiredFormatter {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, dyn core::any::Any>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Inline,
            function: FormattingFunction::Display,
            priority: 200, // Very high priority - always shows first
        }
    }
}

// ============================================================================
// Demo functions
// ============================================================================

fn demo_api_error() -> Result<(), Report> {
    let error = ApiError {
        code: 503,
        message: "Service Unavailable".to_string(),
        endpoint: "/api/v1/users".to_string(),
    };

    Err(report!("Request failed").attach(error))
}

fn demo_stack_trace() -> Result<(), Report> {
    let trace = StackTrace {
        frames: vec![
            "process_request at src/api.rs:42".to_string(),
            "handle_connection at src/server.rs:123".to_string(),
            "main at src/main.rs:15".to_string(),
        ],
    };

    Err(report!("Internal server error")
        .attach("User-facing error message")
        .attach(trace))
}

fn demo_credentials() -> Result<(), Report> {
    let creds = Credentials {
        username: "admin".to_string(),
        password: "super_secret_123".to_string(),
    };

    Err(report!("Authentication failed").attach(creds))
}

fn demo_priority() -> Result<(), Report> {
    Err(report!("Configuration error")
        .attach("Detailed diagnostic info here")
        .attach(ActionRequired("Update your config.toml file".to_string()))
        .attach("Additional context about the error"))
}

fn main() {
    // Register all our custom formatters
    register_attachment_hook::<ApiError, _>(ApiErrorFormatter);
    register_attachment_hook::<StackTrace, _>(StackTraceFormatter);
    register_attachment_hook::<Credentials, _>(HideCredentialsFormatter);
    register_attachment_hook::<ActionRequired, _>(ActionRequiredFormatter);

    println!("=== Example 1: InlineWithHeader for structured data ===\n");
    match demo_api_error() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 2: Appendix for verbose data ===\n");
    println!("Note: Stack trace appears in appendix section\n");
    match demo_stack_trace() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 3: Hidden sensitive data ===\n");
    println!("Note: Credentials are completely hidden\n");
    match demo_credentials() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 4: Priority-based ordering ===\n");
    println!("Note: ACTION REQUIRED appears first due to high priority\n");
    match demo_priority() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}"),
    }
}
