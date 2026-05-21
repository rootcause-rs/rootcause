use alloc::collections::vec_deque::VecDeque;
use core::{iter::FusedIterator, marker::PhantomData};

use crate::{ReportRef, markers::Dynamic, report_collection::ReportCollection};

/// An iterator over a report and all its descendant reports.
///
/// This iterator yields [`ReportRef`] items, which are references to the reports
/// in the hierarchy. By default the traversal is depth-first; use
/// [`ReportIter::bfs`] to switch to breadth-first.
#[must_use]
pub struct ReportIter<'a, Ownership: 'static, ThreadSafety: 'static, Strategy = Dfs> {
    stack: VecDeque<ReportRef<'a, Dynamic, Ownership, ThreadSafety>>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
    _traversal: PhantomData<Strategy>,
}

impl<'a, O, T, S> ReportIter<'a, O, T, S> {
    /// Creates a new [`ReportIter`] from a deque of raw report references
    pub(crate) fn from_raw(stack: VecDeque<ReportRef<'a, Dynamic, O, T>>) -> Self {
        Self {
            stack,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
            _traversal: PhantomData,
        }
    }
}

impl<'a, O, T> ReportIter<'a, O, T, Dfs> {
    /// Convert this traversal to a breadth-first search.
    ///
    /// **Warning:** if this function is called mid-traversal,
    /// the result is unspecified behavior. Nothing unsound will
    /// happen, but the traversal order will not be guaranteed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rootcause::{report, ReportIter, report_collection::ReportCollection};
    /// let rep = /*
    ///         root
    ///       /     \
    ///     1         2
    ///    / \      /  \
    ///  1.1 1.2  2.1 2.2 */
    /// # (1..=2).map(|i| {
    /// #     (1..=2)
    /// #         .map(|j| report!(format!("{}.{}", i, j)).into_cloneable())
    /// #         .collect::<ReportCollection>()
    /// #         .context(format!("{}", i))
    /// # })
    /// # .collect::<ReportCollection>()
    /// # .context("root").into_dynamic()
    /// ;
    /// assert_eq!(
    ///     rep.iter_reports().bfs()
    ///         .map(|e| e.format_current_context().to_string())
    ///         .collect::<Vec<_>>(),
    ///     &["root", "1", "2", "1.1", "1.2", "2.1", "2.2"]
    /// );
    /// ```
    pub fn bfs(self) -> ReportIter<'a, O, T, Bfs> {
        ReportIter::from_raw(self.stack)
    }
}

impl<'a, O, T> ReportIter<'a, O, T, Bfs> {
    /// Convert this traversal to a depth-first search.
    ///
    /// **Warning:** if this function is called mid-traversal,
    /// the result is unspecified behavior. Nothing unsound will
    /// happen, but the traversal order will not be guaranteed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rootcause::{report, ReportIter, report_collection::ReportCollection};
    /// let rep = /*
    ///         root
    ///       /     \
    ///     1         2
    ///    / \      /  \
    ///  1.1 1.2  2.1 2.2 */
    /// # (1..=2).map(|i| {
    /// #     (1..=2)
    /// #         .map(|j| report!(format!("{}.{}", i, j)).into_cloneable())
    /// #         .collect::<ReportCollection>()
    /// #         .context(format!("{}", i))
    /// # })
    /// # .collect::<ReportCollection>()
    /// # .context("root").into_dynamic()
    /// ;
    /// assert_eq!(
    ///     rep.iter_reports()
    ///         .map(|e| e.format_current_context().to_string())
    ///         .collect::<Vec<_>>(),
    ///     &["root", "1", "1.1", "1.2", "2", "2.1", "2.2"]
    /// );
    /// ```
    pub fn dfs(self) -> ReportIter<'a, O, T, Dfs> {
        ReportIter::from_raw(self.stack)
    }
}

/// Marker type for depth-first traversal in the [`ReportIter`] type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dfs;

/// Marker type for breadth-first traversal in the [`ReportIter`] type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bfs;

fn list_children<'a, O: 'static, T>(
    children: &'a ReportCollection<Dynamic, T>,
) -> impl DoubleEndedIterator<Item = ReportRef<'a, Dynamic, O, T>> {
    children.iter().map(|child_report| {
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
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T, Dfs> {
    type Item = ReportRef<'a, Dynamic, O, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur: ReportRef<'a, Dynamic, O, T> = self.stack.pop_back()?;
        self.stack.extend(list_children(cur.children()).rev());
        Some(cur)
    }
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T, Bfs> {
    type Item = ReportRef<'a, Dynamic, O, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur: ReportRef<'a, Dynamic, O, T> = self.stack.pop_front()?;
        self.stack.extend(list_children(cur.children()));
        Some(cur)
    }
}

impl<'a, O, T> FusedIterator for ReportIter<'a, O, T, Dfs> {}
impl<'a, O, T> FusedIterator for ReportIter<'a, O, T, Bfs> {}

impl<'a, O, T, S> Unpin for ReportIter<'a, O, T, S> {}

/// An iterator over all contexts that can successfully be downcasted to `D`, belonging
/// a report and all its decendants in a depth-first order.
///
/// This iterator yields `&D` items, which are references to the reports' contexts
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
