//! Collections of reports.
//!
//! This module provides types and functionality for managing collections of
//! reports.

mod iter;
mod owned;

pub use self::{
    iter::{ReportCollectionIntoIter, ReportCollectionIter},
    owned::ReportCollection,
};
