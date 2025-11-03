//! Collections of reports.
//!
//! This module provides the [`ReportCollection`] type for managing collections
//! of child reports. Collections are useful for aggregating multiple related
//! errors or representing parallel error paths.
//!
//! # Overview
//!
//! The [`ReportCollection`] type is similar to a `Vec<Report<C,
//! markers::Cloneable, T>>` but provides a more convenient API tailored for
//! error handling. It includes methods for:
//!
//! - Adding and removing reports with [`push`] and [`pop`]
//! - Iterating over reports with [`iter`]
//! - Creating parent reports with [`context`] and [`context_custom`]
//! - Converting between context and thread safety markers
//!
//! # Examples
//!
//! ## Creating and populating a collection
//!
//! ```
//! use rootcause::{report, report_collection::ReportCollection};
//!
//! let mut collection = ReportCollection::new();
//! collection.push(report!("Database connection failed").into_cloneable());
//! collection.push(report!("Cache initialization failed").into_cloneable());
//!
//! assert_eq!(collection.len(), 2);
//! ```
//!
//! ## Wrapping a collection with context
//!
//! ```
//! use rootcause::{report, report_collection::ReportCollection, Report};
//!
//! let collection: ReportCollection = [
//!     report!("Service A failed"),
//!     report!("Service B failed"),
//! ]
//! .into_iter()
//! .collect();
//!
//! let report: Report<&str> = collection.context("Multiple services unavailable");
//! println!("{}", report);
//! ```
//!
//! [`push`]: ReportCollection::push
//! [`pop`]: ReportCollection::pop
//! [`iter`]: ReportCollection::iter
//! [`context`]: ReportCollection::context
//! [`context_custom`]: ReportCollection::context_custom

mod iter;
mod owned;

pub use self::{
    iter::{ReportCollectionIntoIter, ReportCollectionIter},
    owned::ReportCollection,
};
