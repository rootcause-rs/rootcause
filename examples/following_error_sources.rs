//! Demonstrates how to display error source chains in reports.
//!
//! By default, rootcause only displays the immediate error context. This
//! example shows how to enable source chain traversal to display the full error
//! chain, providing better diagnostic information.

use rootcause::{
    ReportRef,
    hooks::{Hooks, context_formatter::ContextFormatterHook},
    markers::{Dynamic, Local, Uncloneable},
    prelude::*,
};
use rootcause_internals::handlers::{ContextFormattingStyle, ContextHandler, FormattingFunction};

// A simple error type that can chain to other errors
#[derive(Debug)]
struct ChainedError {
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ChainedError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl std::fmt::Display for ChainedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(source) = &self.source {
            Some(&**source)
        } else {
            None
        }
    }
}

fn main() {
    // Create an error with a source chain
    let error = ChainedError::new("request failed").with_source(
        ChainedError::new("connection error")
            .with_source(ChainedError::new("TLS handshake failed")),
    );

    println!("=== Method 1: Using a ContextHandler ===\n");
    println!("Use this approach when the error type itself should control formatting.");
    println!("Best for:");
    println!("  • Library authors defining how their error types are displayed");
    println!("  • Different behavior for different error types");
    println!("  • When the decision is inherent to what the error represents\n");

    // Define a custom handler that enables source chain following.
    // This associates the behavior directly with the ChainedError type.
    struct ErrorWithSourcesHandler;
    impl ContextHandler<ChainedError> for ErrorWithSourcesHandler {
        fn source(value: &ChainedError) -> Option<&(dyn std::error::Error + 'static)> {
            std::error::Error::source(value)
        }

        fn display(
            value: &ChainedError,
            formatter: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            std::fmt::Display::fmt(value, formatter)
        }

        fn debug(
            value: &ChainedError,
            formatter: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            std::fmt::Debug::fmt(value, formatter)
        }

        fn preferred_formatting_style(
            _value: &ChainedError,
            formatting_function: FormattingFunction,
        ) -> ContextFormattingStyle {
            ContextFormattingStyle {
                function: formatting_function,
                // Enable source chain traversal
                follow_source: true,
                // Note: Set follow_source_depth to Some(n) to limit chain depth
                // No depth limit (show all)
                follow_source_depth: None,
            }
        }
    }

    let report = Report::new_sendsync_custom::<ErrorWithSourcesHandler>(error)
        .context("Failed to fetch data");
    println!("{report}");

    println!("\n=== Method 2: Using a ContextFormatterHook ===\n");
    println!("Use this approach for application-wide configuration of a specific type.");
    println!("Best for:");
    println!("  • Configuring third-party error types you don't control");
    println!("  • Environment-based behavior (dev vs production)");
    println!("  • Changing formatting without modifying where errors are created");
    println!("  • Centralizing configuration instead of specifying at each creation site\n");

    // Install a global hook that enables source chain following for ChainedError.
    // Unlike the handler approach, this applies to ALL ChainedError instances
    // in your application, even when created with Report::new() instead of
    // Report::new_sendsync_custom().
    struct ChainedErrorFormatter;
    impl ContextFormatterHook<ChainedError> for ChainedErrorFormatter {
        fn preferred_context_formatting_style(
            &self,
            _report: ReportRef<'_, Dynamic, Uncloneable, Local>,
            _report_formatting_function: FormattingFunction,
        ) -> ContextFormattingStyle {
            ContextFormattingStyle {
                function: FormattingFunction::Display,
                follow_source: true,
                follow_source_depth: None,
            }
        }
    }

    Hooks::new()
        .context_formatter::<ChainedError, _>(ChainedErrorFormatter)
        .install()
        .ok();

    // Create a new error (same structure as before)
    let error2 = ChainedError::new("request failed").with_source(
        ChainedError::new("connection error")
            .with_source(ChainedError::new("TLS handshake failed")),
    );

    // No custom handler needed - the hook applies globally
    let report2 = report!(error2).context("Failed to fetch data");
    println!("{report2}");
}
