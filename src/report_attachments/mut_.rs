use alloc::vec::Vec;
use core::{any::Any, marker::PhantomData};

use rootcause_internals::RawAttachment;

use crate::{
    markers::{self, SendSync},
    report_attachment::{ReportAttachment, ReportAttachmentRef},
    report_attachments::{ReportAttachments, ReportAttachmentsIter},
};

#[repr(transparent)]
pub struct ReportAttachmentsMut<'a, ThreadSafety = SendSync>
where
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: &'a mut Vec<RawAttachment>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, T> ReportAttachmentsMut<'a, T>
where
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new AttachmentsMut from a mutable reference to a vec of raw
    /// attachments
    pub(crate) unsafe fn from_raw(raw: &'a mut Vec<RawAttachment>) -> Self {
        Self {
            raw,
            _thread_safety: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn push(&mut self, attachment: ReportAttachment<dyn Any, T>) {
        self.raw.push(attachment.into_raw())
    }

    pub fn pop(&mut self) -> Option<ReportAttachment<dyn Any, T>> {
        let attachment = self.raw.pop()?;

        // SAFETY: The thread safety marker matches, because we only
        // contain attachments with a matching thread safety marker
        let attachment = unsafe { ReportAttachment::from_raw(attachment) };

        Some(attachment)
    }

    pub fn take(&mut self) -> ReportAttachments<T> {
        let raw = core::mem::take(self.raw);
        unsafe { ReportAttachments::from_raw(raw) }
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ReportAttachmentRef<'_, dyn Any>> {
        let attachment = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportAttachmentRef::from_raw(attachment)) }
    }

    pub fn iter(&self) -> ReportAttachmentsIter<'_> {
        unsafe { ReportAttachmentsIter::from_raw(self.raw.iter()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::Local;

    #[test]
    fn test_attachments_mut_send_sync() {
        static_assertions::assert_not_impl_any!(ReportAttachmentsMut<'static, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentsMut<'static, Local>: Send, Sync);
    }

    #[test]
    fn test_attachments_mut_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportAttachmentsMut<'static, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachmentsMut<'static, Local>: Copy, Clone);
    }
}
