use core::{any::Any, marker::PhantomData};

use rootcause_internals::{RawReport, handlers::FormattingFunction};

use crate::{
    ReportRef,
    markers::{self, Cloneable, Local, SendSync},
    report_collection::ReportCollectionIter,
};

#[repr(transparent)]
pub struct ReportCollectionRef<'a, Context = dyn Any, ThreadSafety = SendSync>
where
    Context: markers::ObjectMarker + ?Sized,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: &'a [RawReport],
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, C, T> Copy for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
}
impl<'a, C, T> Clone for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C, T> ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new ReportCollectionRef from a slice of raw reports
    ///
    /// # Safety
    ///
    /// - The thread safety marker must match the contents of the reports. More
    ///   specifically if the marker is `SendSync`, then all the data
    ///   (recursively) contained by the reports must be `Send+Sync`.
    /// - The caller must ensure that the contexts of the `RawReport`s are
    ///   actually of type `C` when `C` if is is a type different from `dyn
    ///   Any`.
    pub(crate) unsafe fn from_raw(raw: &'a [RawReport]) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> &'a [RawReport] {
        self.raw
    }

    pub fn len(self) -> usize {
        self.raw.len()
    }

    pub fn get(self, index: usize) -> Option<ReportRef<'a, C, Cloneable, T>> {
        let raw = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportRef::from_raw(raw)) }
    }

    pub fn is_empty(self) -> bool {
        self.raw.is_empty()
    }

    pub fn iter(self) -> ReportCollectionIter<'a, C, T> {
        unsafe { ReportCollectionIter::from_raw(self.raw) }
    }

    pub fn to_owned(self) -> crate::report_collection::ReportCollection<C, T> {
        self.iter().collect()
    }

    pub fn into_dyn_any(self) -> ReportCollectionRef<'a, dyn Any, T> {
        unsafe { ReportCollectionRef::from_raw(self.into_raw()) }
    }

    pub fn into_local(self) -> ReportCollectionRef<'a, C, Local> {
        unsafe { ReportCollectionRef::from_raw(self.into_raw()) }
    }
}

impl<'a, C, T> IntoIterator for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportCollectionIter<'a, C, T>;
    type Item = ReportRef<'a, C, Cloneable, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, C, T> core::fmt::Display for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reports: &'a [RawReport] = self.raw;
        let reports: &'a [ReportRef<'a, dyn Any, markers::Uncloneable, markers::Local>] =
            unsafe { core::mem::transmute(reports) };
        crate::hooks::report_formatting::format_reports(reports, f, FormattingFunction::Display)
    }
}

impl<'a, C, T> core::fmt::Debug for ReportCollectionRef<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reports: &'a [RawReport] = self.raw;
        let reports: &'a [ReportRef<'_, dyn Any, markers::Uncloneable, markers::Local>] =
            unsafe { core::mem::transmute(reports) };
        crate::hooks::report_formatting::format_reports(reports, f, FormattingFunction::Display)
    }
}

mod from_impls {
    use super::*;

    macro_rules! unsafe_report_collection_to_report_collection {
        ($(
            <
                $($param:ident),*
            >:
            $context1:ty => $context2:ty,
            $thread_safety1:ty => $thread_safety2:ty
        ),* $(,)?) => {
            $(
                impl<'a, $($param),*> From<ReportCollectionRef<'a, $context1, $thread_safety1>> for ReportCollectionRef<'a, $context2, $thread_safety2>
                    where
                        $($param: markers::ObjectMarker)*
                    {
                    fn from(value: ReportCollectionRef<'a, $context1, $thread_safety1>) -> Self {
                        unsafe { ReportCollectionRef::from_raw(value.raw) }
                    }
                }
            )*
        };
    }

    unsafe_report_collection_to_report_collection! {
        <C>: C => C, SendSync => Local,
        <C>: C => dyn Any, SendSync => SendSync,
        <C>: C => dyn Any, SendSync => Local,
        <C>: C => dyn Any, Local => Local,
        <>: dyn Any => dyn Any, SendSync => Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::Local;

    #[test]
    fn test_report_collection_ref_send_sync() {
        static_assertions::assert_not_impl_all!(ReportCollectionRef<'static, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollectionRef<'static, Local>: Send, Sync);
    }

    #[test]
    fn test_report_collection_ref_copy_clone() {
        static_assertions::assert_impl_all!(ReportCollectionRef<'static, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportCollectionRef<'static, Local>: Copy, Clone);
    }
}
