//! Type-safe error handling with `Report<C>`.
//!
//! **Run this example:** `cargo run --example typed_reports`
//!
//! By default, rootcause uses `Report<Dynamic>` which can hold any error type.
//! This is perfect when you just need errors to propagate. But sometimes you
//! want callers to be able to pattern match and handle specific errors
//! programmatically - that's when you use `Report<YourError>`.
//!
//! ## When to Use Typed Errors
//!
//! **Use `Report` (dynamic) when:**
//! - You just need errors to propagate with context
//! - Callers don't need to make decisions based on error variants
//! - Like anyhow - simple and flexible
//!
//! **Use `Report<YourError>` (typed) when:**
//! - Callers need to pattern match on specific errors
//! - You want exhaustiveness checking on error handling
//! - You want both type safety AND rich context (better than plain thiserror)
//!
//! **Example:** This example shows a database function that returns
//! `Report<DatabaseError>` so callers can retry transient errors
//! (ConnectionLost, QueryTimeout) but not permanent ones (ConstraintViolation,
//! NotFound). The retry logic below demonstrates pattern matching on typed
//! errors.
//!
//! ## Key Concepts
//!
//! - `Report<C>` preserves type information for pattern matching
//! - `Report<Dynamic>` type-erases for flexibility with multiple error types
//! - The `?` operator automatically converts between them
//! - Use `.current_context()` to access the typed error for pattern matching
//!
//! **Note:** This example focuses on the context type parameter. For other
//! type parameters (Cloneable, Local), see the API docs.
//!
//! **What's next?**
//! - Confused about type conversions? → `error_coercion.rs` explains how `?`
//!   works
//! - See all examples? → `examples/README.md`

use rootcause::prelude::*;

#[derive(Debug, Clone)]
enum DatabaseError {
    ConnectionLost,
    QueryTimeout,
    ConstraintViolation { constraint: String },
    NotFound,
}

impl core::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConnectionLost => write!(f, "Database connection lost"),
            Self::QueryTimeout => write!(f, "Query execution timeout"),
            Self::ConstraintViolation { constraint } => {
                write!(f, "Constraint violation: {constraint}")
            }
            Self::NotFound => write!(f, "Record not found"),
        }
    }
}

impl core::error::Error for DatabaseError {}

/// Returns typed report: Report<DatabaseError> preserves type for pattern
/// matching.
fn query_user(user_id: u32) -> Result<String, Report<DatabaseError>> {
    // Simulate different error conditions based on user_id
    let error = match user_id {
        1 => DatabaseError::ConnectionLost,
        2 => DatabaseError::QueryTimeout,
        3 => DatabaseError::ConstraintViolation {
            constraint: "unique_email".to_string(),
        },
        _ => DatabaseError::NotFound,
    };

    Err(report!(error).attach(format!("User ID: {user_id}")))
}

/// Pattern matching on typed reports enables intelligent error handling.
///
/// This retry logic only retries transient errors (ConnectionLost,
/// QueryTimeout) and immediately fails on permanent errors
/// (ConstraintViolation, NotFound).
fn query_user_with_retry(user_id: u32) -> Result<String, Report> {
    const MAX_RETRIES: usize = 3;

    let mut attempt = 1;
    loop {
        match query_user(user_id) {
            Ok(user) => return Ok(user),
            Err(report) => {
                // .current_context() gives us access to the typed error (DatabaseError)
                // so we can pattern match on it to make intelligent decisions
                let should_retry = match report.current_context() {
                    DatabaseError::ConnectionLost | DatabaseError::QueryTimeout => true,
                    DatabaseError::ConstraintViolation { .. } | DatabaseError::NotFound => false,
                };

                if !(should_retry && attempt < MAX_RETRIES) {
                    // Convert to dynamic report since application code doesn't need
                    // to pattern match anymore - the retry logic already handled it
                    return Err(report
                        .context(format!("Failed to query user after {attempt} attempts"))
                        .into_dynamic());
                }
            }
        }
        attempt += 1;
    }
}

/// Libraries use Report<SpecificError> so callers can pattern match.
fn library_function_example() -> Result<(), Report<DatabaseError>> {
    query_user(42)?;
    Ok(())
}

/// Applications use Report<Dynamic> to handle diverse error types uniformly.
fn application_function_example() -> Result<(), Report> {
    // ? automatically converts Report<DatabaseError> to Report<Dynamic>
    library_function_example()?;
    Ok(())
}

fn main() {
    println!("=== Typed Reports Example ===\n");

    println!("Example 1: Pattern matching enables selective retry\n");

    println!("User ID 1 (ConnectionLost - transient, will retry):");
    if let Err(report) = query_user_with_retry(1) {
        println!("{report}\n");
    }

    println!("User ID 3 (ConstraintViolation - permanent, won't retry):");
    if let Err(report) = query_user_with_retry(3) {
        println!("{report}\n");
    }

    println!("\nExample 2: Library returns typed report for pattern matching");
    if let Err(report) = library_function_example() {
        println!("{report}\n");
    }

    println!("Example 3: Application uses dynamic report for flexibility");
    if let Err(report) = application_function_example() {
        println!("{report}");
    }
}
