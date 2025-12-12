//! Using derive_more errors with rootcause.
//!
//! **Run this example:** `cargo run --example derive_more_interop`
//!
//! Three patterns for integrating derive_more-generated errors:
//!
//! 1. **Type-nesting with `#[from]`** - Traditional derive_more pattern
//!    - One location captured per error
//!    - Minimal migration from plain derive_more
//!
//! 2. **Early Report creation** - Return `Report<E>` from lower functions
//!    - Locations captured closer to the error source
//!    - Easier to capture multiple locations
//!    - Use `.context_transform()` or `ReportConversion` trait
//!
//! 3. **Flat enums with Report nesting** - Categories with child Reports
//!    - Multiple locations captured
//!    - Flexible categorization via `.context()` or `ReportConversion`
//!
//! **What's next?**
//! - See all examples? → `examples/README.md`

use derive_more::{Display, Error, From};
use rootcause::{ReportConversion, markers, prelude::*};

// Shared error types used across patterns
#[derive(Error, Debug, Display)]
#[expect(dead_code, reason = "example code")]
enum DatabaseError {
    #[display("Connection timeout after {seconds}s")]
    ConnectionTimeout { seconds: u64 },
    #[display("Query timeout after {seconds}s")]
    QueryTimeout { seconds: u64 },
}

#[derive(Error, Debug, Display)]
#[expect(dead_code, reason = "example code")]
enum ConfigError {
    #[display("Invalid format in {file}")]
    InvalidFormat { file: String },
}

// ============================================================================
// Pattern 1: Type-nesting with #[from]
// ============================================================================

#[derive(Error, Debug, Display, From)]
enum AppError1 {
    #[display("Database error")]
    Database(DatabaseError),
}

fn query_plain(_id: u32) -> Result<String, DatabaseError> {
    Err(DatabaseError::ConnectionTimeout { seconds: 30 })
}

// Only ONE location captured (here)
fn process_type_nested(user_id: u32) -> Result<String, Report<AppError1>> {
    let data = query_plain(user_id).map_err(AppError1::from)?;
    Ok(data)
}

// Pattern matching on typed reports enables selective handling
fn handle_typed_error(user_id: u32) -> Result<String, Report<AppError1>> {
    match process_type_nested(user_id) {
        Ok(data) => Ok(data),
        Err(report) => {
            // Access the typed error for decision-making
            match report.current_context() {
                AppError1::Database(DatabaseError::ConnectionTimeout { .. }) => {
                    println!("  → Detected connection timeout, could retry here");
                    Err(report)
                }
                AppError1::Database(DatabaseError::QueryTimeout { .. }) => {
                    println!("  → Query timeout, could adjust query parameters");
                    Err(report)
                }
            }
        }
    }
}

// ============================================================================
// Pattern 2: Early Report creation
// ============================================================================

#[derive(Error, Debug, Display, From)]
enum AppError2 {
    #[display("Database error")]
    Database(DatabaseError),
}

// Locations captured close to the error source
fn query_report(_id: u32) -> Result<String, Report<DatabaseError>> {
    Err(report!(DatabaseError::ConnectionTimeout { seconds: 30 }))
}

// This still captures only ONE location per error, but it's close to the source
fn process_early_report(user_id: u32) -> Result<String, Report<AppError2>> {
    let data = query_report(user_id).context_transform(AppError2::Database)?;
    Ok(data)
}

// If we want to capture multiple locations, we can use context_transform_nested
// instead
fn process_early_report_multiple_locations(user_id: u32) -> Result<String, Report<AppError2>> {
    let data = query_report(user_id).context_transform_nested(AppError2::Database)?;
    Ok(data)
}

// Systematic conversion with ReportConversion trait
impl<T> ReportConversion<DatabaseError, markers::Mutable, T> for AppError2
where
    AppError2: markers::ObjectMarkerFor<T>,
{
    fn convert_report(
        report: Report<DatabaseError, markers::Mutable, T>,
    ) -> Report<Self, markers::Mutable, T> {
        report.context_transform(AppError2::Database)
    }
}

fn process_with_conversion(user_id: u32) -> Result<String, Report<AppError2>> {
    let data = query_report(user_id).context_to::<AppError2>()?;
    Ok(data)
}

// ============================================================================
// Pattern 3: Flat enums with Report nesting (flexible categorization)
// ============================================================================
//
// Unlike Patterns 1-2 (which require 1-to-1 mapping with #[from]), Pattern 3
// lets you design categories independently of error type structure:
// - Split: one error type into multiple categories based on variants
// - Merge: multiple error types into one category

#[derive(Error, Debug, Display)]
enum AppError3 {
    #[display("Database connection issue")]
    DatabaseConnection,
    #[display("Database operation failed")]
    DatabaseOther,
    #[display("System error")]
    System,
}

// Split: one DatabaseError type into different categories
impl<T> ReportConversion<DatabaseError, markers::Mutable, T> for AppError3
where
    AppError3: markers::ObjectMarkerFor<T>,
{
    fn convert_report(
        report: Report<DatabaseError, markers::Mutable, T>,
    ) -> Report<Self, markers::Mutable, T> {
        match report.current_context() {
            DatabaseError::ConnectionTimeout { .. } => report.context(Self::DatabaseConnection),
            DatabaseError::QueryTimeout { .. } => report.context(Self::DatabaseOther),
        }
    }
}

// Merge: different error types into one category
impl<T> ReportConversion<ConfigError, markers::Mutable, T> for AppError3
where
    AppError3: markers::ObjectMarkerFor<T>,
{
    fn convert_report(
        report: Report<ConfigError, markers::Mutable, T>,
    ) -> Report<Self, markers::Mutable, T> {
        report.context(Self::System)
    }
}

fn main() {
    println!("\n## Pattern 1: Type-nesting with #[from]\n");
    println!("Only one location captured and it's not at the source:");
    if let Err(report) = process_type_nested(123) {
        eprintln!("{report}\n");
    }

    println!("Pattern matching on typed reports:");
    let _ = handle_typed_error(123);
    println!();

    println!("\n## Pattern 2: Early Report creation\n");
    println!("Locations captured close to the error source:");
    if let Err(report) = process_early_report(123) {
        eprintln!("{report}\n");
    }

    println!("Multiple locations captured using context_transform_nested:");
    if let Err(report) = process_early_report_multiple_locations(123) {
        eprintln!("{report}\n");
    }

    println!("Using ReportConversion trait:");
    if let Err(report) = process_with_conversion(123) {
        eprintln!("{report}\n");
    }

    println!("\n## Pattern 3: Flexible categorization\n");
    println!("The data type of AppError3 does not have to be 1-to-1 with the underlying errors:");
    if let Err(report) = query_report(123).context_to::<AppError3>() {
        eprintln!("{report}\n");
    }
}
