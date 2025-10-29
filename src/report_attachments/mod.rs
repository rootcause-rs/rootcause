//! Collections of report attachments.
//!
//! This module provides types and functionality for managing collections of
//! report attachments.

mod iter;
mod owned;

pub use self::{
    iter::{ReportAttachmentsIntoIter, ReportAttachmentsIter},
    owned::ReportAttachments,
};
