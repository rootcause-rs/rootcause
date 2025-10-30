//! Commonly used items for convenient importing.
//!
//! The prelude module re-exports the most frequently used types, traits, and macros
//! from the rootcause library. This allows you to import everything you need with a
//! single use statement.
//!
//! # Usage
//!
//! ```rust
//! use rootcause::prelude::*;
//!
//! fn divide(a: i32, b: i32) -> Result<i32, Report> {
//!     if b == 0 {
//!         bail!("cannot divide by zero");
//!     }
//!     Ok(a / b)
//! }
//!
//! fn main() {
//!     let result: Result<i32, Report> = divide(10, 2);
//!     assert_eq!(result.unwrap(), 5);
//! }
//! ```
//!
//! # What's Included
//!
//! This prelude includes:
//!
//! - **[`Report`]**: The main error reporting type
//! - **[`ResultExt`]**: Extension methods for `Result` types
//! - **[`IteratorExt`]**: Extension methods for iterators
//! - **[`report!`]** and **[`bail!`]**: Macros for creating and returning errors
//! - **[`handlers`]**: Built-in error handlers for common scenarios
//! - **[`markers`]**: Type markers for controlling report behavior
//! - **[`report_attachment!`]**: Macro for attaching contextual data
//! - **[`Any`]**: Re-exported from `core::any` for dynamic typing
//!
//! # When to Use the Prelude
//!
//! Use the prelude when you need standard error handling functionality without
//! writing multiple import statements. For more specialized needs, import specific
//! items directly from their respective modules.

pub use core::any::Any;

pub use crate::{
    Report, bail, handlers, iterator_ext::IteratorExt, markers, report, report_attachment,
    result_ext::ResultExt,
};
