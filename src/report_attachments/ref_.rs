use core::{any::Any, marker::PhantomData};

use rootcause_internals::RawAttachment;

use crate::{
    markers::{self, SendSync},
    report_attachment::ReportAttachmentRef,
    report_attachments::ReportAttachmentsIter,
};

#[repr(transparent)]
pub struct ReportAttachmentsRef<'a, ThreadSafety = SendSync>
where
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: &'a [RawAttachment],
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, T> Copy for ReportAttachmentsRef<'a, T> where T: markers::ThreadSafetyMarker {}
impl<'a, T> Clone for ReportAttachmentsRef<'a, T>
where
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> ReportAttachmentsRef<'a, T>
where
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new AttachmentsRef from a slice of raw attachments
    ///
    /// # Safety
    ///
    /// The thread safety marker must match the contents of the attachments.
    /// More specifically if the marker is `SendSync`, then all of the inner
    /// attachments must be `Send+Sync`
    pub(crate) unsafe fn from_raw(raw: &'a [RawAttachment]) -> Self {
        Self {
            raw,
            _thread_safety: PhantomData,
        }
    }

    pub fn len(self) -> usize {
        self.raw.len()
    }

    pub fn is_empty(self) -> bool {
        self.raw.is_empty()
    }

    pub fn get(self, index: usize) -> Option<ReportAttachmentRef<'a, dyn Any>> {
        let attachment = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportAttachmentRef::from_raw(attachment)) }
    }

    pub fn iter(self) -> ReportAttachmentsIter<'a> {
        unsafe { ReportAttachmentsIter::from_raw(self.raw.iter()) }
    }
}

impl<'a, T> IntoIterator for ReportAttachmentsRef<'a, T>
where
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportAttachmentsIter<'a>;
    type Item = ReportAttachmentRef<'a, dyn Any>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::Local;

    #[test]
    fn test_attachments_ref_send_sync() {
        static_assertions::assert_not_impl_any!(ReportAttachmentsRef<'static, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentsRef<'static, Local>: Send, Sync);
    }

    #[test]
    fn test_attachments_ref_copy_clone() {
        static_assertions::assert_impl_all!(ReportAttachmentsRef<'static, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentsRef<'static, Local>: Copy, Clone);
    }
}
