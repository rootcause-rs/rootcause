//! Tracing span capture for rootcause error reports.
//!
//! This example demonstrates automatic tracing span capture. When errors occur,
//! they include the active span context showing which operations were running.
//!
//! If you currently use `tracing_subscriber::fmt::init()`, this shows how to
//! expand that setup to add `RootcauseLayer`.

use rootcause::{hooks::Hooks, prelude::*};
use rootcause_tracing::{RootcauseLayer, SpanCollector};
use tracing::instrument;
use tracing_subscriber::{Registry, layer::SubscriberExt};

// Simple error types for this example
#[derive(Debug, thiserror::Error)]
#[error("database query failed")]
struct DatabaseError;

#[derive(Debug, thiserror::Error)]
#[error("permission denied")]
struct PermissionError;

#[derive(Debug, thiserror::Error)]
#[error("request failed")]
struct RequestError;

#[instrument(fields(query, table))]
fn query_database(_query: &str, _table: &str) -> Result<String, Report<DatabaseError>> {
    Err(report!(DatabaseError))?
}

#[instrument(fields(user_id, role))]
fn check_user_permission(_user_id: u64, _role: &str) -> Result<(), Report<PermissionError>> {
    query_database("SELECT permissions FROM users WHERE id = ?", "users")
        .attach("Failed to fetch user permissions")
        .context(PermissionError)?;

    Ok(())
}

#[instrument(fields(request_id, endpoint))]
fn handle_api_request(_request_id: &str, _endpoint: &str) -> Result<(), Report<RequestError>> {
    check_user_permission(12345, "admin")
        .attach("User lacks required permissions")
        .context(RequestError)?;

    Ok(())
}

fn main() {
    // Set up tracing subscriber with RootcauseLayer
    // This replaces `tracing_subscriber::fmt::init()` to add span capture
    let subscriber = Registry::default()
        .with(RootcauseLayer)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber).expect("failed to set default subscriber");

    // Install hook to automatically attach spans to all errors
    Hooks::new()
        .report_creation_hook(SpanCollector::new())
        .install()
        .expect("failed to install hooks");

    let result = handle_api_request("req-abc-123", "/api/admin/users");

    if let Err(report) = result {
        println!("{report}");
        println!();
        println!("Each error level shows the active spans from innermost to outermost.");
        println!("The deepest error includes all three spans with their field values.");
    }
}
