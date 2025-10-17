//! Built-in attachment collectors for automatically gathering debug information.
//!
//! This module provides attachment collectors that automatically gather useful debugging
//! information when reports are created. These collectors are typically registered as
//! report creation hooks to automatically attach relevant data to every report.
//!
//! ## Available Collectors
//!
//! - **[`location`]**: Captures the source code location where the report was created
//! - **[`backtrace`]** (requires `backtrace` feature): Captures a stack backtrace
//!
//! ## Usage
//!
//! These collectors are typically registered during application initialization:
//!
//! ```rust
//! use rootcause::hooks::{register_report_creation_hook, attachment_collectors::location::LocationCollector};
//!
//! // Register location collection for all reports
//! register_report_creation_hook(LocationCollector);
//! ```
//!
//! The collected attachments will automatically appear in formatted reports with
//! appropriate formatting determined by their respective handlers.

#[cfg(feature = "backtrace")]
pub mod backtrace;
pub mod location;
