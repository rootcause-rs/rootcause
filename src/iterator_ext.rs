use alloc::vec::Vec;
use core::{iter::FusedIterator, mem};

use crate::{IntoReport, markers, report_collection::ReportCollection};

/// Extension trait for iterators over `Result`s to collect errors into a
/// `ReportCollection`.
pub trait IteratorExt<A, E>: Sized + Iterator<Item = Result<A, E>> {
    /// Collect all `Ok` values into a `Container`. If any `Err` values are
    /// encountered, stop iteration and return a `ReportCollection`
    /// containing all encountered errors.
    ///
    /// This is similar to using [`Iterator::collect`] to collect a
    /// `Result<Container, E>`, but instead of returning early in case of an
    /// error, this will iterate through all the values and collect
    /// all errors into a single [`ReportCollection`].
    ///
    /// # Examples
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use rootcause::{prelude::*, report_collection::ReportCollection};
    ///
    /// let inputs = vec!["1", "2", "foo", "4", "bar"];
    /// let results: Result<
    ///     BTreeSet<u8>,
    ///     ReportCollection<std::num::ParseIntError, markers::SendSync>,
    /// > = inputs
    ///     .into_iter()
    ///     .map(|s| s.parse::<u8>())
    ///     .collect_reports();
    /// assert!(results.is_err());
    /// let errors = results.unwrap_err();
    /// assert_eq!(errors.len(), 2);
    ///
    /// let inputs = vec!["1", "2", "2"];
    /// let results: Result<
    ///     BTreeSet<u8>,
    ///     ReportCollection<std::num::ParseIntError, markers::SendSync>,
    /// > = inputs
    ///     .into_iter()
    ///     .map(|s| s.parse::<u8>())
    ///     .collect_reports();
    /// assert!(results.is_ok());
    /// let errors = results.unwrap();
    /// assert_eq!(errors, BTreeSet::from([1u8, 2]));
    /// ```
    #[track_caller]
    fn collect_reports<Container, ThreadSafety>(
        self,
    ) -> Result<Container, ReportCollection<E::Context, ThreadSafety>>
    where
        Container: FromIterator<A>,
        ThreadSafety: crate::markers::ThreadSafetyMarker,
        E: IntoReport<ThreadSafety>;

    /// Specialized version of [`IteratorExt::collect_reports`] that only works
    /// for [`Vec`].
    ///
    /// This might help with type inference in some cases. It might also
    /// generate slightly simplier code, which might be useful if you call
    /// this function from performance-critical code.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{prelude::*, report_collection::ReportCollection};
    ///
    /// let inputs = vec!["1", "2", "foo", "2", "bar"];
    /// let results: Result<Vec<u8>, ReportCollection<std::num::ParseIntError, markers::SendSync>> =
    ///     inputs
    ///         .into_iter()
    ///         .map(|s| s.parse::<u8>())
    ///         .collect_reports_vec();
    /// assert!(results.is_err());
    /// let errors = results.unwrap_err();
    /// assert_eq!(errors.len(), 2);
    ///
    /// let inputs = vec!["1", "2", "2"];
    /// let results: Result<Vec<u8>, ReportCollection<std::num::ParseIntError, markers::SendSync>> =
    ///     inputs
    ///         .into_iter()
    ///         .map(|s| s.parse::<u8>())
    ///         .collect_reports_vec();
    /// assert!(results.is_ok());
    /// let errors = results.unwrap();
    /// assert_eq!(errors, &[1, 2, 2]);
    /// ```
    #[track_caller]
    fn collect_reports_vec<ThreadSafety>(
        self,
    ) -> Result<Vec<A>, ReportCollection<E::Context, ThreadSafety>>
    where
        ThreadSafety: crate::markers::ThreadSafetyMarker,
        E: IntoReport<ThreadSafety>;
}

struct IteratorWrapper<'a, Iter, Error, ThreadSafety>
where
    Error: IntoReport<ThreadSafety>,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    iter: Iter,
    error_collection: &'a mut Option<ReportCollection<Error::Context, ThreadSafety>>,
}

impl<'a, Iter, ThreadSafety, Object, Error> Iterator
    for IteratorWrapper<'a, Iter, Error, ThreadSafety>
where
    ThreadSafety: markers::ThreadSafetyMarker,
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
    ThreadSafety: markers::ThreadSafetyMarker,
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
        ThreadSafety: crate::markers::ThreadSafetyMarker,
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
        ThreadSafety: crate::markers::ThreadSafetyMarker,
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
