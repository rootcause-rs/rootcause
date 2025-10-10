use alloc::vec::Vec;
use core::{any::Any, marker::PhantomData};

use rootcause_internals::RawReport;

use crate::{
    Report, ReportRef,
    markers::{self, Cloneable, SendSync},
    report_collection::{ReportCollection, ReportCollectionIter, ReportCollectionRef},
};

#[repr(transparent)]
pub struct ReportCollectionMut<'a, Context: ?Sized + 'static = dyn Any, ThreadSafety = SendSync> {
    raw: &'a mut Vec<RawReport>,
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, C: ?Sized, T> ReportCollectionMut<'a, C, T> {
    /// Creates a new ReportCollectionRef from a slice of raw reports
    pub(crate) unsafe fn from_raw(raw: &'a mut Vec<RawReport>) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<'a, C, T> ReportCollectionMut<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    pub fn reborrow<'b: 'a>(&'b mut self) -> ReportCollectionMut<'b, C, T> {
        let reborrowed = &mut self.raw;
        unsafe { Self::from_raw(reborrowed) }
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

    pub fn take(&mut self) -> ReportCollection<C, T> {
        let raw = core::mem::take(self.raw);
        unsafe { ReportCollection::from_raw(raw) }
    }

    pub fn iter(&self) -> ReportCollectionIter<'_, C, T> {
        self.as_ref().iter()
    }

    pub fn into_iter(self) -> ReportCollectionIter<'a, C, T> {
        self.into_ref().iter()
    }

    pub fn as_ref(&self) -> ReportCollectionRef<'_, C, T> {
        unsafe { ReportCollectionRef::from_raw(self.raw.as_slice()) }
    }

    pub fn into_ref(self) -> ReportCollectionRef<'a, C, T> {
        unsafe { ReportCollectionRef::from_raw(self.raw.as_slice()) }
    }
}

impl<'a, C, T> IntoIterator for ReportCollectionMut<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type Item = ReportRef<'a, C, Cloneable, T>;
    type IntoIter = ReportCollectionIter<'a, C, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::Local;

    #[test]
    fn test_report_collection_mut_send_sync() {
        static_assertions::assert_not_impl_all!(ReportCollectionMut<'static, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollectionMut<'static, Local>: Send, Sync);
    }

    #[test]
    fn test_report_collection_mut_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportCollectionMut<'static, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportCollectionMut<'static, Local>: Copy, Clone);
    }
}
