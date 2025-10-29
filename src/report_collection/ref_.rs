use core::{any::Any, marker::PhantomData};

use rootcause_internals::{RawReport, handlers::FormattingFunction};

use crate::{
    ReportRef,
    markers::{self, Cloneable, Local, SendSync},
    report_collection::ReportCollectionIter,
};

/// A borrowed reference to a collection of reports with optional type constraints.
///
/// This struct provides a lightweight, borrowed view of a collection of reports without
/// taking ownership. It offers various methods for accessing individual reports,
/// iterating over the collection, and converting between different type constraints.
///
/// Unlike [`ReportCollection`], this struct does not own the underlying reports and
/// has a lifetime parameter that ties it to the source data. It's designed to be
/// [`Copy`] and [`Clone`] for efficient passing and sharing.
///
/// # Type Parameters
///
/// * `Context` - The type of error context contained in the reports (defaults to `dyn Any`)
/// * `ThreadSafety` - Thread safety marker (defaults to [`SendSync`])
///
/// # Examples
///
/// Basic usage with default `dyn Any` context:
///
/// ```
/// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
/// use std::any::Any;
///
/// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
/// collection.push(report!("Error 1").into_cloneable());
/// collection.push(report!("Error 2").into_cloneable());
///
/// // Get a borrowed reference to the collection
/// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
/// println!("Collection has {} reports", collection_ref.len());
///
/// // Access individual reports
/// if let Some(first_report) = collection_ref.get(0) {
///     println!("First report: {}", first_report);
/// }
/// ```
///
/// Using a custom context type:
///
/// ```
/// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
///
/// #[derive(Debug)]
/// struct DatabaseError {
///     code: u32,
///     message: String,
/// }
///
/// impl std::fmt::Display for DatabaseError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "Database Error {}: {}", self.code, self.message)
///     }
/// }
///
/// let mut collection: ReportCollection<DatabaseError> = ReportCollection::new();
/// collection.push(report!(DatabaseError {
///     code: 1001,
///     message: "Connection failed".to_string()
/// }).into_cloneable());
/// collection.push(report!(DatabaseError {
///     code: 1002,
///     message: "Query timeout".to_string()
/// }).into_cloneable());
///
/// let collection_ref: ReportCollectionRef<'_, DatabaseError> = collection.as_ref();
/// assert_eq!(collection_ref.len(), 2);
/// ```
///
/// [`ReportCollection`]: crate::report_collection::ReportCollection
#[repr(transparent)]
pub struct ReportCollectionRef<'a, Context = dyn Any, ThreadSafety = SendSync>
where
    Context: markers::ObjectMarker + ?Sized,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: &'a [RawReport],
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, C, T> Copy for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
}
impl<'a, C, T> Clone for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C, T> ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new ReportCollectionRef from a slice of raw reports
    ///
    /// # Safety
    ///
    /// - The thread safety marker must match the contents of the reports. More
    ///   specifically if the marker is `SendSync`, then all the data
    ///   (recursively) contained by the reports must be `Send+Sync`.
    /// - The caller must ensure that the contexts of the `RawReport`s are
    ///   actually of type `C` when `C` if is is a type different from `dyn
    ///   Any`.
    pub(crate) unsafe fn from_raw(raw: &'a [RawReport]) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> &'a [RawReport] {
        self.raw
    }

    /// Returns the number of reports in this collection reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
    /// use std::any::Any;
    ///
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("Error").into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
    /// assert_eq!(collection_ref.len(), 1);
    /// ```
    pub fn len(self) -> usize {
        self.raw.len()
    }

    /// Returns a reference to the report at the given index, or `None` if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
    /// use std::any::Any;
    ///
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("First error").into_cloneable());
    /// collection.push(report!("Second error").into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
    /// assert!(collection_ref.get(0).is_some());
    /// assert!(collection_ref.get(5).is_none());
    /// ```
    ///
    /// Using a custom error context type:
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
    ///
    /// #[derive(Debug)]
    /// struct NetworkError {
    ///     status_code: u16,
    ///     url: String,
    /// }
    ///
    /// impl std::fmt::Display for NetworkError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "HTTP {} error for {}", self.status_code, self.url)
    ///     }
    /// }
    ///
    /// let mut collection: ReportCollection<NetworkError> = ReportCollection::new();
    /// collection.push(report!(NetworkError {
    ///     status_code: 404,
    ///     url: "https://api.example.com/users".to_string()
    /// }).into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, NetworkError> = collection.as_ref();
    /// if let Some(error_report) = collection_ref.get(0) {
    ///     println!("Retrieved error: {}", error_report);
    /// }
    /// ```
    pub fn get(self, index: usize) -> Option<ReportRef<'a, C, Cloneable, T>> {
        let raw = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportRef::from_raw(raw)) }
    }

    /// Returns `true` if the collection contains no reports.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::report_collection::{ReportCollection, ReportCollectionRef};
    /// use std::any::Any;
    ///
    /// let collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
    /// assert!(collection_ref.is_empty());
    /// ```
    pub fn is_empty(self) -> bool {
        self.raw.is_empty()
    }

    /// Returns an iterator over references to the reports in this collection.
    ///
    /// The iterator yields [`ReportRef`] instances that reference the reports
    /// without taking ownership of them.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
    /// use std::any::Any;
    ///
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("Error 1").into_cloneable());
    /// collection.push(report!("Error 2").into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
    /// for report in collection_ref.iter() {
    ///     println!("Report: {}", report);
    /// }
    /// ```
    pub fn iter(self) -> ReportCollectionIter<'a, C, T> {
        unsafe { ReportCollectionIter::from_raw(self.raw) }
    }

    /// Converts this collection reference into an owned [`ReportCollection`].
    ///
    /// This method clones all the reports in the collection, creating a new owned
    /// collection with the same type parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
    /// use std::any::Any;
    ///
    /// let mut original: ReportCollection<dyn Any> = ReportCollection::new();
    /// original.push(report!("Error").into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = original.as_ref();
    /// let owned_copy: ReportCollection<dyn Any> = collection_ref.to_owned();
    /// assert_eq!(owned_copy.len(), 1);
    /// ```
    ///
    /// [`ReportCollection`]: crate::report_collection::ReportCollection
    pub fn to_owned(self) -> crate::report_collection::ReportCollection<C, T> {
        self.iter().collect()
    }

    /// Converts this collection reference to use `dyn Any` as the context type.
    ///
    /// This method performs a type erasure, allowing you to work with reports
    /// that have different context types in a uniform way.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}};
    /// use std::any::Any;
    ///
    /// // Create a collection with default dyn Any context type
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("Error").into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
    /// let dyn_any_ref: ReportCollectionRef<'_, dyn Any> = collection_ref.into_dyn_any();
    /// assert_eq!(dyn_any_ref.len(), 1);
    /// ```
    pub fn into_dyn_any(self) -> ReportCollectionRef<'a, dyn Any, T> {
        unsafe { ReportCollectionRef::from_raw(self.into_raw()) }
    }

    /// Converts this collection reference to use [`Local`] thread safety.
    ///
    /// This method changes the thread safety marker to [`Local`], indicating
    /// that the collection should not be sent across thread boundaries.
    /// This is useful when you need to work with reports that contain
    /// non-`Send` or non-`Sync` data.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::{ReportCollection, ReportCollectionRef}, markers::Local};
    /// use std::any::Any;
    ///
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("Error").into_cloneable());
    ///
    /// let collection_ref: ReportCollectionRef<'_, dyn Any> = collection.as_ref();
    /// let local_ref: ReportCollectionRef<'_, dyn Any, Local> = collection_ref.into_local();
    /// ```
    pub fn into_local(self) -> ReportCollectionRef<'a, C, Local> {
        unsafe { ReportCollectionRef::from_raw(self.into_raw()) }
    }
}

impl<'a, C, T> IntoIterator for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportCollectionIter<'a, C, T>;
    type Item = ReportRef<'a, C, Cloneable, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, C, T> core::fmt::Display for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reports: &'a [RawReport] = self.raw;
        let reports: &'a [ReportRef<'a, dyn Any, markers::Uncloneable, markers::Local>] =
            unsafe { core::mem::transmute(reports) };
        crate::hooks::report_formatting::format_reports(reports, f, FormattingFunction::Display)
    }
}

impl<'a, C, T> core::fmt::Debug for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reports: &'a [RawReport] = self.raw;
        let reports: &'a [ReportRef<'_, dyn Any, markers::Uncloneable, markers::Local>] =
            unsafe { core::mem::transmute(reports) };
        crate::hooks::report_formatting::format_reports(reports, f, FormattingFunction::Display)
    }
}

mod from_impls {
    use super::*;

    macro_rules! unsafe_report_collection_to_report_collection {
        ($(
            <
                $($param:ident),*
            >:
            $context1:ty => $context2:ty,
            $thread_safety1:ty => $thread_safety2:ty
        ),* $(,)?) => {
            $(
                impl<'a, $($param),*> From<ReportCollectionRef<'a, $context1, $thread_safety1>> for ReportCollectionRef<'a, $context2, $thread_safety2>
                    where
                        $($param: markers::ObjectMarker)*
                    {
                    fn from(value: ReportCollectionRef<'a, $context1, $thread_safety1>) -> Self {
                        unsafe { ReportCollectionRef::from_raw(value.raw) }
                    }
                }
            )*
        };
    }

    unsafe_report_collection_to_report_collection! {
        <C>: C => C, SendSync => Local,
        <C>: C => dyn Any, SendSync => SendSync,
        <C>: C => dyn Any, SendSync => Local,
        <C>: C => dyn Any, Local => Local,
        <>: dyn Any => dyn Any, SendSync => Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::Local;

    #[test]
    fn test_report_collection_ref_send_sync() {
        static_assertions::assert_not_impl_all!(ReportCollectionRef<'static, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollectionRef<'static, Local>: Send, Sync);
    }

    #[test]
    fn test_report_collection_ref_copy_clone() {
        static_assertions::assert_impl_all!(ReportCollectionRef<'static, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportCollectionRef<'static, Local>: Copy, Clone);
    }
}
