#![no_std]
#![forbid(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    clippy::multiple_unsafe_ops_per_block
)]
#![allow(rustdoc::private_intra_doc_links)]
//! Internal crate for the `rootcause` crate.
//!
//! This crate contains the core data structures used by the [`rootcause`] crate, and encapsulates most of the
//! unsafe operations needed to make it work.
//!
//! This crate is considered an implementation detail of the [`rootcause`] crate, and as such no semantic versioning
//! guarantees are made for this crate.
//!
//! [`rootcause`]: https://docs.rs/rootcause/latest/rootcause/

extern crate alloc;

mod attachment;
pub mod handlers;
mod report;
mod util;

pub use attachment::{RawAttachment, RawAttachmentRef};
pub use report::{RawReport, RawReportMut, RawReportRef};
