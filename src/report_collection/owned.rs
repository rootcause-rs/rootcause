use alloc::vec::Vec;
use core::{any::Any, marker::PhantomData};

use rootcause_internals::{RawReport, handlers::ContextHandler};

use crate::{
    Report, ReportRef, handlers,
    markers::{self, Cloneable, Local, Mutable, SendSync},
    report_attachments::ReportAttachments,
    report_collection::{ReportCollectionIntoIter, ReportCollectionIter, ReportCollectionRef},
};

/// A collection of reports.
///
/// You can think of a [`ReportCollection<C, T>`] as a wrapper around a `Vec<Report<C, markers::Cloneable, T>>`,
/// however, it has a slightly different API:
/// - It provides methods such as [`context`](Self::context) and [`context_custom`](Self::context_custom)
///   to create new reports with the collection as children.
/// - It has convenience methods to convert between different context and thread safety markers such as
///   [`into_dyn_any`](Self::into_dyn_any) and [`into_local`](Self::into_local).
/// - It also possible to convert between different context and thread safety markers using
///   the [`From`] and [`Into`] traits.
#[repr(transparent)]
pub struct ReportCollection<Context = dyn Any, ThreadSafety = SendSync>
where
    Context: markers::ObjectMarker + ?Sized,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: Vec<RawReport>,
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<C, T> ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new ReportCollection from a vector of raw reports
    ///
    /// # Safety
    /// - The thread safety marker must match the contents of the reports. More specifically if the marker is `SendSync`, then
    ///   all the data (recursively) contained by the reports must be `Send+Sync`.
    /// - The caller must ensure that the contexts of the `RawReport`s are actually of
    ///   type `C` when `C` if is is a type different from `dyn Any`.
    pub(crate) unsafe fn from_raw(raw: Vec<RawReport>) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> Vec<RawReport> {
        self.raw
    }

    /// Creates a new, empty `ReportCollection`.
    ///
    /// The collection will be initially empty and will have no capacity allocated.
    /// This method is equivalent to calling [`Default::default()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::report_collection::ReportCollection;
    ///
    /// let collection: ReportCollection = ReportCollection::new();
    /// assert!(collection.is_empty());
    /// assert_eq!(collection.len(), 0);
    /// ```
    pub fn new() -> Self {
        unsafe { Self::from_raw(Vec::new()) }
    }

    /// Appends a report to the end of the collection.
    ///
    /// This method takes ownership of the report and adds it to the collection.
    /// The report must have the [`Cloneable`] ownership marker, which allows it to be
    /// stored in the collection and cloned when needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// let report = report!("An error occurred").into_cloneable();
    ///
    /// collection.push(report);
    /// assert_eq!(collection.len(), 1);
    /// ```
    pub fn push(&mut self, report: Report<C, Cloneable, T>) {
        self.raw.push(report.into_raw())
    }

    /// Removes and returns the last report from the collection.
    ///
    /// Returns [`None`] if the collection is empty.
    ///
    /// This method provides LIFO (last in, first out) behavior, making the collection
    /// behave like a stack for the most recently added reports.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// let report1 = report!("First error").into_cloneable();
    /// let report2 = report!("Second error").into_cloneable();
    ///
    /// collection.push(report1);
    /// collection.push(report2);
    ///
    /// let last_report = collection.pop().unwrap();
    /// assert_eq!(collection.len(), 1);
    ///
    /// let empty_pop = ReportCollection::<&str>::new().pop();
    /// assert!(empty_pop.is_none());
    /// ```
    pub fn pop(&mut self) -> Option<Report<C, Cloneable, T>> {
        let report = self.raw.pop()?;

        // SAFETY: The thread safety marker matches, because we only
        // contain attachments with a matching thread safety marker
        let report = unsafe { Report::from_raw(report) };

        Some(report)
    }

    /// Returns the number of reports in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// assert_eq!(collection.len(), 0);
    ///
    /// collection.push(report!("Error 1").into_cloneable());
    /// collection.push(report!("Error 2").into_cloneable());
    /// assert_eq!(collection.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Returns a reference to the report at the given index.
    ///
    /// Returns [`None`] if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// collection.push(report!("First error").into_cloneable());
    /// collection.push(report!("Second error").into_cloneable());
    ///
    /// let first_report = collection.get(0).unwrap();
    /// let second_report = collection.get(1).unwrap();
    /// let out_of_bounds = collection.get(2);
    ///
    /// assert!(out_of_bounds.is_none());
    /// ```
    pub fn get(&self, index: usize) -> Option<ReportRef<'_, C, Cloneable, T>> {
        let raw = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportRef::from_raw(raw)) }
    }

    /// Returns `true` if the collection contains no reports.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// assert!(collection.is_empty());
    ///
    /// collection.push(report!("An error").into_cloneable());
    /// assert!(!collection.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Returns an iterator over references to the reports in the collection.
    ///
    /// The iterator yields [`ReportRef`] items, which are lightweight references
    /// to the reports in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// collection.push(report!("Error 1").into_cloneable());
    /// collection.push(report!("Error 2").into_cloneable());
    ///
    /// for (i, report_ref) in collection.iter().enumerate() {
    ///     println!("Report {}: {}", i, report_ref);
    /// }
    /// ```
    pub fn iter(&self) -> ReportCollectionIter<'_, C, T> {
        self.as_ref().into_iter()
    }

    /// Returns a reference view of the collection.
    ///
    /// This method creates a [`ReportCollectionRef`] which provides a lightweight,
    /// non-owning view of the collection. This is useful for passing the collection
    /// to functions that don't need to take ownership.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection = ReportCollection::new();
    /// collection.push(report!("An error").into_cloneable());
    ///
    /// let collection_ref = collection.as_ref();
    /// assert_eq!(collection_ref.len(), 1);
    /// ```
    pub fn as_ref(&self) -> ReportCollectionRef<'_, C, T> {
        unsafe { ReportCollectionRef::from_raw(self.raw.as_slice()) }
    }

    /// Converts the collection to use type-erased contexts via `dyn Any`.
    ///
    /// This performs type erasure on the context type parameter, allowing collections
    /// with different concrete context types to be stored together or passed to
    /// functions that accept `ReportCollection<dyn Any, T>`.
    ///
    /// This method does not actually modify the collection in any way. It only has the effect of "forgetting"
    /// that the context actually has the type `C`.
    ///
    /// The thread safety marker `T` is preserved during this conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::Any;
    /// use rootcause::{report_collection::ReportCollection, report};
    ///
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("String error").into_cloneable());
    ///
    /// let erased: ReportCollection<dyn Any> = collection.into_dyn_any();
    /// assert_eq!(erased.len(), 1);
    /// ```
    pub fn into_dyn_any(self) -> ReportCollection<dyn Any, T> {
        unsafe { ReportCollection::from_raw(self.into_raw()) }
    }

    /// Converts the collection to use `Local` thread safety semantics.
    ///
    /// This changes the thread safety marker from any type to [`Local`], which means
    /// the resulting collection will not implement [`Send`] or [`Sync`]. This is useful
    /// when you want to use the collection in single-threaded contexts and potentially
    /// store non-thread-safe data.
    ///
    /// This method does not actually modify the collection in any way. It only has the effect of "forgetting" that
    /// all objects in the [`ReportCollection`] are actually [`Send`] and [`Sync`].
    ///
    /// The context type `C` is preserved during this conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, markers::Local, report};
    ///
    /// let mut collection: ReportCollection<dyn std::any::Any> = ReportCollection::new(); // defaults to SendSync
    /// collection.push(report!("An error").into_cloneable());
    ///
    /// let local_collection: ReportCollection<dyn std::any::Any, Local> = collection.into_local();
    /// assert_eq!(local_collection.len(), 1);
    /// ```
    pub fn into_local(self) -> ReportCollection<C, Local> {
        unsafe { ReportCollection::from_raw(self.into_raw()) }
    }

    /// Creates a new [`Report`] with the given context and sets the current report collection as the children of the new report.
    ///
    /// The new context will use the [`handlers::Display`] handler to format the context.
    ///
    /// This is a convenience method used for chaining method calls; it consumes the [`ReportCollection`] and returns a new [`Report`].
    ///
    /// If you want a different context handler, you can use [`Report::context_custom`].
    ///
    /// If you want to more directly control the allocation of the new report, you can use [`Report::from_parts`],
    /// which is the underlying method used to implement this method.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{Report, report_collection::ReportCollection, report};
    /// let report_collection: ReportCollection = [report!("error A"), report!("error B")]
    ///     .into_iter()
    ///     .collect();
    /// let report: Report<&str> = report_collection.context("additional context");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context<D>(self, context: D) -> Report<D, Mutable, T>
    where
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
    {
        self.context_custom::<handlers::Display, _>(context)
    }

    /// Creates a new [`Report`] with the given context and sets the current report collection as the children of the new report.
    ///
    /// This is a convenience method used for chaining method calls; it consumes the [`ReportCollection`] and returns a [`Report`].
    ///
    /// If you want to more directly control the allocation of the new report, you can use [`Report::from_parts`],
    /// which is the underlying method used to implement this method.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{Report, report_collection::ReportCollection, report, handlers};
    /// let report_collection: ReportCollection = [report!("error A"), report!("error B")]
    ///     .into_iter()
    ///     .collect();
    /// let report: Report<&str> = report_collection.context_custom::<handlers::Debug, _>("context");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context_custom<H, D>(self, context: D) -> Report<D, Mutable, T>
    where
        D: markers::ObjectMarkerFor<T>,
        H: ContextHandler<D>,
    {
        Report::from_parts::<H>(context, self.into_dyn_any(), ReportAttachments::new())
    }
}

impl<C, T> Default for ReportCollection<C, T>
where
    C: markers::ObjectMarkerFor<T> + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<C> ReportCollection<C, SendSync>
where
    C: markers::ObjectMarkerFor<SendSync> + ?Sized,
{
    /// Creates a new, empty `ReportCollection` with `SendSync` thread safety.
    ///
    /// This is equivalent to calling [`new()`](Self::new) but makes the thread safety
    /// marker explicit. The resulting collection can be safely sent between threads
    /// and shared across threads.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, markers::SendSync};
    ///
    /// let collection: ReportCollection<&str, SendSync> = ReportCollection::new_sendsync();
    /// assert!(collection.is_empty());
    /// ```
    pub fn new_sendsync() -> Self {
        Self::new()
    }
}

impl<C> ReportCollection<C, Local>
where
    C: markers::ObjectMarker + ?Sized,
{
    /// Creates a new, empty `ReportCollection` with `Local` thread safety.
    ///
    /// This creates a collection that is not [`Send`] or [`Sync`], meaning it cannot be
    /// transferred between threads or shared across threads. This is useful for
    /// single-threaded applications or when you need to store non-thread-safe data.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, markers::Local};
    ///
    /// let collection: ReportCollection<&str, Local> = ReportCollection::new_local();
    /// assert!(collection.is_empty());
    /// ```
    pub fn new_local() -> Self {
        Self::new()
    }
}

impl<C, O, T> Extend<Report<C, O, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = Report<C, O, T>>>(&mut self, iter: I) {
        for report in iter {
            self.push(report.into_cloneable());
        }
    }
}

impl<C, O, T> Extend<Report<C, O, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = Report<C, O, T>>>(&mut self, iter: I) {
        for report in iter {
            self.push(report.into_dyn_any().into_cloneable());
        }
    }
}

impl<'a, C, T> Extend<ReportRef<'a, C, Cloneable, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(&mut self, iter: I) {
        for report in iter {
            self.push(report.clone_arc());
        }
    }
}

impl<'a, C, T> Extend<ReportRef<'a, C, Cloneable, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(&mut self, iter: I) {
        for report in iter {
            self.push(report.clone_arc().into_dyn_any());
        }
    }
}

impl<C, O, T> FromIterator<Report<C, O, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = Report<C, O, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<C, O, T> FromIterator<Report<C, O, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = Report<C, O, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<'a, C, T> FromIterator<ReportRef<'a, C, Cloneable, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<'a, C, T> FromIterator<ReportRef<'a, C, Cloneable, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<C, T> core::fmt::Display for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.as_ref(), f)
    }
}

impl<C, T> core::fmt::Debug for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), f)
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
                impl<$($param),*> From<ReportCollection<$context1, $thread_safety1>> for ReportCollection<$context2, $thread_safety2>
                    where
                        $($param: markers::ObjectMarker)*
                    {
                    fn from(value: ReportCollection<$context1, $thread_safety1>) -> Self {
                        unsafe { ReportCollection::from_raw(value.raw) }
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

impl<C, T> From<Vec<Report<C, Cloneable, T>>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(reports: Vec<Report<C, Cloneable, T>>) -> Self {
        let raw_reports = reports.into_iter().map(|v| v.into_raw()).collect();
        unsafe { ReportCollection::from_raw(raw_reports) }
    }
}

impl<const N: usize, C, T> From<[Report<C, Cloneable, T>; N]> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(reports: [Report<C, Cloneable, T>; N]) -> Self {
        let raw_reports = reports.into_iter().map(|v| v.into_raw()).collect();
        unsafe { ReportCollection::from_raw(raw_reports) }
    }
}

unsafe impl<C> Send for ReportCollection<C, SendSync> where C: markers::ObjectMarker + ?Sized {}
unsafe impl<C> Sync for ReportCollection<C, SendSync> where C: markers::ObjectMarker + ?Sized {}

impl<C, T> IntoIterator for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type Item = Report<C, Cloneable, T>;
    type IntoIter = ReportCollectionIntoIter<C, T>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { ReportCollectionIntoIter::from_raw(self.raw) }
    }
}

impl<C, T> Clone for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        self.iter().map(|child| child.clone_arc()).collect()
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[allow(dead_code)]
    struct NonSend(*const ());
    static_assertions::assert_not_impl_any!(NonSend: Send, Sync);

    #[test]
    fn test_report_collection_send_sync() {
        static_assertions::assert_impl_all!(ReportCollection<(), SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportCollection<String, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportCollection<NonSend, SendSync>: Send, Sync); // This still makes sense, since you won't actually be able to construct this report
        static_assertions::assert_impl_all!(ReportCollection<dyn Any, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportCollection<(), Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollection<String, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollection<NonSend, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollection<dyn Any, Local>: Send, Sync);
    }

    #[test]
    fn test_report_collection_copy_clone() {
        static_assertions::assert_impl_all!(ReportCollection<(), SendSync>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<String, SendSync>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<NonSend, SendSync>: Clone); // This still makes sense, since you won't actually be able to construct this report
        static_assertions::assert_impl_all!(ReportCollection<dyn Any, SendSync>: Clone);

        static_assertions::assert_impl_all!(ReportCollection<(), Local>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<String, Local>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<NonSend, Local>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<dyn Any, Local>: Clone);

        static_assertions::assert_not_impl_any!(ReportCollection<(), SendSync>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<String, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<NonSend, SendSync>: Copy); // This still makes sense, since you won't actually be able to construct this report_collection collection
        static_assertions::assert_not_impl_any!(ReportCollection<dyn Any, SendSync>: Copy);

        static_assertions::assert_not_impl_any!(ReportCollection<(), Local>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<String, Local>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<NonSend, Local>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<dyn Any, Local>: Copy);
    }
}
