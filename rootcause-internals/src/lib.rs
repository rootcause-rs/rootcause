#![no_std]
#![forbid(
    missing_docs,
    clippy::alloc_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::missing_safety_doc,
    clippy::missing_docs_in_private_items,
    clippy::undocumented_unsafe_blocks,
    clippy::multiple_unsafe_ops_per_block,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::broken_intra_doc_links,
    missing_copy_implementations,
    unused_doc_comments
)]
#![allow(rustdoc::private_intra_doc_links)]
//! Internal implementation crate for [`rootcause`].
//!
//! # Overview
//!
//! This crate contains the low-level, type-erased data structures and unsafe
//! operations that power the [`rootcause`] error reporting library. It provides
//! the foundation for zero-cost type erasure through vtable-based dispatch.
//!
//! **This crate is an implementation detail.** No semantic versioning guarantees
//! are provided. Users should depend on the [`rootcause`] crate, not this one.
//!
//! # Architecture
//!
//! The crate is organized around two parallel type hierarchies for attachments
//! and reports:
//!
//! - **[`attachment`]**: Type-erased attachment storage
//!   - [`RawAttachment`]: Owned attachment with [`Box`]-based allocation
//!   - [`RawAttachmentRef`]: Borrowed reference to an attachment
//!   - [`AttachmentData`]: `#[repr(C)]` wrapper enabling field access on erased types
//!   - [`AttachmentVtable`]: Function pointers for type-erased dispatch
//!
//! - **[`report`]**: Type-erased report storage (similar structure)
//!   - [`RawReport`]: Owned report with [`Arc`]-based allocation
//!   - [`RawReportRef`]/[`RawReportMut`]: Borrowed references (shared/mutable)
//!   - [`ReportData`]: `#[repr(C)]` wrapper for field access
//!   - [`ReportVtable`]: Function pointers for dispatch
//!
//! - **[`handlers`]**: Trait definitions for formatting and behavior
//!   - [`ContextHandler`]: Defines how error contexts are formatted
//!   - [`AttachmentHandler`]: Defines how attachments are formatted
//!
//! # Safety Strategy
//!
//! Type erasure requires careful handling to maintain Rust's type safety
//! guarantees. When we erase a type like `AttachmentData<MyError>` to
//! `AttachmentData<Erased>`, we must ensure that the vtable function pointers
//! still match the actual concrete type stored in memory.
//!
//! This crate maintains safety through:
//!
//! - **Module-based encapsulation**: Safety-critical types keep fields
//!   module-private, making invariants locally verifiable within a single file
//! - **`#[repr(C)]` layout**: Enables safe field projection on type-erased
//!   pointers without constructing invalid references
//! - **Documented vtable contracts**: Each vtable method specifies exactly when
//!   it can be safely called
//!
//! See the individual module documentation ([`attachment`], [`report`]) for
//! detailed explanations of how these patterns are applied.
//!
//! [`rootcause`]: https://docs.rs/rootcause/latest/rootcause/
//! [`AttachmentData`]: attachment::data::AttachmentData
//! [`AttachmentVtable`]: attachment::vtable::AttachmentVtable
//! [`ReportData`]: report::data::ReportData
//! [`ReportVtable`]: report::vtable::ReportVtable
//! [`ContextHandler`]: handlers::ContextHandler
//! [`AttachmentHandler`]: handlers::AttachmentHandler
//! [`Box`]: alloc::boxed::Box
//! [`Arc`]: triomphe::Arc

extern crate alloc;

mod attachment;
pub mod handlers;
mod report;
mod util;

pub use attachment::{RawAttachment, RawAttachmentRef};
pub use report::{RawReport, RawReportMut, RawReportRef};
