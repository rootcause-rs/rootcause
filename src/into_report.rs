use crate::{markers, prelude::Report, report_collection::ReportCollection};

/// Converts errors and reports into [`Report`] instances with specific thread-safety markers.
///
/// This trait is primarily used internally by the rootcause library for trait bounds in
/// extension methods like [`ResultExt`](crate::result_ext::ResultExt). While it's available
/// for direct use, most applications will find the [`report!`](crate::report!) macro more
/// convenient for creating reports.
///
/// # Internal Usage
///
/// This trait enables generic conversions in methods like
/// [`Result::context`](crate::result_ext::ResultExt::context), allowing them to
/// accept various error types and convert them uniformly into reports.
///
/// # Automatic Implementations
///
/// This trait is automatically implemented for:
/// - All types implementing `std::error::Error` (converts to new `Report`)
/// - Existing `Report` instances (performs identity or marker conversion)
///
/// # Thread Safety
///
/// The type parameter `T` specifies the desired thread-safety marker:
/// - [`markers::SendSync`](crate::markers::SendSync): Report can be sent between threads
/// - [`markers::Local`](crate::markers::Local): Report is restricted to the current thread
///
/// When converting from `SendSync` to `Local`, the conversion always succeeds. Converting
/// from `Local` to `SendSync` is only available if the context type is `Send + Sync`.
///
/// # Typical Usage
///
/// Most applications won't need to call this trait directly. Instead, consider:
/// - Using [`report!`](crate::report!) to create reports from errors or strings
/// - Using [`ResultExt`](crate::result_ext::ResultExt) methods to add context to `Result` types
/// - Using the `From` trait for generic type conversions
///
/// # Examples
///
/// Direct usage is possible, though the alternatives above are often more ergonomic:
///
/// ```rust
/// use rootcause::{IntoReport, prelude::*};
/// use std::io;
///
/// // Direct usage
/// let error: io::Error = io::Error::new(io::ErrorKind::NotFound, "file not found");
/// let report: Report<io::Error> = error.into_report();
///
/// // Alternative using the macro (often more convenient)
/// let error2: io::Error = io::Error::new(io::ErrorKind::NotFound, "config.toml");
/// let report2: Report<io::Error> = report!(error2);
/// ```
pub trait IntoReport<T: markers::ThreadSafetyMarker> {
    /// The context type of the resulting report.
    type Context: markers::ObjectMarker + ?Sized;

    /// The ownership marker of the resulting report.
    type Ownership: markers::ReportOwnershipMarker;

    /// Converts `self` into a [`Report`] with the specified thread-safety marker.
    ///
    /// Most applications will find the [`report!`](crate::report!) macro more convenient
    /// for creating reports.
    #[track_caller]
    #[must_use]
    fn into_report(self) -> Report<Self::Context, Self::Ownership, T>;
}

impl<C, O> IntoReport<markers::SendSync> for Report<C, O, markers::SendSync>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
{
    type Context = C;
    type Ownership = O;

    #[inline(always)]
    fn into_report(self) -> Report<Self::Context, Self::Ownership, markers::SendSync> {
        self
    }
}

impl<C, O, T> IntoReport<markers::Local> for Report<C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    type Context = C;
    type Ownership = O;

    #[inline(always)]
    fn into_report(self) -> Report<Self::Context, Self::Ownership, markers::Local> {
        self.into_local()
    }
}

impl<C, T> IntoReport<T> for C
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
    T: markers::ThreadSafetyMarker,
{
    type Context = C;
    type Ownership = markers::Mutable;

    #[inline(always)]
    fn into_report(self) -> Report<C, markers::Mutable, T> {
        Report::new(self)
    }
}

/// Converts errors and reports into [`ReportCollection`] instances.
///
/// This trait is primarily used internally by the rootcause library for trait bounds.
/// While it's available for direct use, most applications will find the `From` trait
/// or iterator methods more convenient for creating collections of reports.
///
/// # Internal Usage
///
/// This trait provides trait bounds for generic conversions to [`ReportCollection`],
/// similar to how [`IntoReport`] works for single reports.
///
/// # Automatic Implementations
///
/// This trait is automatically implemented for:
/// - All types implementing `std::error::Error` (creates single-item collection)
/// - `Report` instances (creates single-item collection)
/// - `ReportCollection` instances (identity or marker conversion)
///
/// # Typical Usage
///
/// Most applications won't need to call this trait directly. Instead, consider:
/// - Using iterator methods: `iter.map(|e| report!(e)).collect()`
/// - Using `From` trait implementations for type conversions
/// - Using `ReportCollection::new()` or builder methods
///
/// # Examples
///
/// Direct usage is possible, though the alternatives above are often more ergonomic:
///
/// ```rust
/// use rootcause::{IntoReportCollection, prelude::*, report_collection::ReportCollection};
/// use std::io;
///
/// // Direct usage
/// let error: io::Error = io::Error::other("An error occurred");
/// let collection: ReportCollection<io::Error> = error.into_report_collection();
/// assert_eq!(collection.len(), 1);
///
/// // Alternative using iterators (often more convenient for multiple errors)
/// let errors: Vec<io::Error> = vec![io::Error::other("error 1")];
/// let collection2: ReportCollection = errors
///     .into_iter()
///     .map(|e| report!(e))
///     .collect();
/// ```
pub trait IntoReportCollection<T: markers::ThreadSafetyMarker> {
    /// The context type of the resulting report collection.
    type Context: markers::ObjectMarker + ?Sized;

    /// Converts `self` into a [`ReportCollection`] with the specified thread-safety marker.
    ///
    /// Most applications will find iterator methods or the `From` trait more convenient
    /// for creating collections.
    #[track_caller]
    #[must_use]
    fn into_report_collection(self) -> ReportCollection<Self::Context, T>;
}

impl<C, O> IntoReportCollection<markers::SendSync> for Report<C, O, markers::SendSync>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
{
    type Context = C;

    #[inline(always)]
    fn into_report_collection(self) -> ReportCollection<Self::Context, markers::SendSync> {
        core::iter::once(self).collect()
    }
}

impl<C, O, T> IntoReportCollection<markers::Local> for Report<C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    type Context = C;

    #[inline(always)]
    fn into_report_collection(self) -> ReportCollection<Self::Context, markers::Local> {
        core::iter::once(self.into_local()).collect()
    }
}

impl<C> IntoReportCollection<markers::SendSync> for ReportCollection<C, markers::SendSync>
where
    C: markers::ObjectMarker + ?Sized,
{
    type Context = C;

    #[inline(always)]
    fn into_report_collection(self) -> ReportCollection<Self::Context, markers::SendSync> {
        self
    }
}

impl<C, T> IntoReportCollection<markers::Local> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type Context = C;

    #[inline(always)]
    fn into_report_collection(self) -> ReportCollection<Self::Context, markers::Local> {
        self.into_local()
    }
}

impl<C, T> IntoReportCollection<T> for C
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
    T: markers::ThreadSafetyMarker,
{
    type Context = C;

    #[inline(always)]
    fn into_report_collection(self) -> ReportCollection<C, T> {
        core::iter::once(Report::new(self)).collect()
    }
}
