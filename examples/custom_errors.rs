//! Creating custom errors
//!
//! **Run this example:** `cargo run --example custom_errors`
//!
//! After basic.rs showed wrapping external errors with `.context()` and
//! `.attach()`, this example shows how to create errors from scratch when
//! there's no underlying error to wrap.
//!
//! Key concepts:
//! 1. `report!()` with string messages - for quick validation errors
//! 2. Custom error types - for structured, domain-specific errors
//! 3. Mixing approaches - combining report!(), custom types, and external
//!    errors
//!
//! **What's next?** Most users can stop here - you know enough to be
//! productive!
//! - Performance optimization? → `lazy_evaluation.rs` (`.attach_with()`,
//!   `.context_with()`)
//! - Type system deep dive? → `typed_reports.rs` (preserving error types)
//! - See all examples? → `examples/README.md`

use rootcause::prelude::*;

// ============================================================================
// PART 1: Creating Errors with report!()
// ============================================================================
// When you need a custom error but don't want to define a type

/// Simple validation using report!() for quick error messages.
fn validate_email(email: &str) -> Result<(), Report> {
    if !email.contains('@') {
        return Err(report!("Invalid email format"));
    }
    if email.len() < 3 {
        return Err(report!("Email too short"));
    }
    Ok(())
}

/// report!() works with .attach() to add debugging information.
fn validate_age(age: i32) -> Result<(), Report> {
    if age < 0 {
        return Err(report!("Age cannot be negative").attach(format!("Provided age: {age}")));
    }
    if age > 150 {
        return Err(report!("Age seems unrealistic")
            .attach(format!("Provided age: {age}"))
            .attach("Maximum reasonable age: 150"));
    }
    Ok(())
}

/// report!() errors can be wrapped with .context() just like external errors.
fn validate_user(email: &str, age: i32) -> Result<(), Report> {
    validate_email(email).context("Email validation failed")?;
    validate_age(age).context("Age validation failed")?;
    Ok(())
}

// ============================================================================
// PART 2: Custom Error Types
// ============================================================================
// For domain-specific errors with structure and behavior

/// A custom error type for configuration validation.
///
/// This gives you more structure than plain strings and allows
/// programmatic inspection of error details.
#[derive(Debug)]
struct ConfigError {
    field: String,
    expected: String,
    actual: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Invalid config field '{}': expected {}, got {}",
            self.field, self.expected, self.actual
        )
    }
}

impl std::error::Error for ConfigError {}

/// Use report!() with your custom error type.
fn validate_port(port: u16) -> Result<(), Report> {
    if port == 0 {
        return Err(report!(ConfigError {
            field: "port".to_string(),
            expected: "1-65535".to_string(),
            actual: "0".to_string(),
        })
        .into());
    }
    Ok(())
}

/// Another custom error for business logic validation.
#[derive(Debug)]
enum ValidationError {
    OutOfRange {
        field_name: String,
        min: i32,
        max: i32,
        actual: i32,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationError::OutOfRange {
                field_name,
                min,
                max,
                actual,
            } => {
                write!(
                    f,
                    "Field '{}' out of range: {} not in [{}, {}]",
                    field_name, actual, min, max
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

fn validate_quantity(quantity: i32) -> Result<(), Report> {
    if quantity < 1 || quantity > 100 {
        return Err(report!(ValidationError::OutOfRange {
            field_name: "quantity".to_string(),
            min: 1,
            max: 100,
            actual: quantity,
        })
        .into());
    }
    Ok(())
}

// ============================================================================
// PART 3: Mixing Approaches
// ============================================================================
// Real code combines external errors, report!(), and custom types

/// A realistic example showing all three approaches together.
fn process_order(user_email: &str, quantity: i32, config_port: u16) -> Result<(), Report> {
    // Quick validation with report!() - simple string message
    if user_email.is_empty() {
        return Err(report!("Email cannot be empty").attach("Field: user_email"));
    }

    // External error handling (from validate_email which uses report!())
    validate_email(user_email).context("User email validation failed")?;

    // Custom error type for business logic
    validate_quantity(quantity)
        .attach(format!(
            "Order details: email={}, qty={}",
            user_email, quantity
        ))
        .context("Order quantity validation failed")?;

    // Custom error type for configuration
    validate_port(config_port).context("Configuration validation failed")?;

    Ok(())
}

fn main() {
    println!("=== Creating Custom Errors Tutorial ===\n");
    println!("This example shows three ways to create errors:\n");

    println!("=== Part 1: report!() with String Messages ===\n");
    println!("Use case: Quick validation where you don't need structured error data\n");

    if let Err(report) = validate_user("invalid-email", 25) {
        eprintln!("{report}\n");
    }

    println!("{}\n", "=".repeat(70));
    println!("=== Part 1b: report!() with .attach() ===\n");
    println!("Use case: Quick validation + debugging information\n");

    if let Err(report) = validate_age(-5) {
        eprintln!("{report}\n");
    }

    println!("{}\n", "=".repeat(70));
    println!("=== Part 2: Custom Error Types (Struct) ===\n");
    println!("Use case: Domain-specific errors with structure and Display impl\n");

    if let Err(report) = validate_port(0) {
        eprintln!("{report}\n");
    }

    println!("{}\n", "=".repeat(70));
    println!("=== Part 2b: Custom Error Types (Enum) ===\n");
    println!("Use case: Multiple error variants with structured data\n");

    if let Err(report) = validate_quantity(150) {
        eprintln!("{report}\n");
    }

    println!("{}\n", "=".repeat(70));
    println!("=== Part 3: Mixing All Approaches ===\n");
    println!("Use case: Real-world code combining different error strategies\n");

    if let Err(report) = process_order("user@example.com", 150, 8080) {
        eprintln!("{report}\n");
    }

    println!("{}\n", "=".repeat(70));
    println!(
        "DECISION GUIDE - Which approach should you use?\n\
         \n\
         ✓ report!(\"message\") when:\n\
           • Quick validation in small functions\n\
           • Error message is simple and self-explanatory\n\
           • You don't need to programmatically inspect error details\n\
         \n\
         ✓ Custom error type when:\n\
           • You have domain-specific error categories\n\
           • You need structured data (fields, variants)\n\
           • Callers might pattern match on error details\n\
           • You want to implement custom Display/Debug\n\
         \n\
         ✓ External errors (from basic.rs):\n\
           • Wrapping std errors or library errors\n\
           • Use .context() and .attach() to add meaning\n\
         \n\
         All three approaches compose! Mix and match as needed.\n"
    );
}
