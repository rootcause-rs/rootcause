//! Deferred computation with `.attach_with()` and `.context_with()`.
//!
//! **Run this example:** `cargo run --example lazy_evaluation`
//!
//! In `basic.rs`, you learned `.context()` and `.attach()`. This example shows
//! when to use the `_with()` variants that defer computation.
//!
//! **Use `_with()` on Results when you need to compute the value:**
//! - Formatting with `format!()`
//! - Expensive computations that shouldn't run on success
//!
//! The closure only runs if the Result is Err, avoiding unnecessary work.
//!
//! **Don't use `_with()` when already constructing an error:**
//!
//! If you're using `bail!()` or `report!()`, you're already on the error path.
//! Just compute the value directly and use `.attach()` or `.context()`.
//!
//! **What's next?**
//! - Want to understand type preservation? → `typed_reports.rs`
//! - See all examples? → `examples/README.md`

use rootcause::prelude::*;

fn database_query(id: u32) -> Result<String, Report> {
    if id == 999 {
        bail!("Connection timeout");
    }
    Ok(format!("Record {id}"))
}

// Defer expensive formatting or other computations until actually needed
fn fetch_user_record(user_id: u32, db_host: &str, timestamp: u64) -> Result<String, Report> {
    let record = database_query(user_id).context_with(|| {
        format!("Failed to fetch user {user_id} from {db_host} at timestamp {timestamp}")
    })?;
    Ok(record)
}

fn validate_item(item: &str) -> Result<(), Report> {
    if item.is_empty() {
        bail!("Item cannot be empty");
    }
    if item.len() > 100 {
        bail!("Item exceeds maximum length");
    }
    Ok(())
}

// This can be extra important in loops as the savings compound
fn process_batch(items: &[&str]) -> Result<(), Report> {
    for (index, item) in items.iter().enumerate() {
        validate_item(item)
            .attach_with(|| format!("Item {index}: '{item}'"))
            .context("Batch processing failed")?;
    }
    Ok(())
}

// Anti-pattern: If you already have an error, it no longer makes sense to defer
fn validate_data(data: &str) -> Result<(), Report> {
    if data.is_empty() {
        // Don't do this:
        //
        //   return Err(report!("Empty data"))
        //     .attach_with(|| format!("Length: {}, Expected: >0", data.len()));
        //
        // Instead, just compute directly:
        return Err(report!("Empty data").attach(format!("Length: {}, Expected: >0", data.len())));
    }
    Ok(())
}

fn main() {
    println!("Deferring expensive formatting:\n");
    if let Err(report) = fetch_user_record(999, "db.example.com", 1234567890) {
        eprintln!("{report}\n");
    }

    println!("Avoiding unnecessary work in a loop:\n");
    let items = vec!["valid", "", "also valid"];
    if let Err(report) = process_batch(&items) {
        eprintln!("{report}\n");
    }

    println!("Already on error path - compute directly:\n");
    if let Err(report) = validate_data("") {
        eprintln!("{report}\n");
    }
}
