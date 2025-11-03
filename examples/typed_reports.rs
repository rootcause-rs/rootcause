//! Demonstrates the difference between `Report<C>` and `Report<dyn Any>`.
//!
//! This example shows:
//! 1. When to use typed reports (`Report<C>`) vs dynamic reports (`Report<dyn Any>`)
//! 2. Pattern matching on typed reports for error recovery
//! 3. Different error handling strategies based on error type
//! 4. Converting between typed and dynamic reports

use rootcause::prelude::*;

/// Custom error type for database operations.
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

/// Simulates a database operation that can fail with specific error types.
///
/// Returns `Report<DatabaseError>` to preserve type information, allowing
/// callers to handle different error cases differently.
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

/// Demonstrates pattern matching on typed reports for intelligent error recovery.
///
/// This function uses `Report<DatabaseError>` to implement retry logic that
/// only retries on transient errors (connection loss, timeout).
fn query_user_with_retry(user_id: u32) -> Result<String, Report> {
    const MAX_RETRIES: usize = 3;

    for attempt in 1..=MAX_RETRIES {
        match query_user(user_id) {
            Ok(user) => return Ok(user),
            Err(report) => {
                // Pattern match on the typed report to decide whether to retry
                let should_retry = match report.as_ref().current_context() {
                    DatabaseError::ConnectionLost | DatabaseError::QueryTimeout => {
                        // Transient errors - worth retrying
                        true
                    }
                    DatabaseError::ConstraintViolation { .. } | DatabaseError::NotFound => {
                        // Permanent errors - no point in retrying
                        false
                    }
                };

                if should_retry && attempt < MAX_RETRIES {
                    println!("  Retrying after transient error (attempt {attempt})...");
                    continue;
                }

                // Out of retries or permanent error - return the error
                // ? coerces Report<DatabaseError> to Report<dyn Any>
                return Err(
                    report.context(format!("Failed to query user after {attempt} attempts"))
                )?;
            }
        }
    }

    unreachable!()
}

/// Demonstrates using typed reports in library code.
///
/// Libraries often return `Report<SpecificError>` so that callers can:
/// 1. Pattern match on error types for recovery
/// 2. Handle different errors differently
/// 3. Make informed decisions about retry logic
fn library_function_example() -> Result<(), Report<DatabaseError>> {
    query_user(42)?;
    Ok(())
}

/// Demonstrates converting typed to dynamic reports in application code.
///
/// Application code typically uses `Report<dyn Any>` because:
/// 1. It needs to handle many different error types
/// 2. It doesn't usually pattern match on specific errors
/// 3. It focuses on error reporting rather than recovery
fn application_function_example() -> Result<(), Report> {
    // The ? operator coerces Report<DatabaseError> to Report<dyn Any>
    library_function_example()?;
    Ok(())
}

fn main() {
    println!("=== Typed Reports Example ===\n");

    // Example 1: Pattern matching for intelligent retry
    println!("Example 1: Retry on transient errors only");
    println!("User ID 1 (ConnectionLost - will retry):");
    if let Err(report) = query_user_with_retry(1) {
        println!("{report}\n");
    }

    println!("User ID 3 (ConstraintViolation - won't retry):");
    if let Err(report) = query_user_with_retry(3) {
        println!("{report}\n");
    }

    // Example 2: Library vs application patterns
    println!("\nExample 2: Library function with typed report");
    if let Err(report) = library_function_example() {
        println!("Library error (Report<DatabaseError>):");
        println!("{report}\n");
    }

    println!("Example 3: Application function with dynamic report");
    if let Err(report) = application_function_example() {
        println!("Application error (Report<dyn Any>):");
        println!("{report}");
    }
}
