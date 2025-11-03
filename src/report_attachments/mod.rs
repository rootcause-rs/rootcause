//! Collections of report attachments.
//!
//! This module provides the [`ReportAttachments`] type for managing collections
//! of attachments that can be added to reports. Attachments provide additional
//! context or debugging information beyond the main error chain.
//!
//! # Overview
//!
//! The [`ReportAttachments`] type is similar to a `Vec<ReportAttachment<dyn
//! Any, T>>` but provides a more convenient API for working with collections of
//! type-erased attachments. It includes methods for:
//!
//! - Adding and removing attachments with [`push`] and [`pop`]
//! - Iterating over attachments with [`iter`]
//! - Converting between thread safety markers with [`into_local`]
//!
//! # Examples
//!
//! ```
//! use rootcause::{
//!     report_attachment::ReportAttachment,
//!     report_attachments::ReportAttachments,
//! };
//!
//! let mut attachments = ReportAttachments::new_sendsync();
//! attachments.push(ReportAttachment::new("debug info").into_dyn_any());
//! attachments.push(ReportAttachment::new(42).into_dyn_any());
//!
//! assert_eq!(attachments.len(), 2);
//!
//! for attachment in attachments.iter() {
//!     println!("Attachment type: {:?}", attachment.inner_type_id());
//! }
//! ```
//!
//! [`push`]: ReportAttachments::push
//! [`pop`]: ReportAttachments::pop
//! [`iter`]: ReportAttachments::iter
//! [`into_local`]: ReportAttachments::into_local

mod iter;
mod owned;

pub use self::{
    iter::{ReportAttachmentsIntoIter, ReportAttachmentsIter},
    owned::ReportAttachments,
};
