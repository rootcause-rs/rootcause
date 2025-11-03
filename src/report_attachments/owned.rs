use alloc::vec::Vec;
use core::{any::Any, marker::PhantomData};

use rootcause_internals::RawAttachment;

use crate::{
    markers::{self, Local, SendSync},
    report_attachment::{ReportAttachment, ReportAttachmentRef},
    report_attachments::{ReportAttachmentsIntoIter, ReportAttachmentsIter},
};

/// A collection of report attachments.
///
/// This type provides storage and management for multiple attachments that can
/// be added to a report.
///
/// You can think of a [`ReportAttachments<T>`] as a wrapper around a
/// `Vec<ReportAttachment<dyn Any, T>>`, however, it has a slightly
/// different API:
/// - It has convenience methods to convert between different thread safety
///   markers such as [`into_local`](Self::into_local).
/// - It is also possible to convert between different context and thread safety
///   markers using the [`From`] and [`Into`] traits.
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
    /// Creates a new [`ReportAttachments`] from a vector of raw attachments
    ///
    /// # Safety
    ///
    /// The thread safety marker must match the contents of the attachments.
    /// More specifically if the marker is `SendSync`, then all the inner
    /// attachments must be `Send+Sync`
    #[must_use]
    pub(crate) unsafe fn from_raw(raw: Vec<RawAttachment>) -> Self {
        Self {
            raw,
            _thread_safety: PhantomData,
        }
    }

    /// Creates a reference to [`ReportAttachments`] from reference to a vector
    /// of raw attachments
    ///
    /// # Safety
    ///
    /// The thread safety marker must match the contents of the attachments.
    /// More specifically if the marker is `SendSync`, then all the inner
    /// attachments must be `Send+Sync`
    #[must_use]
    pub(crate) unsafe fn from_raw_ref(raw: &Vec<RawAttachment>) -> &Self {
        unsafe { &*(raw as *const Vec<RawAttachment> as *const Self) }
    }

    /// Creates a mutable reference to [`ReportAttachments`] from a mutable
    /// vector of raw attachments
    ///
    /// # Safety
    ///
    /// The thread safety marker must match the contents of the attachments.
    /// More specifically if the marker is `SendSync`, then all the inner
    /// attachments must be `Send+Sync`
    #[must_use]
    pub(crate) unsafe fn from_raw_mut(raw: &mut Vec<RawAttachment>) -> &mut Self {
        unsafe { &mut *(raw as *mut Vec<RawAttachment> as *mut Self) }
    }

    #[must_use]
    pub(crate) fn into_raw(self) -> Vec<RawAttachment> {
        self.raw
    }

    /// Creates a new, empty attachments collection.
    ///
    /// The collection will not allocate until attachments are added to it.
    /// This method is generic over the thread safety marker, but for better
    /// type inference, consider using [`new_sendsync()`] or [`new_local()`]
    /// instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{markers::SendSync, report_attachments::ReportAttachments};
    ///
    /// let attachments: ReportAttachments<SendSync> = ReportAttachments::new();
    /// assert!(attachments.is_empty());
    /// assert_eq!(attachments.len(), 0);
    /// ```
    ///
    /// [`new_sendsync()`]: ReportAttachments<SendSync>::new_sendsync
    /// [`new_local()`]: ReportAttachments<Local>::new_local
    #[must_use]
    pub fn new() -> Self {
        unsafe { Self::from_raw(Vec::new()) }
    }

    /// Appends an attachment to the end of the collection.
    ///
    /// This method takes ownership of the attachment and adds it to the
    /// collection. The attachment must be type-erased to `dyn Any` to be
    /// stored in the collection alongside other attachments of potentially
    /// different types.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_attachment::ReportAttachment, report_attachments::ReportAttachments};
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// let attachment = ReportAttachment::new("debug info").into_dyn_any();
    ///
    /// attachments.push(attachment);
    /// assert_eq!(attachments.len(), 1);
    /// ```
    pub fn push(&mut self, attachment: ReportAttachment<dyn Any, T>) {
        self.raw.push(attachment.into_raw())
    }

    /// Removes and returns the last attachment from the collection.
    ///
    /// Returns [`None`] if the collection is empty.
    ///
    /// This method provides LIFO (last in, first out) behavior, making the
    /// collection behave like a stack for the most recently added attachments.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_attachment::ReportAttachment, report_attachments::ReportAttachments};
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// attachments.push(ReportAttachment::new("first").into_dyn_any());
    /// attachments.push(ReportAttachment::new("second").into_dyn_any());
    ///
    /// assert_eq!(attachments.len(), 2);
    /// let last = attachments.pop().unwrap();
    /// assert_eq!(attachments.len(), 1);
    ///
    /// // Verify it was the last one added
    /// assert_eq!(last.inner_type_id(), std::any::TypeId::of::<&str>());
    /// ```
    pub fn pop(&mut self) -> Option<ReportAttachment<dyn Any, T>> {
        let attachment = self.raw.pop()?;

        // SAFETY: The thread safety marker matches, because we only
        // contain attachments with a matching thread safety marker
        let attachment = unsafe { ReportAttachment::from_raw(attachment) };

        Some(attachment)
    }

    /// Returns the number of attachments in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_attachment::ReportAttachment, report_attachments::ReportAttachments};
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// assert_eq!(attachments.len(), 0);
    ///
    /// attachments.push(ReportAttachment::new("info").into_dyn_any());
    /// attachments.push(ReportAttachment::new(42).into_dyn_any());
    /// assert_eq!(attachments.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Returns a reference to the attachment at the given index.
    ///
    /// Returns [`None`] if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_attachment::ReportAttachment, report_attachments::ReportAttachments};
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// attachments.push(ReportAttachment::new("first").into_dyn_any());
    /// attachments.push(ReportAttachment::new("second").into_dyn_any());
    ///
    /// let first = attachments.get(0).unwrap();
    /// assert_eq!(first.inner_type_id(), std::any::TypeId::of::<&str>());
    ///
    /// assert!(attachments.get(10).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, index: usize) -> Option<ReportAttachmentRef<'_, dyn Any>> {
        let attachment = self.raw.get(index)?.as_ref();
        unsafe { Some(ReportAttachmentRef::from_raw(attachment)) }
    }

    /// Returns `true` if the collection contains no attachments.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_attachment::ReportAttachment, report_attachments::ReportAttachments};
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// assert!(attachments.is_empty());
    ///
    /// attachments.push(ReportAttachment::new("info").into_dyn_any());
    /// assert!(!attachments.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Returns an iterator over references to the attachments in the
    /// collection.
    ///
    /// The iterator yields [`ReportAttachmentRef`] items, which provide
    /// non-owning access to the attachments. For owning iteration, use
    /// [`into_iter()`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_attachment::ReportAttachment, report_attachments::ReportAttachments};
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// attachments.push(ReportAttachment::new("first").into_dyn_any());
    /// attachments.push(ReportAttachment::new("second").into_dyn_any());
    ///
    /// for attachment in attachments.iter() {
    ///     println!("Attachment type: {:?}", attachment.inner_type_id());
    /// }
    /// ```
    ///
    /// [`into_iter()`]: Self::into_iter
    pub fn iter(&self) -> ReportAttachmentsIter<'_> {
        ReportAttachmentsIter::from_raw(self.raw.iter())
    }

    /// Converts this collection to use the [`Local`] thread safety marker.
    ///
    /// This conversion consumes the collection and returns a new one with
    /// the [`Local`] marker, which allows the collection to contain attachments
    /// that are not `Send + Sync`. This is always safe since local thread
    /// safety is less restrictive than send/sync thread safety.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     markers::{Local, SendSync},
    ///     report_attachment::ReportAttachment,
    ///     report_attachments::ReportAttachments,
    /// };
    ///
    /// let mut attachments: ReportAttachments<SendSync> = ReportAttachments::new_sendsync();
    /// attachments.push(ReportAttachment::new("info").into_dyn_any());
    ///
    /// let local_attachments: ReportAttachments<Local> = attachments.into_local();
    /// assert_eq!(local_attachments.len(), 1);
    /// ```
    #[must_use]
    pub fn into_local(self) -> ReportAttachments<Local> {
        unsafe { ReportAttachments::from_raw(self.into_raw()) }
    }

    /// Returns a reference to this collection with the [`Local`] thread safety
    /// marker.
    ///
    /// This method provides a zero-cost view of the collection with local
    /// thread safety semantics, without consuming the original collection.
    /// This is always safe since local thread safety is less restrictive
    /// than send/sync thread safety.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     markers::{Local, SendSync},
    ///     report_attachment::ReportAttachment,
    ///     report_attachments::ReportAttachments,
    /// };
    ///
    /// let mut attachments: ReportAttachments<SendSync> = ReportAttachments::new_sendsync();
    /// attachments.push(ReportAttachment::new("info").into_dyn_any());
    ///
    /// let local_view: &ReportAttachments<Local> = attachments.as_local();
    /// assert_eq!(local_view.len(), 1);
    /// assert_eq!(attachments.len(), 1); // Original is still usable
    /// ```
    #[must_use]
    pub fn as_local(&self) -> &ReportAttachments<Local> {
        unsafe { ReportAttachments::from_raw_ref(&self.raw) }
    }
}

impl ReportAttachments<SendSync> {
    /// Creates a new, empty attachments collection with [`SendSync`] thread
    /// safety.
    ///
    /// Attachments in this collection must be `Send + Sync`, making the
    /// collection itself safe to share across threads. This is the most
    /// common thread safety mode and is used by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     markers::SendSync, report_attachment::ReportAttachment,
    ///     report_attachments::ReportAttachments,
    /// };
    ///
    /// let mut attachments = ReportAttachments::new_sendsync();
    /// attachments.push(ReportAttachment::new("thread-safe attachment").into_dyn_any());
    /// assert_eq!(attachments.len(), 1);
    /// ```
    #[must_use]
    pub fn new_sendsync() -> Self {
        Self::new()
    }
}

impl ReportAttachments<Local> {
    /// Creates a new, empty attachments collection with [`Local`] thread
    /// safety.
    ///
    /// Attachments in this collection can be any type and are not required to
    /// be `Send + Sync`. This collection itself cannot be shared across
    /// threads, but is useful when you need to store non-thread-safe
    /// attachments.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use rootcause::{
    ///     markers::Local, report_attachment::ReportAttachment, report_attachments::ReportAttachments,
    /// };
    ///
    /// let mut attachments = ReportAttachments::new_local();
    /// // Rc is not Send+Sync, but can be stored in a Local collection
    /// let rc_attachment = ReportAttachment::new(Rc::new("local-only")).into_dyn_any();
    /// attachments.push(rc_attachment);
    /// assert_eq!(attachments.len(), 1);
    /// ```
    #[must_use]
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
        attachments.into_local()
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

impl<'a, T> IntoIterator for &'a ReportAttachments<T>
where
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportAttachmentsIter<'a>;
    type Item = ReportAttachmentRef<'a, dyn Any>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
