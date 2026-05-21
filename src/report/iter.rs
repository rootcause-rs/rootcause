use alloc::vec::Vec;
use core::{iter::FusedIterator, marker::PhantomData};

use crate::{ReportRef, markers::Dynamic};

/// An iterator over a report and all its descendant reports in depth-first
/// order.
///
/// This iterator yields [`ReportRef`] items, which are references to the reports
/// in the hierarchy. The iterator traverses the report tree in a depth-first
/// manner, starting from the root report and visiting each child report before
/// moving to the next sibling.
#[must_use]
pub struct ReportIter<'a, Ownership: 'static, ThreadSafety: 'static> {
    stack: Vec<ReportRef<'a, Dynamic, Ownership, ThreadSafety>>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, O, T> ReportIter<'a, O, T> {
    /// Creates a new [`ReportIter`] from a vector of raw report references
    pub(crate) fn from_raw(stack: Vec<ReportRef<'a, Dynamic, O, T>>) -> Self {
        Self {
            stack,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T> {
    type Item = ReportRef<'a, Dynamic, O, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur: ReportRef<'a, Dynamic, O, T> = self.stack.pop()?;

        let new_children = cur
            .children()
            .iter()
            .map(|child_report| {
                // SAFETY:
                // 1. At this point we have an instance of a `ReportRef<'a, Dynamic, O, T>` in
                //    scope.  This means we can invoke the safety invariants of that ReportRef.
                //    One of the safety invariants of that `ReportRef` is that `O` must either
                //    be `Cloneable` or `Uncloneable`. But this fulfills our requirements for
                //    calling `ReportRef::from_cloneable` using that same `O`.
                unsafe {
                    // @add-unsafe-context: Dynamic
                    ReportRef::<Dynamic, O, T>::from_cloneable(child_report)
                }
            })
            .rev();
        self.stack.extend(new_children);
        Some(cur)
    }
}

impl<'a, O, T> FusedIterator for ReportIter<'a, O, T> {}

impl<'a, O, T> Unpin for ReportIter<'a, O, T> {}

/// An iterator over all contexts that can successfully be downcasted to [`D`], belonging
/// a report and all its decendants in a depth-first order.
///
/// This iterator yields [`&D`] items, which are references to the reports' contexts
/// in the hierarchy.
pub struct DowncastIterator<'a, D, Ownership: 'static, ThreadSafety: 'static> {
    pub(crate) iter: ReportIter<'a, Ownership, ThreadSafety>,
    pub(crate) _phantom: PhantomData<D>,
}

impl<'a, D: 'static, Ownership: 'static, ThreadSafety: 'static> Iterator
    for DowncastIterator<'a, D, Ownership, ThreadSafety>
{
    type Item = &'a D;

    fn next(&mut self) -> Option<Self::Item> {
        for report in self.iter.by_ref() {
            let Some(report) = report.downcast_current_context() else {
                continue;
            };
            return Some(report);
        }

        None
    }
}

impl<'a, D: 'static, O, T> FusedIterator for DowncastIterator<'a, D, O, T> {}

impl<'a, D, O, T> Unpin for DowncastIterator<'a, D, O, T> {}
