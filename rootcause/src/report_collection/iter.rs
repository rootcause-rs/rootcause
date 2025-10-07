use alloc::vec::Vec;
use core::{any::Any, iter::FusedIterator, marker::PhantomData};

use rootcause_internals::RawReport;

use crate::{
    markers::{self, Cloneable, SendSync},
    report::{Report, ReportRef},
};

pub struct ReportCollectionIter<
    'a,
    Context: markers::ObjectMarker + ?Sized = dyn Any,
    ThreadSafety: markers::ThreadSafetyMarker = SendSync,
> {
    iter: core::slice::Iter<'a, RawReport>,
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, C, T> ReportCollectionIter<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    pub(crate) unsafe fn from_raw(raw: &'a [RawReport]) -> Self {
        Self {
            iter: raw.iter(),
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<'a, C, T> Iterator for ReportCollectionIter<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type Item = ReportRef<'a, C, Cloneable, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?.as_ref();
        unsafe { Some(ReportRef::from_raw(item)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, C, T> DoubleEndedIterator for ReportCollectionIter<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.iter.next_back()?.as_ref();
        unsafe { Some(ReportRef::from_raw(item)) }
    }
}

impl<'a, C, T> ExactSizeIterator for ReportCollectionIter<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, C, T> FusedIterator for ReportCollectionIter<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
}

pub struct ReportCollectionIntoIter<Context: ?Sized + 'static = dyn Any, ThreadSafety = SendSync> {
    iter: alloc::vec::IntoIter<RawReport>,
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<C, T> ReportCollectionIntoIter<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    pub(crate) unsafe fn from_raw(raw: Vec<RawReport>) -> Self {
        Self {
            iter: raw.into_iter(),
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<C, T> Iterator for ReportCollectionIntoIter<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type Item = Report<C, Cloneable, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        unsafe { Some(Report::from_raw(item)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<C, T> DoubleEndedIterator for ReportCollectionIntoIter<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.iter.next_back()?;
        unsafe { Some(Report::from_raw(item)) }
    }
}

impl<C, T> ExactSizeIterator for ReportCollectionIntoIter<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<C, T> FusedIterator for ReportCollectionIntoIter<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
}
