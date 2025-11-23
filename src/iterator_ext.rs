use alloc::vec::Vec;
use core::{iter::FusedIterator, mem};

use crate::{IntoReport, report_collection::ReportCollection};

/// Extension methods for iterators over `Result` types to collect errors.
///
/// This trait provides methods to collect successful values while accumulating
/// all errors into a [`ReportCollection`], rather than stopping at the first
/// error like [`Iterator::collect`] does.
///
/// # When to Use
///
/// Use these methods when you want to:
/// - Process all items in an iterator, even if some fail
/// - Collect all errors that occurred, not just the first one
/// - Validate multiple items and report all validation failures at once
///
/// # Comparison to Standard Collect
///
/// The standard library's [`Iterator::collect`] stops at the first error, while
/// these methods continue processing and collect all errors:
///
/// ```rust
/// use rootcause::{prelude::*, report_collection::ReportCollection};
///
/// let inputs = vec!["1", "2", "invalid", "4", "bad"];
///
/// // Standard collect stops at first error
/// let standard: Result<Vec<u8>, _> = inputs.iter().map(|s| s.parse::<u8>()).collect();
/// assert!(standard.is_err()); // Stopped at "invalid", never saw "bad"
///
/// // collect_reports_vec processes ALL items and collects ALL errors
/// let result: Result<Vec<u8>, ReportCollection<std::num::ParseIntError>> = inputs
///     .into_iter()
///     .map(|s| s.parse::<u8>())
///     .collect_reports_vec();
///
/// assert!(result.is_err());
/// let all_errors = result.unwrap_err();
/// assert_eq!(all_errors.len(), 2); // Both "invalid" and "bad" collected
/// ```
pub trait IteratorExt<A, E>: Sized + Iterator<Item = Result<A, E>> {
    /// Collects successful values into a container, or all errors into a
    /// [`ReportCollection`].
    ///
    /// This method processes the entire iterator, collecting all `Ok` values
    /// into the specified container type. If any `Err` values are
    /// encountered, iteration continues to collect **all** errors before
    /// returning them in a [`ReportCollection`].
    ///
    /// This is different from using [`Iterator::collect`] on `Result`s, which
    /// stops at the first error.
    ///
    /// # Type Parameters
    ///
    /// - `Container`: The collection type for successful values (e.g., `Vec`,
    ///   `HashSet`)
    /// - `ThreadSafety`: The thread-safety marker for the error collection
    ///
    /// # Returns
    ///
    /// - `Ok(Container)`: If all items were successful
    /// - `Err(ReportCollection)`: If any errors occurred, containing all of
    ///   them
    ///
    /// # Examples
    ///
    /// ## Collecting into Different Container Types
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use rootcause::{prelude::*, report_collection::ReportCollection};
    ///
    /// let inputs = vec!["1", "2", "foo", "4", "bar"];
    /// let result: Result<BTreeSet<u8>, ReportCollection<std::num::ParseIntError>> = inputs
    ///     .into_iter()
    ///     .map(|s| s.parse::<u8>())
    ///     .collect_reports();
    ///
    /// // All errors were collected
    /// assert!(result.is_err());
    /// let errors = result.unwrap_err();
    /// assert_eq!(errors.len(), 2); // "foo" and "bar" both failed
    /// ```
    ///
    /// ## All Successful Items
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use rootcause::{prelude::*, report_collection::ReportCollection};
    ///
    /// let inputs = vec!["1", "2", "2"];
    /// let result: Result<BTreeSet<u8>, ReportCollection<std::num::ParseIntError>> = inputs
    ///     .into_iter()
    ///     .map(|s| s.parse::<u8>())
    ///     .collect_reports();
    ///
    /// assert!(result.is_ok());
    /// let values = result.unwrap();
    /// assert_eq!(values, BTreeSet::from([1u8, 2]));
    /// ```
    #[track_caller]
    fn collect_reports<Container, ThreadSafety>(
        self,
    ) -> Result<Container, ReportCollection<E::Context, ThreadSafety>>
    where
        Container: FromIterator<A>,
        E: IntoReport<ThreadSafety>;

    /// Collects successful values into a `Vec`, or all errors into a
    /// [`ReportCollection`].
    ///
    /// This is a specialized version of
    /// [`collect_reports`](IteratorExt::collect_reports) that always
    /// collects into a `Vec`. It may help with type inference and could
    /// generate slightly more optimized code.
    ///
    /// # When to Use
    ///
    /// Use this method instead of
    /// [`collect_reports`](IteratorExt::collect_reports) when:
    /// - You're collecting into a `Vec` and want simpler type annotations
    /// - You're in performance-critical code and want potential optimization
    /// - Type inference is having trouble with the generic container parameter
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<A>)`: If all items were successful
    /// - `Err(ReportCollection)`: If any errors occurred, containing all of
    ///   them
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{prelude::*, report_collection::ReportCollection};
    ///
    /// let inputs = vec!["1", "2", "foo", "2", "bar"];
    /// let result: Result<Vec<u8>, ReportCollection<std::num::ParseIntError>> = inputs
    ///     .into_iter()
    ///     .map(|s| s.parse::<u8>())
    ///     .collect_reports_vec();
    ///
    /// assert!(result.is_err());
    /// let errors = result.unwrap_err();
    /// assert_eq!(errors.len(), 2); // Both "foo" and "bar" failed
    /// ```
    #[track_caller]
    fn collect_reports_vec<ThreadSafety>(
        self,
    ) -> Result<Vec<A>, ReportCollection<E::Context, ThreadSafety>>
    where
        E: IntoReport<ThreadSafety>;
}

struct IteratorWrapper<'a, Iter, Error, ThreadSafety: 'static>
where
    Error: IntoReport<ThreadSafety>,
{
    iter: Iter,
    error_collection: &'a mut Option<ReportCollection<Error::Context, ThreadSafety>>,
}

impl<'a, Iter, ThreadSafety, Object, Error> Iterator
    for IteratorWrapper<'a, Iter, Error, ThreadSafety>
where
    Iter: Iterator<Item = Result<Object, Error>>,
    Error: IntoReport<ThreadSafety>,
{
    type Item = Object;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.error_collection.is_some() {
            return None;
        }

        match self.iter.next() {
            Some(Ok(object)) => Some(object),
            Some(Err(err)) => {
                *self.error_collection = Some(ReportCollection::from_iter(
                    core::iter::once(err)
                        .chain((&mut self.iter).filter_map(|v| v.err()))
                        .map(|err| err.into_report().into_cloneable()),
                ));
                None
            }
            None => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.error_collection.is_some() {
            (0, Some(0))
        } else {
            let (_, upper) = self.iter.size_hint();
            (0, upper)
        }
    }
}

impl<'a, Iter, ThreadSafety, Object, Error> FusedIterator
    for IteratorWrapper<'a, Iter, Error, ThreadSafety>
where
    Iter: FusedIterator<Item = Result<Object, Error>>,
    Error: IntoReport<ThreadSafety>,
{
}

impl<A, E, I> IteratorExt<A, E> for I
where
    I: Iterator<Item = Result<A, E>>,
{
    #[inline]
    fn collect_reports<Container, ThreadSafety>(
        self,
    ) -> Result<Container, ReportCollection<E::Context, ThreadSafety>>
    where
        Container: FromIterator<A>,
        E: IntoReport<ThreadSafety>,
    {
        let mut error_collection = None;
        let result = Container::from_iter(IteratorWrapper {
            iter: self,
            error_collection: &mut error_collection,
        });
        if let Some(error_collection) = error_collection {
            Err(error_collection)
        } else {
            Ok(result)
        }
    }

    #[inline]
    fn collect_reports_vec<ThreadSafety>(
        mut self,
    ) -> Result<Vec<A>, ReportCollection<E::Context, ThreadSafety>>
    where
        E: IntoReport<ThreadSafety>,
    {
        let mut out = Vec::new();
        while let Some(v) = self.next() {
            match v {
                Ok(v) => out.push(v),
                Err(err) => {
                    mem::drop(out);
                    let mut collection = ReportCollection::new();
                    collection.push(err.into_report().into_cloneable());
                    collection.extend(
                        self.filter_map(|v| v.err())
                            .map(|e| e.into_report().into_cloneable()),
                    );
                    return Err(collection);
                }
            }
        }
        Ok(out)
    }
}
