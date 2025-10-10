use crate::{markers, prelude::Report, report_collection::ReportCollection};

/// Trait for converting an error or a report into a [`Report`] with the specified
/// thread safety marker.
pub trait IntoReport<T: markers::ThreadSafetyMarker> {
    /// The context type of the resulting report.
    type Context: markers::ObjectMarker + ?Sized;

    /// The ownership marker of the resulting report.
    type Ownership: markers::ReportOwnershipMarker;

    /// Converts `self` into a [`Report`] with the specified context, ownership,
    /// and thread safety markers.
    ///
    /// # Examples
    /// ```
    /// use rootcause::prelude::*;
    /// let my_error = std::io::Error::other("An error occurred");
    /// let report: Report<std::io::Error> = my_error.into_report();
    /// let report2: Report<std::io::Error> = report.into_report();
    /// ```
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

/// Trait for converting an error or a report into a [`ReportCollection`] with the specified
/// thread safety marker.
pub trait IntoReportCollection<T: markers::ThreadSafetyMarker> {
    /// The context type of the resulting report collection.
    type Context: markers::ObjectMarker + ?Sized;

    /// Converts `self` into a [`ReportCollection`] with the specified context and
    /// thread safety markers.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{IntoReportCollection, prelude::*, report_collection::ReportCollection};
    /// let my_error = std::io::Error::other("An error occurred");
    /// let report_collection: ReportCollection<std::io::Error> = my_error.into_report_collection();
    /// ```
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
