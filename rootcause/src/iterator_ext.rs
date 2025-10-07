use alloc::vec::Vec;
use core::mem;

use crate::{markers, report::Report, report_collection::ReportCollection};

pub trait IteratorExt<A, E>: Sized + Iterator<Item = Result<A, E>> {
    fn collect_reports<C, O, T>(self) -> Result<Vec<A>, ReportCollection<C, T>>
    where
        C: crate::markers::ObjectMarkerFor<T>,
        T: crate::markers::ThreadSafetyMarker,
        E: Into<Report<C, markers::Cloneable, T>>;
}

impl<A, E, I> IteratorExt<A, E> for I
where
    I: Iterator<Item = Result<A, E>>,
{
    fn collect_reports<C, O, T>(mut self) -> Result<Vec<A>, ReportCollection<C, T>>
    where
        C: crate::markers::ObjectMarkerFor<T>,
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
