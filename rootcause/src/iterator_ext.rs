use alloc::vec::Vec;
use core::mem;

use crate::{markers, report::Report, report_collection::ReportCollection};

/// Extension trait for iterators over `Result`s to collect errors into a `ReportCollection`.
pub trait IteratorExt<A, E>: Sized + Iterator<Item = Result<A, E>> {
    /// Collect all `Ok` values into a `Vec`. If any `Err` values are encountered, stop iteration
    /// and return a `ReportCollection` containing all encountered errors.
    ///
    /// This is similar to `Result::collect` but collects all errors into a single `ReportCollection`
    /// instead of returning the first error encountered.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{prelude::*, report_collection::ReportCollection};
    ///
    /// let inputs = vec!["1", "2", "foo", "4", "bar"];
    /// let results: Result<Vec<u8>, ReportCollection<dyn core::any::Any, markers::SendSync>> = inputs
    ///     .into_iter()
    ///     .map(|s| s.parse::<u8>())
    ///     .collect_reports();
    /// assert!(results.is_err());
    /// let errors = results.unwrap_err();
    /// assert_eq!(errors.len(), 2);
    /// ```
    fn collect_reports<C, T>(self) -> Result<Vec<A>, ReportCollection<C, T>>
    where
        C: ?Sized + crate::markers::ObjectMarkerFor<T>,
        T: crate::markers::ThreadSafetyMarker,
        E: Into<Report<C, markers::Cloneable, T>>;
}

impl<A, E, I> IteratorExt<A, E> for I
where
    I: Iterator<Item = Result<A, E>>,
{
    fn collect_reports<C, T>(mut self) -> Result<Vec<A>, ReportCollection<C, T>>
    where
        C: ?Sized + crate::markers::ObjectMarkerFor<T>,
        T: crate::markers::ThreadSafetyMarker,
        E: Into<Report<C, markers::Cloneable, T>>,
    {
        let mut out = Vec::new();
        while let Some(v) = self.next() {
            match v {
                Ok(v) => out.push(v),
                Err(err) => {
                    mem::drop(out);
                    let mut collection = ReportCollection::new();
                    collection.push(err.into());
                    for v in self {
                        if let Err(err) = v {
                            collection.push(err.into());
                        }
                    }
                    return Err(collection);
                }
            }
        }
        Ok(out)
    }
}
