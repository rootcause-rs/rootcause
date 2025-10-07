use alloc::vec::Vec;
use core::{any::Any, iter::FusedIterator, marker::PhantomData};

use rootcause_internals::RawReportRef;

use crate::{markers, report::ReportRef};

pub struct ReportIter<'a, Ownership, ThreadSafety>
where
    Ownership: markers::ReportRefOwnershipMarker,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    stack: Vec<RawReportRef<'a>>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, O, T> ReportIter<'a, O, T>
where
    O: crate::markers::ReportRefOwnershipMarker,
    T: crate::markers::ThreadSafetyMarker,
{
    pub(crate) unsafe fn from_raw(stack: Vec<RawReportRef<'a>>) -> Self {
        Self {
            stack,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
        }
    }
}

impl<'a, O, T> Iterator for ReportIter<'a, O, T>
where
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    type Item = ReportRef<'a, dyn Any, markers::Cloneable, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.stack.pop()?;
        self.stack
            .extend(cur.children().iter().map(|r| r.as_ref()).rev());
        Some(unsafe { ReportRef::from_raw(cur) })
    }
}

impl<'a, O, T> FusedIterator for ReportIter<'a, O, T>
where
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
}
