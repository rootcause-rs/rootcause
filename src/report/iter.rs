use alloc::vec::Vec;
use core::{iter::FusedIterator, marker::PhantomData};

use crate::{ReportRef, markers::Dynamic};

/// An iterator over a report and all its descendant reports in depth-first
/// pre-order.
///
/// This iterator yields [`ReportRef`] items, which are references to the reports
/// in the hierarchy. The traversal order is guaranteed to be depth-first
/// pre-order: each report is yielded before its descendants, and each child's
/// entire subtree is yielded before the next sibling. Children are visited in
/// the order they appear in [`ReportRef::children`].
///
/// # Examples
///
/// ```
/// # use rootcause::prelude::*;
/// # let mut a: Report = report!("a");
/// # a.children_mut().push(report!("a1").into_cloneable());
/// # a.children_mut().push(report!("a2").into_cloneable());
/// # let mut b: Report = report!("b");
/// # b.children_mut().push(report!("b1").into_cloneable());
/// # let mut root: Report = report!("root");
/// # root.children_mut().push(a.into_cloneable());
/// # root.children_mut().push(b.into_cloneable());
/// // The report tree:      root
/// //                      /    \
/// //                     a      b
/// //                    / \      \
/// //                   a1  a2     b1
/// let order: Vec<String> = root
///     .iter_reports()
///     .map(|report| report.format_current_context().to_string())
///     .collect();
/// assert_eq!(order, ["root", "a", "a1", "a2", "b", "b1"]);
/// ```
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

/// An iterator over all contexts that can successfully be downcasted to `D`,
/// belonging to a report and all its descendants.
///
/// This iterator yields `&D` items, which are references to the reports' contexts
/// in the hierarchy. Matches are yielded in the same guaranteed depth-first
/// pre-order as [`ReportIter`].
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
