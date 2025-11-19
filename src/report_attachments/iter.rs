use core::{any::Any, iter::FusedIterator};

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
    raw: core::slice::Iter<'a, RawAttachment>,
}

impl<'a> ReportAttachmentsIter<'a> {
    /// Creates a new `AttachmentsIter` from an iterator of raw attachments
    pub(crate) fn from_raw(raw: core::slice::Iter<'a, RawAttachment>) -> Self {
        Self { raw }
    }
}

impl<'a> Iterator for ReportAttachmentsIter<'a> {
    type Item = ReportAttachmentRef<'a, dyn Any>;

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.raw.next()?.as_ref();

        // SAFETY:
        // 1. `A = dyn Any`, so this is trivially satisfied.
        let attachment = unsafe { ReportAttachmentRef::<dyn Any>::from_raw(raw) };

        Some(attachment)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.raw.size_hint()
    }
}

impl<'a> DoubleEndedIterator for ReportAttachmentsIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let raw = self.raw.next_back()?.as_ref();

        // SAFETY:
        // 1. `A = dyn Any`, so this is trivially satisfied.
        let attachment = unsafe { ReportAttachmentRef::from_raw(raw) };

        Some(attachment)
    }
}

impl<'a> ExactSizeIterator for ReportAttachmentsIter<'a> {
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<'a> FusedIterator for ReportAttachmentsIter<'a> {}

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::RawAttachment;

    use crate::markers;

    /// An iterator that consumes report attachments and yields owned values.
    ///
    /// This iterator yields [`ReportAttachment`] items and is created by
    /// calling [`ReportAttachments::into_iter`].
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
        /// # Safety
        ///
        /// The following safety invariants must be upheld as long as this
        /// struct exists:
        ///
        /// 1. If `T = SendSync`: All of the inner attachments must be `Send +
        ///    Sync`.
        raw: alloc::vec::IntoIter<RawAttachment>,
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
        /// The caller must ensure:
        ///
        /// 1. If `T = SendSync`: All of the inner attachments must be `Send +
        ///    Sync`.
        pub(crate) unsafe fn from_raw(raw: alloc::vec::IntoIter<RawAttachment>) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            Self {
                raw,
                _thread_safety: PhantomData,
            }
        }

        /// Provides access to the inner raw iterator
        pub(crate) fn as_raw(&self) -> &alloc::vec::IntoIter<RawAttachment> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. No mutation is possible through this reference
            &self.raw
        }

        /// Provides mutable access to the inner raw iterator
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `T = SendSync`: No mutation is performed that invalidate the
        ///    invariant that all inner attachments are `Send + Sync`.
        pub(crate) unsafe fn as_raw_mut(&mut self) -> &mut alloc::vec::IntoIter<RawAttachment> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            &mut self.raw
        }
    }
}
pub use limit_field_access::ReportAttachmentsIntoIter;

impl<T> Iterator for ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    type Item = ReportAttachment<dyn Any, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // 1. We do not mutate the iterator or add any additional attachments during
        //    this call.
        let raw = unsafe { self.as_raw_mut() };

        let attachment = raw.next()?;

        // SAFETY:
        // 1. `A=dyn Any`, so this is trivially satisfied.
        // 2. Guaranteed by the invariants of this type.
        let report_attachment = unsafe { ReportAttachment::<dyn Any, T>::from_raw(attachment) };

        Some(report_attachment)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.as_raw().size_hint()
    }
}

impl<T> DoubleEndedIterator for ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // 1. We do not mutate the iterator or add any additional attachments during
        //    this call.
        let raw = unsafe { self.as_raw_mut() };

        let attachment = raw.next_back()?;

        // SAFETY:
        // 1. `A=dyn Any`, so this is trivially satisfied.
        // 2. Guaranteed by the invariants of this type.
        let attachment = unsafe { ReportAttachment::from_raw(attachment) };

        Some(attachment)
    }
}

impl<T> ExactSizeIterator for ReportAttachmentsIntoIter<T>
where
    T: markers::ThreadSafetyMarker,
{
    fn len(&self) -> usize {
        self.as_raw().len()
    }
}

impl<T> FusedIterator for ReportAttachmentsIntoIter<T> where T: markers::ThreadSafetyMarker {}
