//! Demonstrates how following error sources reveals valuable diagnostic
//! information hidden in error chains.
//!
//! Many error types (like `reqwest::Error`) wrap underlying causes but only
//! display a generic message at the top level. This example shows how enabling
//! source chain traversal dramatically improves error clarity.
//!
//! ## What are error sources?
//!
//! In Rust's error system, errors can implement `Error::source()` to point to
//! their underlying cause, creating a chain. For example, a network error might
//! have an IO error as its source, which has an OS error as its source. By
//! default, rootcause only displays the top-level error. Enabling source
//! following traverses this chain to show the complete diagnostic picture.

use rootcause::{
    ReportRef,
    hooks::{Hooks, context_formatter::ContextFormatterHook},
    markers::{Dynamic, Local, Uncloneable},
    prelude::*,
};
use rootcause_internals::handlers::{ContextFormattingStyle, FormattingFunction};

/// Simulates making an HTTP request that fails
async fn make_request(url: &str) -> Result<String, Report> {
    // This will fail with a DNS resolution error
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

/// Context formatter that enables source chain following for reqwest::Error
struct ReqwestErrorFormatter;

impl ContextFormatterHook<reqwest::Error> for ReqwestErrorFormatter {
    fn preferred_context_formatting_style(
        &self,
        _report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: report_formatting_function,
            // Enable following the error source chain
            follow_source: true,
            // Follow the full chain (no depth limit)
            follow_source_depth: None,
        }
    }
}

#[tokio::main]
async fn main() {
    // Many error types (reqwest, hyper, tokio) have rich source chains but only
    // show generic top-level messages. Enable source following to see the full
    // diagnostic chain.

    println!("Without source following:\n");

    let result1 = make_request("https://this-domain-definitely-does-not-exist-12345.com")
        .await
        .context("Failed to fetch user data");

    if let Err(err) = result1 {
        println!("{err}\n");
    }

    println!("With source following:\n");

    // Install a hook to follow the source chain for all reqwest::Error instances
    Hooks::new()
        .context_formatter::<reqwest::Error, _>(ReqwestErrorFormatter)
        .install()
        .expect("Failed to install hooks");

    let result2 = make_request("https://this-domain-definitely-does-not-exist-12345.com")
        .await
        .context("Failed to fetch user data");

    if let Err(err) = result2 {
        println!("{err}\n");
    }

    // Note: For your own error types, you can also enable source following
    // per-instance using ContextHandler. See custom_handler.rs for the handler
    // pattern, then implement preferred_formatting_style() to return
    // ContextFormattingStyle with follow_source: true.
}
