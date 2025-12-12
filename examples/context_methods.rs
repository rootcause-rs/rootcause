//! Comparing context transformation methods.
//!
//! **Run this example:** `cargo run --example context_methods`
//!
//! This example compares four methods for transforming report contexts:
//!
//! - **`context()`** - Wraps report as child under new context
//! - **`context_to()`** - Uses `ReportConversion` trait implementation
//! - **`context_transform()`** - Changes context type in-place
//! - **`context_transform_nested()`** - Preformats and wraps as child
//!
//! The focus is on understanding **what each method does to the report
//! structure** and **what information is preserved or lost**.

use rootcause::{ReportConversion, markers, preformatted::PreformattedContext, prelude::*};

#[derive(Debug)]
enum AppError {
    Parse(std::num::ParseIntError),
    Other,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Parse(e) => write!(f, "Parse error: {e}"),
            AppError::Other => write!(f, "Error occurred"),
        }
    }
}

impl<T> ReportConversion<std::num::ParseIntError, markers::Mutable, T> for AppError
where
    AppError: markers::ObjectMarkerFor<T>,
    rootcause::preformatted::PreformattedContext: markers::ObjectMarkerFor<T>,
{
    fn convert_report(
        report: Report<std::num::ParseIntError, markers::Mutable, T>,
    ) -> Report<Self, markers::Mutable, T> {
        report.context_transform_nested(AppError::Parse)
    }
}

fn parse_error(input: &str) -> Report<std::num::ParseIntError> {
    let parsed = input.parse::<i32>().unwrap_err();
    report!(parsed).attach(format!("input: {}", input))
}

fn main() {
    // context() - Creates new parent node, child type preserved
    println!("Using context():");
    let report1: Report<AppError> = parse_error("not_a_number").context(AppError::Other);
    println!("{report1}\n");
    assert_eq!(report1.iter_sub_reports().count(), 1);
    assert_eq!(
        report1.children().get(0).unwrap().current_context_type_id(),
        std::any::TypeId::of::<std::num::ParseIntError>()
    );

    // context_transform() - Single node, type changed in-place
    println!("Using context_transform():");
    let report2: Report<AppError> = parse_error("not_a_number").context_transform(AppError::Parse);
    println!("{report2}\n");
    assert_eq!(report2.iter_sub_reports().count(), 0);

    // context_transform_nested() - Creates new parent node, child preformatted (type lost)
    println!("Using context_transform_nested():");
    let report3: Report<AppError> =
        parse_error("not_a_number").context_transform_nested(AppError::Parse);
    println!("{report3}\n");
    assert_eq!(report3.iter_sub_reports().count(), 1);
    assert_eq!(
        report3.children().get(0).unwrap().current_context_type_id(),
        std::any::TypeId::of::<PreformattedContext>()
    );

    // context_to() - Uses ReportConversion impl (context_transform_nested in this example)
    println!("Using context_to():");
    let report4: Report<AppError> = parse_error("not_a_number").context_to::<AppError>();
    println!("{report4}\n");
}
