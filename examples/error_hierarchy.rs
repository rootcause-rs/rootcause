//! Converting library errors into application error hierarchies.
//!
//! **Run this example:** `cargo run --example error_hierarchy`
//!
//! When building applications, you often need to wrap errors from external
//! libraries in your own error types. This example shows the recommended
//! patterns for converting between error types while preserving context and
//! debugging information.
//!
//! ## Key Concepts
//!
//! 1. **`context_to()`** - The recommended approach using `ReportConversion`
//!    trait
//! 2. **`context_transform()`** - Lightweight wrapping (preserves original
//!    structure)
//! 3. **`context_transform_nested()`** - Creates new layer with fresh hook data
//!
//! ## When to Use Each Method
//!
//! **Use `context_to()` + `ReportConversion` trait for:**
//! - Reusable conversion patterns you'll use multiple times
//! - Library boundaries where you want a standard conversion
//! - Clean, ergonomic API for your application code
//!
//! **Inside your `ReportConversion` implementation, choose:**
//! - `context_transform()` - When wrapping is just a type change (no semantic
//!   meaning)
//! - `context_transform_nested()` - When crossing major boundaries (want fresh
//!   hooks)
//! - `context()` - When you want to add a descriptive message as a new layer
//!
//! **Prerequisites:** Understanding typed reports (`typed_reports.rs`) and
//! automatic conversions (`error_coercion.rs`).
//!
//! **What's next?** See all examples → `examples/README.md`

use std::io;

use rootcause::{ReportConversion, markers, prelude::*};

// ============================================================================
// SETUP: Define Application Error Hierarchy
// ============================================================================

/// Application-level error enum that wraps various library errors.
///
/// This is the error type your application code will work with. It wraps
/// specific errors from external libraries (IO, parsing, serialization, etc.)
/// into a unified hierarchy.
#[derive(Debug)]
enum AppError {
    IoError(io::Error),
    ParseError(std::num::ParseIntError),
    ValidationError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::IoError(e) => write!(f, "IO error: {e}"),
            AppError::ParseError(e) => write!(f, "Parse error: {e}"),
            AppError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

// ============================================================================
// PATTERN 1: context_transform() - Lightweight Wrapping
// ============================================================================
//
// Use when: The wrapping is just a type change with no semantic meaning.
// The error is mechanically wrapped in your enum but represents the same
// logical failure point.
//
// Benefits:
// - Preserves original report structure (no extra nesting)
// - Keeps original hook data (backtraces, locations, etc.)
// - Minimal overhead
//
// Trade-offs:
// - No fresh hook data at the conversion point
// - Can't add additional context as a new layer

/// Implements lightweight conversion for IO errors.
///
/// Since IO errors are straightforward and don't cross major abstraction
/// boundaries, we use `context_transform()` to simply wrap them in our
/// AppError enum without creating additional nesting.
impl<T> ReportConversion<io::Error, markers::Mutable, T> for AppError
where
    AppError: markers::ObjectMarkerFor<T>,
{
    fn convert_report(
        report: Report<io::Error, markers::Mutable, T>,
    ) -> Report<Self, markers::Mutable, T> {
        // Lightweight wrapping - just changes the type, preserves structure
        report.context_transform(AppError::IoError)
    }
}

// ============================================================================
// PATTERN 2: context_transform_nested() - Heavyweight Transformation
// ============================================================================
//
// Use when: The conversion marks a significant boundary where you want fresh
// hook data (like a new backtrace if using rootcause-backtrace).
//
// Benefits:
// - Captures fresh hook data at the transformation point
// - Shows both original error location AND where it was wrapped
// - Useful for debugging across abstraction boundaries
//
// Trade-offs:
// - Creates an additional nesting level in the report
// - Original context becomes preformatted (loses type information)

/// Implements heavyweight conversion for parse errors.
///
/// Parse errors often cross abstraction boundaries (e.g., parsing user input
/// vs. parsing config files), so we use `context_transform_nested()` to
/// capture fresh hook data at the conversion point. This helps track where
/// the error was wrapped, not just where it originated.
impl<T> ReportConversion<std::num::ParseIntError, markers::Mutable, T> for AppError
where
    AppError: markers::ObjectMarkerFor<T>,
    rootcause::preformatted::PreformattedContext: markers::ObjectMarkerFor<T>,
{
    fn convert_report(
        report: Report<std::num::ParseIntError, markers::Mutable, T>,
    ) -> Report<Self, markers::Mutable, T> {
        // Heavyweight wrapping - creates new layer with fresh hooks
        report.context_transform_nested(AppError::ParseError)
    }
}

// ============================================================================
// PATTERN 3: context() - Adding Descriptive Context
// ============================================================================
//
// Use when: You want to add a descriptive message explaining what operation
// failed, wrapping the entire original error as a child.
//
// Benefits:
// - Adds meaningful explanation of the operation
// - Creates clear semantic layers in error chain
// - Runs creation hooks for the new context
//
// Trade-offs:
// - Creates additional nesting (by design)
// - Might be verbose if overused

/// Alternative conversion that adds descriptive context.
///
/// This isn't always done in ReportConversion (you might use it directly at
/// call sites), but it's shown here to contrast with the transform methods.
#[allow(dead_code)]
fn convert_with_context<T>(
    report: Report<std::num::ParseIntError, markers::Mutable, T>,
) -> Report<AppError, markers::Mutable, T>
where
    AppError: markers::ObjectMarkerFor<T>,
{
    // Adds descriptive message as new layer
    report.context(AppError::ValidationError(
        "Failed to parse user input".to_string(),
    ))
}

// ============================================================================
// DEMONSTRATION: Using context_to() in Application Code
// ============================================================================

/// Reads a number from a file and validates it.
///
/// This function demonstrates how clean your application code becomes when
/// you use `context_to()`. The error conversions happen automatically via
/// the ReportConversion trait implementations above.
fn read_and_parse_number(path: &str) -> Result<i32, Report<AppError>> {
    // .context_to() uses the IoError ReportConversion (context_transform)
    let contents = std::fs::read_to_string(path).context_to()?;

    // .context_to() uses the ParseError ReportConversion (context_transform_nested)
    let number: i32 = contents.trim().parse().context_to()?;

    // Direct AppError creation (not a conversion)
    if number < 0 {
        return Err(report!(AppError::ValidationError(
            "Number must be non-negative".to_string()
        ))
        .attach(format!("Got: {number}")));
    }

    Ok(number)
}

/// Demonstrates the difference in output between transformation approaches.
fn demonstrate_approaches() {
    println!("=== Comparing Transformation Approaches ===\n");

    // Approach 1: context_transform() - Lightweight
    println!("1. context_transform() - Lightweight wrapping:");
    println!("   (Used for IO errors via ReportConversion)\n");

    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let io_report: Report<io::Error> = report!(io_error).attach("Path: /config.txt");

    // This uses the ReportConversion implementation above
    let app_report: Report<AppError> = io_report.context_to();
    println!("{app_report}\n");

    println!("   Notice: Single layer, original structure preserved\n");
    println!("{}\n", "=".repeat(70));

    // Approach 2: context_transform_nested() - Heavyweight
    println!("2. context_transform_nested() - Creates new layer:");
    println!("   (Used for parse errors via ReportConversion)\n");

    // Simulate a parse error
    let parse_result: Result<i32, _> = "not_a_number".parse();
    if let Err(parse_error) = parse_result {
        let parse_report: Report<std::num::ParseIntError> =
            report!(parse_error).attach("Input: not_a_number");

        // This uses the ReportConversion implementation above
        let app_report: Report<AppError> = parse_report.context_to();
        println!("{app_report}\n");

        println!("   Notice: Two layers - wrapped error preserves original context\n");
        println!("   If using rootcause-backtrace, you'd see backtraces at both layers\n");
    }
    println!("{}\n", "=".repeat(70));

    // Approach 3: context() - Descriptive message
    println!("3. context() - Adds descriptive message:");
    println!("   (Typically used directly, not in ReportConversion)\n");

    let parse_result: Result<i32, _> = "also_not_a_number".parse();
    if let Err(parse_error) = parse_result {
        let report_with_context: Report<AppError> = report!(parse_error)
            .attach("Input: also_not_a_number")
            .context(AppError::ValidationError(
                "Failed to parse user input".to_string(),
            ));

        println!("{report_with_context}\n");

        println!("   Notice: Descriptive message as top-level context\n");
    }
}

/// Demonstrates realistic usage in a complete application flow.
fn realistic_example() {
    println!("=== Realistic Application Flow ===\n");

    // Try to read and process a number
    match read_and_parse_number("/nonexistent/config.txt") {
        Ok(number) => println!("Successfully processed: {number}"),
        Err(report) => {
            println!("Application error occurred:\n{report}\n");

            // You can pattern match on the typed error if needed
            match report.current_context() {
                AppError::IoError(_) => {
                    println!("→ This was an IO error - maybe retry or use default config")
                }
                AppError::ParseError(_) => {
                    println!("→ This was a parse error - maybe show user a helpful message")
                }
                AppError::ValidationError(_) => {
                    println!("→ This was a validation error - maybe prompt for new input")
                }
            }
        }
    }
}

fn main() {
    demonstrate_approaches();
    println!("\n{}\n", "=".repeat(70));
    realistic_example();

    println!(
        "\n{}\n\
         Summary:\n\
         \n\
         1. Define your AppError enum to wrap library errors\n\
         2. Implement ReportConversion for each library error type\n\
         3. Choose transformation method based on your needs:\n\
         \n\
         • context_transform() → Lightweight, preserves structure\n\
         • context_transform_nested() → Creates layer, fresh hooks\n\
         • context() → Adds descriptive message\n\
         \n\
         4. Use .context_to() in your code - conversions happen automatically!\n\
         \n\
         What's next?\n\
         • See how ? coerces types → error_coercion.rs\n\
         • Pattern match typed errors → typed_reports.rs\n\
         • All examples → examples/README.md\n",
        "=".repeat(70)
    );
}
