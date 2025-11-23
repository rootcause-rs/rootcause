//! Marker types and traits for defining ownership and thread-safety semantics.
//!
//! This module provides type-level markers that control how reports and
//! attachments behave with respect to ownership, cloning, and thread safety.
//! These markers are used as generic parameters in types like [`Report<C, O,
//! T>`](crate::Report) to enforce compile-time guarantees about how data can be
//! accessed and shared.
//!
//! # Ownership Markers
//!
//! Ownership markers control whether reports and report references can be
//! mutated and cloned.
//!
//! For owned reports ([`Report<C, O, T>`](crate::Report)), the ownership marker
//! `O` can be:
//! - [`Mutable`]: Unique ownership - the report can be mutated but not cloned
//! - [`Cloneable`]: Shared ownership - the report can be cloned but not mutated
//!
//! For report references ([`ReportRef<C, O>`](crate::ReportRef)), the ownership
//! marker `O` can be:
//! - [`Cloneable`]: Enables [`clone_arc`](crate::ReportRef::clone_arc) to get
//!   an owned report
//! - [`Uncloneable`]: Does not provide
//!   [`clone_arc`](crate::ReportRef::clone_arc)
//!
//! # Thread Safety Markers
//!
//! Thread safety markers control whether reports can be sent between threads or
//! shared across threads. These appear as the third type parameter (`T`) in
//! [`Report<C, O, T>`](crate::Report):
//!
//! - [`SendSync`]: The report and all its contents are `Send + Sync`, allowing
//!   the report to cross thread boundaries.
//! - [`Local`]: The report contains non-thread-safe data (like `Rc` or raw
//!   pointers) and cannot be sent between threads.
//!
//! # Examples
//!
//! ## Creating Reports with Different Ownership Semantics
//!
//! ```
//! use rootcause::prelude::*;
//!
//! // Mutable report - can be modified by adding context and attachments
//! let mut report: Report<String, markers::Mutable> = report!("Error".to_string());
//! let report: Report<String, markers::Mutable> = report.attach("Additional context");
//!
//! // Convert to cloneable report - can be cloned but not mutated
//! let cloneable: Report<String, markers::Cloneable> = report.into_cloneable();
//! let cloned: Report<String, markers::Cloneable> = cloneable.clone();
//! assert_eq!(format!("{}", cloneable), format!("{}", cloned));
//! ```
//!
//! ## Working with Thread Safety
//!
//! ```
//! use std::rc::Rc;
//!
//! use rootcause::prelude::*;
//!
//! // Thread-safe report with String (String is Send + Sync)
//! let thread_safe: Report<String, markers::Mutable, markers::SendSync> =
//!     report!("Thread-safe error".to_string());
//!
//! // Can be sent to another thread
//! std::thread::spawn(move || {
//!     println!("{}", thread_safe);
//! });
//!
//! // Local report with Rc (Rc is !Send + !Sync)
//! let local_data: Rc<String> = Rc::new("Not thread-safe".to_string());
//! let local_report: Report<Rc<String>, markers::Mutable, markers::Local> = report!(local_data);
//! // local_report cannot be sent to another thread - won't compile
//! ```

use core::any::Any;

use crate::ReportMut;

/// Marker type for owned reports with unique ownership.
///
/// This marker is used exclusively with [`Report<C, Mutable,
/// T>`](crate::Report) (not [`ReportRef`]). It indicates that
/// the report has unique ownership of its data, which allows mutation
/// operations but prevents cloning.
///
/// # Available Operations
///
/// With `Mutable` ownership, you can:
/// - Add attachments with [`attach`](crate::Report::attach)
/// - Add parent context with [`context`](crate::Report::context)
/// - Get mutable access with [`as_mut`](crate::Report::as_mut)
/// - Convert to [`Cloneable`] with
///   [`into_cloneable`](crate::Report::into_cloneable)
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
///
/// let report: Report<String, markers::Mutable> = report!("Database error".to_string());
///
/// // Can add attachments (consumes and returns the report)
/// let report: Report<String, markers::Mutable> = report.attach("connection timeout");
///
/// // Can add parent context
/// let report: Report<String, markers::Mutable> =
///     report.context("Failed to fetch user data".to_string());
///
/// // Cannot clone - Mutable reports don't implement Clone
/// // let cloned = report.clone(); // ❌ Won't compile
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Mutable;

/// Marker type for cloneable reports and report references.
///
/// This marker is used with both [`Report<C, Cloneable, T>`](crate::Report) and
/// [`ReportRef<C, Cloneable>`](crate::ReportRef). It indicates shared ownership
/// using reference counting (`Arc` internally), which allows cheap cloning but
/// prevents mutation.
///
/// # Usage with Report
///
/// For [`Report<C, Cloneable, T>`](crate::Report), this marker means the report
/// itself implements `Clone`, allowing you to cheaply clone the entire report
/// (shallow copy via `Arc`).
///
/// # Usage with ReportRef
///
/// For [`ReportRef<C, Cloneable>`](crate::ReportRef), the marker enables the
/// [`clone_arc`](crate::ReportRef::clone_arc) method, which clones the
/// underlying `Arc` to produce an owned [`Report<C, Cloneable,
/// T>`](crate::Report). Note that `ReportRef` itself is always `Copy` and
/// `Clone` regardless of the ownership marker - the `Cloneable`
/// marker specifically enables converting the reference back to an owned
/// report.
///
/// # When to Use
///
/// Use `Cloneable` reports when you need to:
/// - Share an error report across multiple code paths
/// - Store reports in collections that require `Clone`
/// - Return the same error from multiple places without deep copying
///
/// # Converting to Cloneable
///
/// Convert a [`Mutable`] report to `Cloneable` using
/// [`into_cloneable`](crate::Report::into_cloneable):
///
/// ```
/// use rootcause::prelude::*;
///
/// let report: Report<String, markers::Mutable> = report!("Error".to_string());
///
/// // Convert to cloneable
/// let cloneable: Report<String, markers::Cloneable> = report.into_cloneable();
///
/// // Now can clone cheaply (shallow clone via Arc)
/// let clone1: Report<String, markers::Cloneable> = cloneable.clone();
/// let clone2: Report<String, markers::Cloneable> = cloneable.clone();
/// ```
///
/// # Examples
///
/// Cloning owned reports:
///
/// ```
/// use rootcause::prelude::*;
///
/// fn process_error(error: Report<String, markers::Cloneable>) {
///     // Can clone the error to pass to multiple handlers
///     let for_logging = error.clone();
///     let for_metrics = error.clone();
///
///     println!("Logging: {}", for_logging);
///     println!("Metrics: {}", for_metrics);
/// }
///
/// let report: Report<String> = report!("An error occurred".to_string());
/// process_error(report.into_cloneable());
/// ```
///
/// Using `clone_arc` on report references:
///
/// ```
/// use rootcause::{ReportRef, prelude::*};
///
/// let report: Report<String, markers::Cloneable> = report!("Error".to_string()).into_cloneable();
///
/// // Get a reference (ReportRef is Copy, so this is cheap)
/// let report_ref: ReportRef<String, markers::Cloneable> = report.as_ref();
///
/// // Clone the underlying Arc to get an owned Report
/// let owned: Report<String, markers::Cloneable> = report_ref.clone_arc();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Cloneable;

/// Marker type for non-cloneable report references.
///
/// This marker is used exclusively with [`ReportRef<C,
/// Uncloneable>`](crate::ReportRef) (not [`Report`](crate::Report)). It
/// indicates that the reference does not provide the
/// [`clone_arc`](crate::ReportRef::clone_arc) method to obtain an owned report.
///
/// Note that `ReportRef` itself is always `Copy` and `Clone` - you can always
/// copy the reference itself. The `Uncloneable` marker only prevents cloning
/// the underlying `Arc` to get an owned `Report`.
///
/// # Common Uses
///
/// `Uncloneable` references typically arise in two situations:
///
/// 1. **Taking a reference to a `Mutable` report**: When you call
///    [`as_ref`](crate::Report::as_ref) on a `Report<C, Mutable>`, you get a
///    `ReportRef<C, Uncloneable>` because the underlying report has unique
///    ownership.
///
/// 2. **Explicitly restricting cloneability**: You can convert a `ReportRef<C,
///    Cloneable>` to `ReportRef<C, Uncloneable>` when you want to pass a
///    reference that explicitly cannot use `clone_arc`, ensuring the recipient
///    can only inspect the report without obtaining ownership. This can be
///    useful in APIs that need to accept both cloneable and uncloneable
///    references.
///
///
/// # Examples
///
/// Taking a reference to a `Mutable` report:
///
/// ```
/// use rootcause::{ReportRef, prelude::*};
///
/// let report: Report<String, markers::Mutable> = report!("An error occurred".to_string());
///
/// // Taking a reference to a Mutable report gives an Uncloneable reference
/// let report_ref: ReportRef<String, markers::Uncloneable> = report.as_ref();
///
/// // The reference itself can be copied (ReportRef is Copy)
/// let copy = report_ref;
///
/// // But you cannot clone the underlying Arc to get an owned Report
/// // let owned = report_ref.clone_arc(); // ❌ Method not available
/// ```
///
/// Explicitly converting to `Uncloneable`:
///
/// ```
/// use rootcause::{ReportRef, prelude::*};
///
/// let report: Report<String, markers::Cloneable> = report!("Error".to_string()).into_cloneable();
///
/// let cloneable_ref: ReportRef<String, markers::Cloneable> = report.as_ref();
///
/// // Convert to uncloneable to restrict the recipient's ability to clone
/// let uncloneable_ref: ReportRef<String, markers::Uncloneable> = cloneable_ref.into_uncloneable();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Uncloneable;

/// Marker type indicating that a report and all its contents are `Send + Sync`.
///
/// This is the default thread-safety marker for reports. When all the context
/// objects and attachments in a report implement `Send + Sync`, the report
/// itself can be safely sent to other threads and shared between threads.
///
/// # When to Use
///
/// Most standard types in Rust are `Send + Sync`, including:
/// - Primitive types (`i32`, `String`, `Vec`, etc.)
/// - Most standard library types
/// - Types explicitly designed for concurrent use
///
/// Use `SendSync` (the default) unless you have a specific need for
/// thread-local data.
///
/// # Examples
///
/// ```
/// use std::thread;
///
/// use rootcause::prelude::*;
///
/// // String is Send + Sync, so the report is too
/// let report: Report<String, markers::Mutable, markers::SendSync> =
///     report!("Thread-safe error".to_string());
///
/// // Can send to another thread
/// thread::spawn(move || {
///     println!("Error in thread: {}", report);
/// })
/// .join()
/// .unwrap();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct SendSync;

/// Marker type indicating that a report is not `Send` or `Sync`.
///
/// This marker is used when a report contains thread-local data that cannot be
/// safely sent between threads or shared across threads. Common examples
/// include `Rc`, raw pointers, or types that explicitly opt out of
/// `Send`/`Sync`.
///
/// # When to Use
///
/// Use `Local` when your error context or attachments contain:
/// - `Rc<T>` or `Weak<T>` (use `Arc<T>` for thread-safe alternative)
/// - Raw pointers (`*const T`, `*mut T`)
/// - Types that wrap thread-local storage
/// - Any type that is `!Send` or `!Sync`
///
/// # Converting to Local
///
/// You can convert a thread-safe report to a local one using
/// [`into_local`](crate::Report::into_local), or create a local report directly
/// when the context type is not `Send + Sync`.
///
/// # Examples
///
/// ```
/// use std::rc::Rc;
///
/// use rootcause::prelude::*;
///
/// // Rc is not Send or Sync, so the report must be Local
/// let local_data: Rc<String> = Rc::new("Thread-local error".to_string());
/// let report: Report<Rc<String>, markers::Mutable, markers::Local> = report!(local_data);
///
/// // This report cannot be sent to another thread
/// // std::thread::spawn(move || {
/// //     println!("{}", report); // ❌ Won't compile
/// // });
/// ```
///
/// Converting a thread-safe report to local:
///
/// ```
/// use std::rc::Rc;
///
/// use rootcause::prelude::*;
///
/// let report: Report<String> = report!("Error".to_string());
///
/// // Convert to local report so we can use thread-local data
/// let local_report: Report<String, markers::Mutable, markers::Local> = report.into_local();
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Local;

mod sealed_report_ownership_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for Mutable {}
    impl Sealed for Cloneable {}
}

/*
mod sealed_report_ref_ownership_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for Cloneable {}
    impl Sealed for Uncloneable {}
}
     */

/*
mod sealed_send_sync_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for SendSync {}
    impl Sealed for Local {}
}
     */

/*
mod sealed_context_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl<C: 'static> Sealed for C {}
    impl Sealed for dyn Any {}
}

/// Marker trait for types that can be used as context or attachment data.
///
/// This trait is automatically implemented for all `Sized + 'static` types, and
/// also for the special type `dyn Any` which is used to represent type-erased
/// contexts and attachments.
///
/// # Type Erasure with `dyn Any`
///
/// The `dyn Any` marker is special: it indicates a type-erased value without
/// actually creating a `Box<dyn Any>` or similar allocation. Converting from a
/// concrete type to `dyn Any` is a zero-cost operation that doesn't change the
/// underlying representation.
///
/// This is used internally by the [`report!`](crate::report!) macro when you
/// use the format string syntax:
///
/// ```
/// use rootcause::prelude::*;
///
/// // This creates Report<dyn Any> using type erasure
/// let report: Report<dyn Any> = report!("Error code: {}", 404);
/// ```
///
/// # Implementation
///
/// You don't need to implement this trait manually - it's automatically
/// available for all appropriate types.
pub trait ObjectMarker: 'static + sealed_context_marker::Sealed {}
impl<T> ObjectMarker for T where T: 'static {}
impl ObjectMarker for dyn Any {}
*/

/// Marker trait for ownership semantics of owned reports.
///
/// This trait defines the ownership behavior for [`Report<C, O,
/// T>`](crate::Report) instances. It's implemented for [`Mutable`] and
/// [`Cloneable`], which control whether reports can be mutated or cloned.
///
/// # Ownership Modes
///
/// - [`Mutable`]: Indicates unique ownership. The report can be mutated but not
///   cloned. Methods like [`attach`](crate::Report::attach) and
///   [`as_mut`](crate::Report::as_mut) are only available in this mode.
///
/// - [`Cloneable`]: Indicates shared ownership via reference counting. The
///   report can be cloned cheaply but cannot be mutated since there may be
///   multiple references.
///
/// # Associated Types
///
/// The `RefMarker` associated type determines what kind of reference you get
/// when calling [`as_ref`](crate::Report::as_ref):
/// - For [`Mutable`], this is [`Uncloneable`] (the reference cannot use
///   `clone_arc`)
/// - For [`Cloneable`], this is [`Cloneable`] (the reference can use
///   `clone_arc`)
///
/// # Implementation
///
/// This trait is sealed and cannot be implemented outside of this crate. You
/// should use the provided implementations for [`Mutable`] and [`Cloneable`].
pub trait ReportOwnershipMarker: sealed_report_ownership_marker::Sealed {
    /// The ownership marker for references to this report type.
    ///
    /// This determines what type of reference is created when you call
    /// [`Report::as_ref`](crate::Report::as_ref):
    ///
    /// - For [`Mutable`]: Returns [`ReportRef<C,
    ///   Uncloneable>`](crate::ReportRef) because the underlying report has
    ///   unique ownership
    /// - For [`Cloneable`]: Returns [`ReportRef<C,
    ///   Cloneable>`](crate::ReportRef) because the underlying report already
    ///   uses shared ownership
    type RefMarker;
}
impl ReportOwnershipMarker for Mutable {
    type RefMarker = Uncloneable;
}
impl ReportOwnershipMarker for Cloneable {
    type RefMarker = Cloneable;
}

/*
/// Marker trait for ownership semantics of report references.
///
/// This trait defines the ownership behavior for [`ReportRef<C,
/// O>`](crate::ReportRef) instances. It's implemented for [`Cloneable`] and
/// [`Uncloneable`], controlling whether the reference provides the
/// [`clone_arc`](crate::ReportRef::clone_arc) method to obtain an owned report.
///
/// Note that `ReportRef` itself is always `Copy` and `Clone` regardless of this
/// marker. The ownership marker only controls access to the `clone_arc` method.
///
/// # Reference Modes
///
/// - [`Cloneable`]: The reference provides
///   [`clone_arc`](crate::ReportRef::clone_arc) to clone the underlying `Arc`
///   and get an owned [`Report<C, Cloneable, T>`](crate::Report).
///
/// - [`Uncloneable`]: The reference does not provide `clone_arc`. This is used
///   for references to uniquely-owned reports, preserving the uniqueness
///   guarantee.
///
/// # Implementation
///
/// This trait is sealed and cannot be implemented outside of this crate.
pub trait ReportRefOwnershipMarker: Sized + sealed_report_ref_ownership_marker::Sealed {
    /// Converts a [`ReportRef`] with [`Cloneable`] ownership to this ownership
    /// type.
    ///
    /// This method mostly exists as a convenience to facilitate conversions
    /// in generic contexts.
    ///
    /// In non-generic code, you can use [`From`]/[`Into`] directly.
    #[doc(hidden)]
    fn convert_cloneable_report_ref<'a, T>(
        report: ReportRef<'a, dyn Any, Cloneable, T>,
    ) -> ReportRef<'a, dyn Any, Self, T>;
}
impl ReportRefOwnershipMarker for Cloneable {
    fn convert_cloneable_report_ref<'a, T>(
        report: ReportRef<'a, dyn Any, Cloneable, T>,
    ) -> ReportRef<'a, dyn Any, Self, T> {
        report
    }
}
impl ReportRefOwnershipMarker for Uncloneable {
    fn convert_cloneable_report_ref<'a, T>(
        report: ReportRef<'a, dyn Any, Cloneable, T>,
    ) -> ReportRef<'a, dyn Any, Self, T> {
        report.into_uncloneable()
    }
}
     */

/*
/// Marker trait for thread-safety semantics of reports.
///
/// This trait defines whether a report can cross thread boundaries. It's
/// implemented for [`SendSync`] and [`Local`], which control the thread-safety
/// requirements of reports.
///
/// # Thread Safety Modes
///
/// - [`SendSync`]: Reports (and all their contents) are `Send + Sync`, meaning
///   they can be safely sent to other threads and shared between threads.
///
/// - [`Local`]: Reports contain thread-local data that is not `Send` or `Sync`.
///   These reports cannot cross thread boundaries.
///
/// # Implementation
///
/// This trait is sealed and cannot be implemented outside of this crate.
pub trait ThreadSafetyMarker: Sized + sealed_send_sync_marker::Sealed {
    /// Runs report creation hooks specific to this thread-safety marker.
    #[doc(hidden)]
    #[track_caller]
    fn run_creation_hooks(report: ReportMut<'_, dyn Any, Self>);
}
impl ThreadSafetyMarker for SendSync {
    #[inline(always)]
    fn run_creation_hooks(report: ReportMut<'_, dyn Any, Self>) {
        crate::hooks::report_creation::run_creation_hooks_sendsync(report);
    }
}
impl ThreadSafetyMarker for Local {
    #[inline(always)]
    fn run_creation_hooks(report: ReportMut<'_, dyn Any, Self>) {
        crate::hooks::report_creation::run_creation_hooks_local(report);
    }
}
     */

/// Marker trait combining object and thread-safety requirements.
///
/// This trait constrains what types can be used as context or attachment data
/// based on the thread-safety requirements of the report.
///
/// # Implementations
///
/// - For `T = Local`: Implemented for all types that implement
///   [`ObjectMarker`], regardless of their `Send`/`Sync` status. This allows
///   using types like `Rc` in local reports.
///
/// - For `T = SendSync`: Implemented only for types that implement
///   [`ObjectMarker`] and are also `Send + Sync`. This ensures thread-safe
///   reports only contain thread-safe data.
///
/// # Usage
///
/// This trait is used internally by the library to enforce that you can't
/// accidentally create a supposedly thread-safe report containing
/// non-thread-safe data:
///
/// ```compile_fail
/// use std::rc::Rc;
/// use rootcause::prelude::*;
///
/// // This won't compile because Rc is not Send + Sync
/// let rc_data: Rc<String> = Rc::new("error".to_string());
/// let report: Report<Rc<String>, markers::Mutable, markers::SendSync> = report!(rc_data);
/// ```
///
/// Use [`Local`] instead for non-thread-safe data:
///
/// ```
/// use std::rc::Rc;
///
/// use rootcause::prelude::*;
///
/// let rc_data: Rc<String> = Rc::new("error".to_string());
/// let report: Report<Rc<String>, markers::Mutable, markers::Local> = report!(rc_data);
/// ```
pub trait ObjectMarkerFor<T>: Sized + 'static {
    /// Runs report creation hooks specific to this thread-safety marker.
    #[doc(hidden)]
    #[track_caller]
    fn run_creation_hooks(report: ReportMut<'_, dyn Any, T>);
}

impl<O: Sized + 'static> ObjectMarkerFor<Local> for O {
    #[inline(always)]
    fn run_creation_hooks(report: ReportMut<'_, dyn Any, Local>) {
        crate::hooks::report_creation::run_creation_hooks_local(report);
    }
}

impl<O: Sized + 'static> ObjectMarkerFor<SendSync> for O
where
    O: Send + Sync,
{
    #[inline(always)]
    fn run_creation_hooks(report: ReportMut<'_, dyn Any, SendSync>) {
        crate::hooks::report_creation::run_creation_hooks_sendsync(report);
    }
}
