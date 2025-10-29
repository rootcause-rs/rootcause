use core::{any::Any, iter::FusedIterator, marker::PhantomData};

use rootcause_internals::RawAttachment;

use crate::{
    markers,
    report_attachment::{ReportAttachment, ReportAttachmentRef},
};

/// An iterator over references to report attachments.
///
/// This iterator yields [`ReportAttachmentRef`] items and is created by calling
/// [`ReportAttachments::iter`].
///
/// [`ReportAttachmentRef`]: crate::report_attachment::ReportAttachmentRef
/// [`ReportAttachments::iter`]: crate::report_attachments::ReportAttachments::iter
///
/// # Examples
///
/// ```
/// use rootcause::{
///     report_attachment::ReportAttachment,
///     report_attachments::{ReportAttachments, ReportAttachmentsIter},
/// };
///
/// let mut attachments = ReportAttachments::new_sendsync();
/// attachments.push(ReportAttachment::new("debug info").into_dyn_any());
/// attachments.push(ReportAttachment::new(42).into_dyn_any());
///
/// let iterator: ReportAttachmentsIter<'_> = attachments.iter();
/// ```
#[must_use]
pub struct ReportAttachmentsIter<'a> {
    iter: core::slice::Iter<'a, RawAttachment>,
}

impl<'a> ReportAttachmentsIter<'a> {
    /// Creates a new `AttachmentsIter` from an iterator of raw attachments
    pub(crate) fn from_raw(iter: core::slice::Iter<'a, RawAttachment>) -> Self {
        Self { iter }
    }
}

impl<'a> Iterator for ReportAttachmentsIter<'a> {
    type Item = ReportAttachmentRef<'a, dyn Any>;

    fn next(&mut self) -> Option<Self::Item> {
        let attachment = self.iter.next()?.as_ref();
        unsafe { Some(ReportAttachmentRef::from_raw(attachment)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for ReportAttachmentsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let attachment = self.iter.next_back()?.as_ref();
        unsafe { Some(ReportAttachmentRef::from_raw(attachment)) }
    }
}

impl<'a> ExactSizeIterator for ReportAttachmentsIter<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> FusedIterator for ReportAttachmentsIter<'a> {}

/// An iterator that consumes report attachments and yields owned values.
///
/// This iterator yields [`ReportAttachment`] items and is created by calling
/// [`ReportAttachments::into_iter`].
///
/// [`ReportAttachment`]: crate::report_attachment::ReportAttachment
/// [`ReportAttachments::into_iter`]: crate::report_attachments::ReportAttachments::into_iter
///
/// # Examples
///
/// ```
/// use rootcause::{
///     report_attachment::ReportAttachment,
///     report_attachments::{ReportAttachments, ReportAttachmentsIntoIter},
/// };
///
/// let mut attachments = ReportAttachments::new_sendsync();
/// attachments.push(ReportAttachment::new("debug info").into_dyn_any());
/// attachments.push(ReportAttachment::new(42).into_dyn_any());
///
/// let iterator: ReportAttachmentsIntoIter<_> = attachments.into_iter();
/// ```
#[must_use]
pub struct ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    iter: alloc::vec::IntoIter<RawAttachment>,
    _thread_safety: PhantomData<T>,
}

impl<T> ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new [`ReportAttachmentsIntoIter`] from an iterator of raw
    /// attachments
    ///
    /// # Safety
    ///
    /// The thread safety marker must match the contents of the attachments.
    /// More specifically if the marker is `SendSync`, then all of the inner
    /// attachments must be `Send+Sync`
    pub(crate) unsafe fn from_raw(iter: alloc::vec::IntoIter<RawAttachment>) -> Self {
        Self {
            iter,
            _thread_safety: PhantomData,
        }
    }
}

impl<T> Iterator for ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    type Item = ReportAttachment<dyn Any, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let attachment = self.iter.next()?;
        unsafe { Some(ReportAttachment::from_raw(attachment)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let attachment = self.iter.next_back()?;
        unsafe { Some(ReportAttachment::from_raw(attachment)) }
    }
}

impl<T> ExactSizeIterator for ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T> FusedIterator for ReportAttachmentsIntoIter<T> where T: markers::ThreadSafetyMarker {}
