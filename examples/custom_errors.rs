//! Creating custom errors with `report!()` and `bail!()`.
//!
//! **Run this example:** `cargo run --example custom_errors`
//!
//! In `basic.rs`, you learned to wrap external errors with `.context()` and
//! `.attach()`. This example shows how to create your own errors from scratch.
//!
//! Two approaches:
//! 1. **Simple validation**: Use `report!()` for quick error messages
//! 2. **Structured errors**: Define custom types with `report!(YourType)`
//!
//! Bonus: `bail!()` is a convenience macro that's shorthand for `return
//! Err(report!(...).into())`
//!
//! **What's next?**
//! - Want to understand type preservation? → `typed_reports.rs`
//! - Need lazy evaluation for performance? → `lazy_evaluation.rs`
//! - See all examples? → `examples/README.md`

use rootcause::prelude::*;

// Use report!() to create errors from scratch
fn validate_email(email: &str) -> Result<(), Report> {
    if !email.contains('@') {
        return Err(report!("Invalid email format"));
    }
    if email.len() < 3 {
        return Err(report!("Email too short: {}", email));
    }
    Ok(())
}

// report!() composes with .attach() and .context() just like external errors
fn validate_user_input(email: &str, age: i32) -> Result<(), Report> {
    validate_email(email).context("Email validation failed")?;

    if age < 0 || age > 150 {
        return Err(report!("Age out of valid range: {}", age));
    }
    Ok(())
}

// bail!() is shorthand for: return Err(report!(...).into())
fn validate_password(password: &str) -> Result<(), Report> {
    if password.len() < 8 {
        bail!("Password too short: minimum 8 characters");
    }
    Ok(())
}

// Custom error types for structured, domain-specific errors
#[derive(Debug)]
enum OrderError {
    InvalidQuantity { min: i32, max: i32, actual: i32 },
    InvalidDiscount { reason: String },
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OrderError::InvalidQuantity { min, max, actual } => {
                write!(f, "Quantity {actual} outside valid range [{min}, {max}]")
            }
            OrderError::InvalidDiscount { reason } => {
                write!(f, "Invalid discount: {reason}")
            }
        }
    }
}

impl std::error::Error for OrderError {}

// All error types compose naturally - mix report!(), bail!(), and custom types
fn validate_order(email: &str, quantity: i32, discount_percent: f32) -> Result<(), Report> {
    // report!() errors compose with .context()
    validate_email(email).context("Customer email validation failed")?;

    // Custom type errors
    if !(1..=100).contains(&quantity) {
        bail!(OrderError::InvalidQuantity {
            min: 1,
            max: 100,
            actual: quantity,
        });
    }

    if discount_percent < 0.0 || discount_percent > 50.0 {
        bail!(OrderError::InvalidDiscount {
            reason: format!("{discount_percent}% exceeds maximum allowed discount of 50%"),
        });
    }

    Ok(())
}

fn main() {
    println!("Creating errors with report!():\n");
    if let Err(report) = validate_user_input("invalid-email", 25) {
        eprintln!("{report}\n");
    }

    println!("Using bail!() as shorthand:\n");
    if let Err(report) = validate_password("short") {
        eprintln!("{report}\n");
    }

    println!("Composing different error types:\n");
    if let Err(report) = validate_order("invalid-email", 150, 60.0) {
        eprintln!("{report}\n");
    }
}
