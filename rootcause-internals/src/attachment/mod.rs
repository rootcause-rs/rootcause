//! Type-erased attachment data structures.
//!
//! This module implements type erasure for error report attachments through
//! vtable-based dispatch. The design allows attachments of any type `A` to be
//! stored uniformly while maintaining the ability to format and inspect them.
//!
//! # Structure
//!
//! - [`data`]: Contains [`AttachmentData<A>`], the `#[repr(C)]` wrapper that
//!   pairs an attachment value with its vtable
//! - [`raw`]: Contains [`RawAttachment`] and [`RawAttachmentRef`], the
//!   type-erased pointer types that users of this module interact with
//! - [`vtable`]: Contains [`AttachmentVtable`], the function pointer table for
//!   type-erased operations
//!
//! [`AttachmentData<A>`]: data::AttachmentData
//! [`AttachmentVtable`]: vtable::AttachmentVtable

pub(crate) mod data;
pub(crate) mod raw;
pub(crate) mod vtable;

pub use self::raw::{RawAttachment, RawAttachmentRef};
