//! Marker types and traits for defining ownership and thread-safety semantics.
//!
//! This module provides the type-level markers that control how reports and
//! attachments behave with respect to ownership, cloning, and thread safety in
//! the rootcause library. These markers are used as generic parameters to
//! enforce compile-time guarantees about how data can be accessed and shared.
//!
//! # Ownership Markers
//!
//! Ownership markers control whether reports can be mutated and cloned:
//!
//! - [`Mutable`]: This [`Report`] has unique ownership and can be mutated
//! - [`Cloneable`]: This [`Report`]/[`ReportRef`] can be cloned but not mutated
//!   (shared ownership)
//! - [`Uncloneable`]: This [`ReportRef`] cannot be cloned
//!
//! [`Report`]: crate::Report
//! [`ReportRef`]: crate::ReportRef
//!
//! # Thread Safety Markers
//!
//! Thread safety markers control whether reports can be sent between threads:
//!
//! - [`SendSync`]: The report and its contents are `Send + Sync` (thread-safe)
//! - [`Local`]: The report is not thread-safe and cannot cross thread
//!   boundaries
//!
//! # Examples
//!
//! ## Creating reports with different ownership semantics:
//!
//! ```
//! use rootcause::{markers, prelude::*};
//!
//! // Mutable report - can be modified
//! let mut report: Report<String, markers::Mutable> = report!("Error".to_string());
//! let report = report.attach("Additional context"); // attach consumes and returns the report
//!
//! // Cloneable report - can be cloned but not modified
//! let cloneable: Report<String, markers::Cloneable> = report.into_cloneable();
//! let cloned = cloneable.clone();
//! ```
//!
//! ## Working with thread safety:
//!
//! ```
//! use std::rc::Rc;
//!
//! use rootcause::{markers, prelude::*};
//!
//! // Thread-safe report (default)
//! let thread_safe: Report<String, markers::Mutable, markers::SendSync> =
//!     report!("Thread-safe error".to_string());
//!
//! // Local-only report (cannot be sent between threads)
//! let local_data = Rc::new("Not thread-safe".to_string());
//! let local_report: Report<Rc<String>, markers::Mutable, markers::Local> = report!(local_data);
//! ```

use core::any::Any;

/// Marker type indicating that a report is the unique owner of its context and
/// attachments. This allows mutating the report, for instance by attaching new
/// attachments or modifying the context.
///
/// This is the default ownership marker for reports.
///
/// # Examples
/// ```
/// use rootcause::prelude::*;
/// let mut report: Report<String, markers::Mutable> = report!("An error occurred".to_string());
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Mutable;

/// Marker type indicating that a report or report reference can be cloned, but
/// does not allow mutating as there may be other references to the same report.
///
/// # Examples
/// ```
/// use rootcause::{ReportRef, prelude::*};
/// let report: Report<String, markers::Cloneable> =
///     report!("An error occurred".to_string()).into_cloneable();
/// let report2 = report.clone();
/// let report_ref: ReportRef<String, markers::Cloneable> = report.as_ref();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Cloneable;

/// Marker type indicating that a report reference cannot be cloned.
///
/// This is used when taking a reference to a [`Mutable`] report, but it is also
/// possible to convert a `ReportRef<C, Cloneable>` into a `ReportRef<C,
/// Uncloneable>`.
///
/// # Examples
/// ```
/// use rootcause::{ReportRef, prelude::*};
/// let mut report: Report<String, markers::Mutable> = report!("An error occurred".to_string());
/// let report_ref: ReportRef<String, markers::Uncloneable> = report.as_ref();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Uncloneable;

/// Marker type indicating that a report can be sent across threads and shared
/// between threads. This requires that the context and all attachments
/// are `Send + Sync`.
///
/// # Examples
/// ```
/// use rootcause::prelude::*;
/// let report: Report<String, markers::Mutable, markers::SendSync> =
///     report!("An error occurred".to_string());
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct SendSync;

/// Marker type indicating that a report is not `Send` or `Sync`, and that it
/// cannot be sent or shared between threads.
///
/// # Examples
/// ```
/// use rootcause::prelude::*;
/// struct NotSendSync {
///     data: std::rc::Rc<String>,
/// }
/// let report: Report<NotSendSync, markers::Mutable, markers::Local> = report!(NotSendSync {
///     data: std::rc::Rc::new("An error occurred".to_string())
/// });
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Local;

mod sealed_report_ownership_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for Mutable {}
    impl Sealed for Cloneable {}
}

mod sealed_report_ref_ownership_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for Cloneable {}
    impl Sealed for Uncloneable {}
}

mod sealed_send_sync_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for SendSync {}
    impl Sealed for Local {}
}

mod sealed_context_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl<C: 'static> Sealed for C {}
    impl Sealed for dyn Any {}
}

/// Marker trait for types that can be used as the context in a report or as an
/// attachment.
///
/// This trait is implemented for all typed that are `Sized + 'static`.
/// Additionally it is implemented for `dyn Any`. This `dyn Any` only is used as
/// a marker for a type-erased value, it does not mean that we create a `Box<dyn
/// Any>` or similar behind the scenes.
///
/// In particular, converting from a concrete type to a `dyn Any` is a zero-cost
/// operation as it does not actually change the underlying representation at
/// all.
pub trait ObjectMarker: 'static + sealed_context_marker::Sealed {}
impl<T> ObjectMarker for T where T: 'static {}
impl ObjectMarker for dyn Any {}

/// Marker trait for types that can be used as the ownership marker for a
/// report.
///
/// This trait is implemented for [`Mutable`] and [`Cloneable`].
///
/// - [`Mutable`] indicates that the report is the unique owner of its context
///   and attachments. This allows calling methods such as [`Report::attach`]
///   and [`Report::as_mut`].
/// - [`Cloneable`] indicates that the report can be cloned, but does not allow
///   mutating as there may be other references to the same report.
///
/// [`Report::attach`]: crate::Report::attach
/// [`Report::as_mut`]: crate::Report::as_mut
///
/// The associated type `RefMarker` indicates the marker to use when taking a
/// reference to the report. For [`Mutable`] this is [`Uncloneable`], for
/// [`Cloneable`] it is [`Cloneable`].
pub trait ReportOwnershipMarker: sealed_report_ownership_marker::Sealed {
    /// The corresponding reference ownership marker.
    ///
    /// This is either [`Uncloneable`] (for [`Mutable`]) or [`Cloneable`] (for
    /// [`Cloneable`]). It is used to specify the ownership semantics when
    /// taking a reference to a report.
    ///
    /// It is used for instance when calling
    /// [`Report::as_ref`](crate::Report::as_ref), which converts a
    /// `Report<C, O>` into a `Report<C, O::RefMarker>`.
    type RefMarker: ReportRefOwnershipMarker;
}
impl ReportOwnershipMarker for Mutable {
    type RefMarker = Uncloneable;
}
impl ReportOwnershipMarker for Cloneable {
    type RefMarker = Cloneable;
}

/// Marker trait for types that can be used as the reference ownership marker
/// for a [`ReportRef`](crate::ReportRef).
///
/// This trait is implemented for [`Cloneable`] and [`Uncloneable`].
///
/// - [`Cloneable`] indicates that the report reference can be cloned.
/// - [`Uncloneable`] indicates that the report reference cannot be cloned. This
///   is used when taking a reference to a [`Mutable`] report, but it is also
///   possible to convert a `ReportRef<C, Cloneable>` into a `ReportRef<C,
///   Uncloneable>`
pub trait ReportRefOwnershipMarker: sealed_report_ref_ownership_marker::Sealed {}
impl ReportRefOwnershipMarker for Cloneable {}
impl ReportRefOwnershipMarker for Uncloneable {}

/// Marker trait for types that can be used as the thread-safety marker for a
/// report.
///
/// This trait is implemented for [`SendSync`] and [`Local`].
/// - [`SendSync`] indicates that the report can be sent across threads and
///   shared between threads. This requires that the context and all attachments
///   are `Send + Sync`.
/// - [`Local`] indicates that the report is not `Send` or `Sync`, and that it
///   cannot be sent or shared between threads.
pub trait ThreadSafetyMarker: sealed_send_sync_marker::Sealed {}
impl ThreadSafetyMarker for SendSync {}
impl ThreadSafetyMarker for Local {}

/// Marker trait for types that can be used as the context or attachment type
/// for a report with the given thread-safety marker.
///
/// - For `T = Local`, this is implemented for all types that implement
///   `ObjectMarker`.
/// - For `T = SendSync`, this is implemented for all types that implement
///   `ObjectMarker` and are also `Send + Sync`.
pub trait ObjectMarkerFor<T: ThreadSafetyMarker>: ObjectMarker {}
impl<T> ObjectMarkerFor<Local> for T where T: ObjectMarker {}
impl<T> ObjectMarkerFor<SendSync> for T where T: ObjectMarker + Send + Sync {}
impl ObjectMarkerFor<Local> for dyn Any {}
impl ObjectMarkerFor<SendSync> for dyn Any {}
