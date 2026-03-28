use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use core::{iter::FusedIterator, marker::PhantomData};

use crate::{ReportRef, markers::Dynamic, report_collection::ReportCollection};

/// An iterator over a report and all its descendant reports in depth-first
/// order.
///
/// This iterator yields `ReportRef` items, which are references to the reports
/// in the hierarchy. The iterator traverses the report tree in a depth-first
/// manner, starting from the root report and visiting each child report before
/// moving to the next sibling.
#[must_use]
pub struct ReportIter<'a, Ownership: 'static, ThreadSafety: 'static, Strategy: ?Sized = DFS> {
    stack: VecDeque<ReportRef<'a, Dynamic, Ownership, ThreadSafety>>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
    _traversal: PhantomData<*mut Strategy>,
}

impl<'a, O, T, S: ?Sized> ReportIter<'a, O, T, S> {
    /// Creates a new [`ReportIter`] from a vector of raw report references
    pub(crate) fn from_raw(stack: VecDeque<ReportRef<'a, Dynamic, O, T>>) -> Self {
        Self {
            stack,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
            _traversal: PhantomData,
        }
    }
}

#[cfg(test)]
fn report_tree() -> crate::Report {
    use crate::report_collection::ReportCollection;
    use alloc::format;

    (1..=2)
        .map(|i| {
            (1..=2)
                .map(|j| report!(format!("{}.{}", i, j)).into_cloneable())
                .collect::<ReportCollection>()
                .context(format!("{}", i))
        })
        .collect::<ReportCollection>()
        .context(format!("root"))
        .into_dynamic()
}

#[cfg(test)]
fn join_contexts<'b, OW: 'static>(
    it: impl Iterator<Item = ReportRef<'b, Dynamic, OW>>,
) -> alloc::string::String {
    use alloc::string::ToString;
    use alloc::vec::Vec;
    it.into_iter()
        .map(|e: ReportRef<'_, Dynamic, OW>| e.format_current_context().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

impl<'a, O, T> ReportIter<'a, O, T, DFS> {
    /// Convert this traversal to a breadth-first search.
    ///
    /// **Warning:** if this function is called mid-traversal,
    /// the result is unspecified behavior. Nothing unsound will
    /// happen, but the traversal order will not be guaranteed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rootcause::ReportIter;
    /// let rep =
    /// # panic!("HOW THE FUCK DO I CALL MY HELPER FUNCTIONS YOU FUCKING MODULE SYSTEM");
    /// //        root
    /// //      /     \
    /// //    1         2
    /// //   / \      /  \
    /// // 1.1 1.2  2.1 2.2
    /// assert_eq!("", "root 1 1.1 1.2 2 2.1 2.2");
    /// ```
    pub fn bfs(self) -> ReportIter<'a, O, T, BFS> {
        ReportIter::from_raw(self.stack)
    }
}

impl<'a, O, T> ReportIter<'a, O, T, BFS> {
    /// Convert this traversal to a depth-first search.
    ///
    /// **Warning:** if this function is called mid-traversal,
    /// the result is unspecified behavior. Nothing unsound will
    /// happen, but the traversal order will not be guaranteed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rootcause::ReportIter;
    /// let rep =
    /// # panic!("HOW THE FUCK DO I CALL MY HELPER FUNCTIONS YOU FUCKING MODULE SYSTEM");
    /// //        root
    /// //      /     \
    /// //    1         2
    /// //   / \      /  \
    /// // 1.1 1.2  2.1 2.2
    /// assert_eq!("", "root 1 1.1 1.2 2 2.1 2.2");
    /// ```
    pub fn dfs(self) -> ReportIter<'a, O, T, DFS> {
        ReportIter::from_raw(self.stack)
    }
}

/// Marker type for depth-first traversal in the [`ReportIter`] type.
pub struct DFS {
    _not_constructible: NotConstructible,
}

/// Marker type for breadth-first traversal in the [`ReportIter`] type.
pub struct BFS {
    _not_constructible: NotConstructible,
}

#[allow(missing_copy_implementations, reason = "not constructible")]
struct NotConstructible;

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

impl<'a, O, T> Iterator for ReportIter<'a, O, T, DFS> {
    type Item = ReportRef<'a, Dynamic, O, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur: ReportRef<'a, Dynamic, O, T> = self.stack.pop_back()?;
        self.stack.extend(list_children(cur.children()).rev());
        Some(cur)
    }
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T, BFS> {
    type Item = ReportRef<'a, Dynamic, O, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur: ReportRef<'a, Dynamic, O, T> = self.stack.pop_front()?;
        self.stack.extend(list_children(cur.children()));
        Some(cur)
    }
}

impl<'a, O, T> FusedIterator for ReportIter<'a, O, T, DFS> {}
impl<'a, O, T> FusedIterator for ReportIter<'a, O, T, BFS> {}

impl<'a, O, T, S> Unpin for ReportIter<'a, O, T, S> {}
