use alloc::vec::Vec;
use core::{any::Any, marker::PhantomData};

use rootcause_internals::RawAttachment;

use crate::{
    markers::{self, Local, SendSync},
    report_attachment::{ReportAttachment, ReportAttachmentRef},
    report_attachments::{ReportAttachmentsIntoIter, ReportAttachmentsIter, ReportAttachmentsRef},
};

#[repr(transparent)]
pub struct ReportAttachments<ThreadSafety = SendSync>
where
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: Vec<RawAttachment>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<T> ReportAttachments<T>
where
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new Attachments from a vector of raw attachments
    ///
    /// # Safety
    ///
    /// The thread safety marker must match the contents of the attachments.
    /// More specifically if the marker is `SendSync`, then all the inner
    /// attachments must be `Send+Sync`
    pub(crate) unsafe fn from_raw(raw: Vec<RawAttachment>) -> Self {
        Self {
            raw,
            _thread_safety: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> Vec<RawAttachment> {
        self.raw
    }

    pub fn new() -> Self {
        unsafe { Self::from_raw(Vec::new()) }
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

    pub fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn get(&self, index: usize) -> Option<ReportAttachmentRef<'_, dyn Any>> {
        let attachment = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportAttachmentRef::from_raw(attachment)) }
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn iter(&self) -> ReportAttachmentsIter<'_> {
        self.as_ref().into_iter()
    }

    pub fn as_ref(&self) -> ReportAttachmentsRef<'_, T> {
        unsafe { ReportAttachmentsRef::from_raw(self.raw.as_slice()) }
    }

    pub fn into_local(self) -> ReportAttachments<Local> {
        unsafe { ReportAttachments::from_raw(self.into_raw()) }
    }
}

impl ReportAttachments<SendSync> {
    pub fn new_sendsync() -> Self {
        Self::new()
    }
}

impl ReportAttachments<Local> {
    pub fn new_local() -> Self {
        Self::new()
    }
}

impl Default for ReportAttachments<SendSync> {
    fn default() -> Self {
        Self::new_sendsync()
    }
}

impl Default for ReportAttachments<Local> {
    fn default() -> Self {
        Self::new_local()
    }
}

impl From<ReportAttachments<SendSync>> for ReportAttachments<Local> {
    fn from(attachments: ReportAttachments<SendSync>) -> Self {
        let attachments = attachments.into_raw();

        // SAFETY: We are turning our `SendSync` attachments into `Local` attachments,
        // which is allowed since all `SendSync` attachments are also valid
        // `Local` attachments.
        unsafe { ReportAttachments::from_raw(attachments) }
    }
}

impl<A, T> From<Vec<ReportAttachment<A, T>>> for ReportAttachments<T>
where
    A: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(attachments: Vec<ReportAttachment<A, T>>) -> Self {
        let raw_attachments = attachments.into_iter().map(|v| v.into_raw()).collect();
        unsafe { ReportAttachments::from_raw(raw_attachments) }
    }
}

impl<const N: usize, A, T> From<[ReportAttachment<A, T>; N]> for ReportAttachments<T>
where
    A: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(attachments: [ReportAttachment<A, T>; N]) -> Self {
        let raw_attachments = attachments.into_iter().map(|v| v.into_raw()).collect();
        unsafe { ReportAttachments::from_raw(raw_attachments) }
    }
}

unsafe impl Send for ReportAttachments<SendSync> {}
unsafe impl Sync for ReportAttachments<SendSync> {}

impl<T> IntoIterator for ReportAttachments<T>
where
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportAttachmentsIntoIter<T>;
    type Item = ReportAttachment<dyn Any, T>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { ReportAttachmentsIntoIter::from_raw(self.raw.into_iter()) }
    }
}

impl<A, T> Extend<ReportAttachment<A, T>> for ReportAttachments<T>
where
    A: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = ReportAttachment<A, T>>>(&mut self, iter: I) {
        for report in iter {
            self.push(report.into_dyn_any());
        }
    }
}

impl<A, T> FromIterator<ReportAttachment<A, T>> for ReportAttachments<T>
where
    A: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = ReportAttachment<A, T>>>(iter: I) -> Self {
        let mut siblings = ReportAttachments::new();
        siblings.extend(iter);
        siblings
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_attachments_send_sync() {
        static_assertions::assert_impl_all!(ReportAttachments<SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachments<Local>: Send, Sync);
    }

    #[test]
    fn test_attachments_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportAttachments<SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachments<Local>: Copy, Clone);
    }
}
