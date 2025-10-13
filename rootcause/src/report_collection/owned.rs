use alloc::vec::Vec;
use core::{any::Any, marker::PhantomData};

use rootcause_internals::{RawReport, handlers::ContextHandler};

use crate::{
    Report, ReportRef, handlers,
    markers::{self, Cloneable, Local, Mutable, SendSync},
    report_attachments::ReportAttachments,
    report_collection::{ReportCollectionIntoIter, ReportCollectionIter, ReportCollectionRef},
};

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

    pub fn new() -> Self {
        unsafe { Self::from_raw(Vec::new()) }
    }

    pub fn push(&mut self, report: Report<C, Cloneable, T>) {
        self.raw.push(report.into_raw())
    }

    pub fn pop(&mut self) -> Option<Report<C, Cloneable, T>> {
        let report = self.raw.pop()?;

        // SAFETY: The thread safety marker matches, because we only
        // contain attachments with a matching thread safety marker
        let report = unsafe { Report::from_raw(report) };

        Some(report)
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn get(&self, index: usize) -> Option<ReportRef<'_, C, Cloneable, T>> {
        let raw = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportRef::from_raw(raw)) }
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn iter(&self) -> ReportCollectionIter<'_, C, T> {
        self.as_ref().into_iter()
    }

    pub fn into_iter(self) -> ReportCollectionIntoIter<C, T> {
        unsafe { ReportCollectionIntoIter::from_raw(self.raw) }
    }

    pub fn as_ref(&self) -> ReportCollectionRef<'_, C, T> {
        unsafe { ReportCollectionRef::from_raw(self.raw.as_slice()) }
    }

    pub fn into_dyn_any(self) -> ReportCollection<dyn Any, T> {
        unsafe { ReportCollection::from_raw(self.into_raw()) }
    }

    pub fn into_local(self) -> ReportCollection<C, Local> {
        unsafe { ReportCollection::from_raw(self.into_raw()) }
    }

    /// Creates a new [`Report`] with the given context and sets the current report collection as the children of the new report.
    ///
    /// The new context will use the [`handlers::Display`] handler to format the context.
    ///
    /// This is a convenience method used for chaining method calls; it consumes the [`ReportCollection`] and returns a new [`Report`].
    ///
    /// If you want a different context handler, you can use [`Report::context_with_handler`].
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
        self.context_with_handler::<handlers::Display, _>(context)
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
    /// let repot: Report<&str> =
    ///     report_collection.context_with_handler::<handlers::Debug, _>("context");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context_with_handler<H, D>(self, context: D) -> Report<D, Mutable, T>
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
    pub fn new_sendsync() -> Self {
        Self::new()
    }
}

impl<C> ReportCollection<C, Local>
where
    C: markers::ObjectMarker + ?Sized,
{
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

mod unsafe_impls {
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
                impl<'a, $($param),*> From<ReportCollection<$context1, $thread_safety1>> for ReportCollection<$context2, $thread_safety2>
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

impl<'a, C, T> From<Vec<Report<C, Cloneable, T>>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(reports: Vec<Report<C, Cloneable, T>>) -> Self {
        let raw_reports = reports.into_iter().map(|v| v.into_raw()).collect();
        unsafe { ReportCollection::from_raw(raw_reports) }
    }
}

impl<'a, const N: usize, C, T> From<[Report<C, Cloneable, T>; N]> for ReportCollection<C, T>
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
        self.into_iter()
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
