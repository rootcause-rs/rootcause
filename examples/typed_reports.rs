//! Pattern matching on typed errors with `Report<YourError>`.
//!
//! By default, `report!()` creates `Report` (shorthand for `Report<Dynamic>`),
//! which can hold any error type—like anyhow. But when you need callers to
//! pattern match on specific error variants, use `Report<YourError>` instead.
//!
//! **Use `Report<YourError>` when:**
//! - Callers need to pattern match and handle specific error variants
//! - You want type-safe access via `.current_context()` without runtime checks
//!
//! **Use `Report` (dynamic) when:**
//! - Combining errors from different sources
//! - Callers just propagate errors without inspecting them
//!
//! Typed and dynamic reports work together seamlessly—use `.into_dynamic()`
//! or let `?` convert automatically.

use rootcause::prelude::*;

#[derive(Debug, Clone)]
enum DatabaseError {
    ConnectionLost,
    ConstraintViolation { constraint: String },
    NotFound,
}

impl core::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConnectionLost => write!(f, "Database connection lost"),
            Self::ConstraintViolation { constraint } => {
                write!(f, "Constraint violation: {constraint}")
            }
            Self::NotFound => write!(f, "Record not found"),
        }
    }
}

fn query_user(user_id: u32) -> Result<String, Report<DatabaseError>> {
    // Simulate different error types for demonstration
    let error = match user_id {
        1 => DatabaseError::ConnectionLost,
        2 => DatabaseError::ConstraintViolation {
            constraint: "unique_email".to_string(),
        },
        _ => DatabaseError::NotFound,
    };

    Err(report!(error).attach(format!("User ID: {user_id}")))
}

// Pattern match on Report<DatabaseError> to retry only transient errors
fn query_user_with_retry(user_id: u32) -> Result<String, Report> {
    const MAX_RETRIES: usize = 3;

    let mut attempt = 1;
    loop {
        match query_user(user_id) {
            Ok(user) => return Ok(user),
            Err(report) => {
                // .current_context() gives type-safe access to &DatabaseError
                let should_retry = match report.current_context() {
                    DatabaseError::ConnectionLost => true,
                    DatabaseError::ConstraintViolation { .. } | DatabaseError::NotFound => false,
                };

                if !(should_retry && attempt < MAX_RETRIES) {
                    return Err(report
                        .context(format!("Failed to query user after {attempt} attempts"))
                        .into_dynamic());
                }
            }
        }
        attempt += 1;
    }
}

fn main() {
    println!("Retrying transient errors:\n");
    if let Err(report) = query_user_with_retry(1) {
        eprintln!("{report}\n");
    }

    println!("Failing immediately on permanent errors:\n");
    if let Err(report) = query_user_with_retry(2) {
        eprintln!("{report}");
    }
}
