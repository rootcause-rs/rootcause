//! Type-erased report data structures.
//!
//! This module implements type erasure for error reports through vtable-based
//! dispatch. The design allows reports with any context type `C` to be stored
//! uniformly while maintaining the ability to format and inspect them.
//!
//! # Structure
//!
//! - [`data`]: Contains [`ReportData<C>`], the `#[repr(C)]` wrapper that
//!   pairs a context value with its vtable, children, and attachments
//! - [`raw`]: Contains [`RawReport`], [`RawReportRef`], and [`RawReportMut`] -
//!   the type-erased pointer types that users of this module interact with
//! - [`vtable`]: Contains [`ReportVtable`], the function pointer table for
//!   type-erased operations
//!
//! # Allocation Strategy
//!
//! Reports use `Arc<ReportData<C>>` for storage, enabling cheap cloning and
//! shared ownership. The `Arc` is converted to a raw pointer for type erasure,
//! then reconstructed when dropping or accessing with the concrete type.
//!
//! [`ReportData<C>`]: data::ReportData
//! [`ReportVtable`]: vtable::ReportVtable

mod data;
mod raw;
mod vtable;

pub use raw::{RawReport, RawReportMut, RawReportRef};
