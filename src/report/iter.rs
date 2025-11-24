use alloc::vec::Vec;
use core::{any::Any, iter::FusedIterator, marker::PhantomData};

use crate::{ReportRef, markers::Cloneable};

/// An iterator over a report and all its descendant reports in depth-first
/// order.
///
/// This iterator yields `ReportRef` items, which are references to the reports
/// in the hierarchy. The iterator traverses the report tree in a depth-first
/// manner, starting from the root report and visiting each child report before
/// moving to the next sibling.
#[must_use]
pub struct ReportIter<'a, Ownership: 'static, ThreadSafety: 'static> {
    stack: Vec<ReportRef<'a, dyn Any, Ownership, ThreadSafety>>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, O, T> ReportIter<'a, O, T> {
    /// Creates a new [`ReportIter`] from a vector of raw report references
    pub(crate) fn from_raw(stack: Vec<ReportRef<'a, dyn Any, O, T>>) -> Self {
        Self {
            stack,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T>
where
    ReportRef<'a, dyn Any, Cloneable, T>: Into<ReportRef<'a, dyn Any, O, T>>,
{
    type Item = ReportRef<'a, dyn Any, O, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.stack.pop()?;
        let new_children = cur.children().iter().map(|c| c.into()).rev();
        self.stack.extend(new_children);
        Some(cur)
    }
}

impl<'a, O, T> FusedIterator for ReportIter<'a, O, T> where
    ReportRef<'a, dyn Any, Cloneable, T>: Into<ReportRef<'a, dyn Any, O, T>>
{
}
