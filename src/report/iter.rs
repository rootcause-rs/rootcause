use alloc::vec::Vec;
use core::{any::Any, iter::FusedIterator, marker::PhantomData};

use rootcause_internals::RawReportRef;

use crate::{ReportRef, markers};

/// An iterator over a report and all its descendant reports in depth-first
/// order.
///
/// This iterator yields `ReportRef` items, which are references to the reports
/// in the hierarchy. The iterator traverses the report tree in a depth-first
/// manner, starting from the root report and visiting each child report before
/// moving to the next sibling.
#[must_use]
pub struct ReportIter<'a, Ownership, ThreadSafety>
where
    Ownership: markers::ReportRefOwnershipMarker,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    stack: Vec<RawReportRef<'a>>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, O, T> ReportIter<'a, O, T>
where
    O: crate::markers::ReportRefOwnershipMarker,
    T: crate::markers::ThreadSafetyMarker,
{
    /// Creates a new `ReportIter` from a vector of raw report references
    ///
    /// # Safety
    ///
    /// - The thread safety marker must match the contents of the reports. More
    ///   specifically if the marker is `SendSync`, then all the data
    ///   (recursively) contained by the reports must be `Send+Sync`.
    /// - The ownership marker must match the ownership semantics of the report
    ///   references.
    #[must_use]
    pub(crate) unsafe fn from_raw(stack: Vec<RawReportRef<'a>>) -> Self {
        Self {
            stack,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T>
where
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    type Item = ReportRef<'a, dyn Any, markers::Cloneable, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.stack.pop()?;
        self.stack
            .extend(cur.children().iter().map(|r| r.as_ref()).rev());
        Some(unsafe { ReportRef::from_raw(cur) })
    }
}

impl<'a, O, T> FusedIterator for ReportIter<'a, O, T>
where
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
}
