use core::iter::FusedIterator;

use rootcause_internals::RawAttachment;

use crate::{
    markers::Dynamic,
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
/// attachments.push(ReportAttachment::new("debug info").into_dynamic());
/// attachments.push(ReportAttachment::new(42).into_dynamic());
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
    type Item = ReportAttachmentRef<'a, Dynamic>;

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.raw.next()?.as_ref();

        // SAFETY:
        // 1. `A = Dynamic`, so this is trivially satisfied.
        // 2. `A = Dynamic`, so this is trivially satisfied.
        let attachment = unsafe {
            // @add-unsafe-context: Dynamic
            ReportAttachmentRef::<'a, Dynamic>::from_raw(raw)
        };

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
        // 1. `A = Dynamic`, so this is trivially satisfied.
        // 2. `A = Dynamic`, so this is trivially satisfied.
        let attachment = unsafe {
            // @add-unsafe-context: Dynamic
            ReportAttachmentRef::<'a, Dynamic>::from_raw(raw)
        };

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
    /// attachments.push(ReportAttachment::new("debug info").into_dynamic());
    /// attachments.push(ReportAttachment::new(42).into_dynamic());
    ///
    /// let iterator: ReportAttachmentsIntoIter<_> = attachments.into_iter();
    /// ```
    #[must_use]
    pub struct ReportAttachmentsIntoIter<ThreadMarker: 'static> {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. Either the collection must be empty or `T` must either be
        ///    `SendSync` or `Local`.
        /// 2. If `T = SendSync`: All of the inner attachments must be `Send +
        ///    Sync`.
        raw: alloc::vec::IntoIter<RawAttachment>,
        _thread_safety: PhantomData<ThreadMarker>,
    }

    impl<T> ReportAttachmentsIntoIter<T> {
        /// Creates a new [`ReportAttachmentsIntoIter`] from an iterator of raw
        /// attachments
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. Either the collection must be empty or `T` must either be
        ///    `SendSync` or `Local`.
        /// 2. If `T = SendSync`: All of the inner attachments must be `Send +
        ///    Sync`.
        pub(crate) unsafe fn from_raw(raw: alloc::vec::IntoIter<RawAttachment>) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            Self {
                raw,
                _thread_safety: PhantomData,
            }
        }

        /// Provides access to the inner raw iterator
        pub(crate) fn as_raw(&self) -> &alloc::vec::IntoIter<RawAttachment> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Upheld as the type parameters do not change.
            // 2. No mutation is possible through this reference
            &self.raw
        }

        /// Provides mutable access to the inner raw iterator
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `T = SendSync`: No mutation is performed that invalidates the
        ///    invariant that either all inner attachments are `Send + Sync` or
        ///    the collection is empty.
        pub(crate) unsafe fn as_raw_mut(&mut self) -> &mut alloc::vec::IntoIter<RawAttachment> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Upheld as the type parameters do not change.
            // 2. Guaranteed by the caller
            &mut self.raw
        }
    }
}
pub use limit_field_access::ReportAttachmentsIntoIter;

impl<T> Iterator for ReportAttachmentsIntoIter<T> {
    type Item = ReportAttachment<Dynamic, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: We only remove items, we don't mutate them.
        // 1. If the collection is already empty, this is a no-op and it is still empty
        //    after. On the other hand, if there are items, we are guaranteed by the
        //    invariants of this type that all inner attachments are `Send + Sync` if `T
        //    = SendSync`.
        let raw = unsafe { self.as_raw_mut() };

        let attachment = raw.next()?;

        // SAFETY:
        // 1. `A=Dynamic`, so this is trivially satisfied.
        // 2. Guaranteed by the invariants of this type.
        // 3. `A=Dynamic`, so this is trivially satisfied.
        // 4. Guaranteed by the invariants of this type.
        let attachment = unsafe { ReportAttachment::<Dynamic, T>::from_raw(attachment) };

        Some(attachment)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.as_raw().size_hint()
    }
}

impl<T> DoubleEndedIterator for ReportAttachmentsIntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY: We only remove items, we don't mutate them.
        // 1. If the collection is already empty, this is a no-op and it is still empty
        //    after. On the other hand, if there are items, we are guaranteed by the
        //    invariants of this type that all inner attachments are `Send + Sync` if `T
        //    = SendSync`.
        let raw = unsafe { self.as_raw_mut() };

        let attachment = raw.next_back()?;

        // SAFETY:
        // 1. `A=Dynamic`, so this is trivially satisfied.
        // 2. Guaranteed by the invariants of this type.
        // 3. `A=Dynamic`, so this is trivially satisfied.
        // 4. Guaranteed by the invariants of this type.
        let attachment = unsafe { ReportAttachment::<Dynamic, T>::from_raw(attachment) };

        Some(attachment)
    }
}

impl<T> ExactSizeIterator for ReportAttachmentsIntoIter<T> {
    fn len(&self) -> usize {
        self.as_raw().len()
    }
}

impl<T> FusedIterator for ReportAttachmentsIntoIter<T> {}

impl<T> Unpin for ReportAttachmentsIntoIter<T> {}
