use alloc::fmt;
use core::any::TypeId;

use rootcause_internals::{
    RawAttachmentMut,
    handlers::{AttachmentFormattingStyle, FormattingFunction},
};

use crate::{
    markers::{Dynamic, SendSync},
    preformatted::PreformattedAttachment,
    report_attachment::{ReportAttachment, ReportAttachmentRef},
    util::format_helper,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::{RawAttachmentMut, RawAttachmentRef};

    use crate::markers::Dynamic;

    /// TODO see [`ReportMut`](crate::report::mut_::ReportMut)
    #[repr(transparent)]
    pub struct ReportAttachmentMut<'a, A: ?Sized + 'static = Dynamic> {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachmentMut`] must be of type `A`.
        /// 3. This reference represents exclusive mutable access to the underlying [`AttachmentData`](rootcause_internals::attachment::data::AttachmentData).
        raw: RawAttachmentMut<'a>,
        _attachment: PhantomData<&'a mut A>,
    }

    impl<'a, A: ?Sized + 'static> ReportAttachmentMut<'a, A> {
        /// Creates a new AttachmentRef from a raw attachment reference
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachmentMut`] must be of type `A`.
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawAttachmentMut<'a>) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            // 3. Guaranteed by safety invariant #3 of [`RawAttachmentMut`]
            ReportAttachmentMut {
                raw,
                _attachment: PhantomData,
            }
        }

        // Creates a raw reference to the underlying report.
        #[must_use]
        pub(crate) fn as_raw_ref<'b>(&'b self) -> RawAttachmentRef<'b> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. Upheld as the type parameter does not change.
            // 2. Upheld as the type parameter does not change.
            // 3. Upheld since `self` is borrowed as shared and no mutable access can happen through the reference.
            let raw: &RawAttachmentMut<'_> = &self.raw;

            raw.as_ref()
        }

        /// Returns the underlying raw attachment reference
        #[must_use]
        pub(crate) fn into_raw_mut(self) -> RawAttachmentMut<'a> {
            // We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }

        /// Creates a raw reference to the underlying report.
        pub(crate) fn as_raw_mut<'b>(&'b mut self) -> RawAttachmentMut<'b> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. Upheld as the type parameter does not change.
            // 2. Upheld as the type parameter does not change.
            // 3. Upheld since `self` is borrowed mutably transfers exclusive access by futher mutable borrow.
            let raw = &mut self.raw;

            raw.reborrow()
        }
    }
}

pub use limit_field_access::ReportAttachmentMut;

impl<'a, A: Sized + 'static> ReportAttachmentMut<'a, A> {
    /// Returns a reference to the attachment data.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportAttachmentMut};
    /// let mut attachment = ReportAttacment::new(41);
    /// let attachment_mut = attachment.as_mut();
    /// let data = attachment_mut.inner();
    /// println!("The answer: {}", *data + 1); // => 42
    /// ```
    #[must_use]
    pub fn inner(&self) -> &A {
        self.as_ref().inner()
    }

    /// Returns a reference to the attachment data.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportAttachmentMut};
    /// let mut attachment = ReportAttacment::new(41);
    /// let attachment_mut = attachment.as_mut();
    /// let data = attachment_mut.inner_mut();
    /// ```
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut A {
        self.as_mut().into_inner_mut()
    }

    /// Obtain the mutable reference to the inner attachment data.
    ///
    /// This method provides direct access to the attachment's data when the
    /// concrete type `A` is known at compile time. The attachment type must
    /// be [`Sized`] to use this method.
    ///
    /// # Panics
    /// This method will panic if the actual type of the attachment doesn't
    /// match the type `A`. For a safe alternative that returns [`Option`],
    /// use [`downcast_inner`] instead.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentMut},
    /// };
    ///
    /// let mut attachment: ReportAttachment<i32> = ReportAttachment::new(40i32);
    /// {
    ///   let attachment_mut: ReportAttachmentMut<'_, i32> = attachment.as_mut();
    ///   let number: &mut i32 = attachment_ref.into_inner();
    ///   *number += 2;
    /// }
    ///
    /// assert_eq!(attachment.as_ref().inner(), &42i32);
    /// ```
    ///
    /// [`downcast_inner`]: Self::downcast_inner
    #[must_use]
    pub fn into_inner_mut(self) -> &'a mut A {
        let raw = self.into_raw_mut();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        unsafe { raw.into_attachment_downcast_unchecked() }
    }
}

impl<'a, A: ?Sized> ReportAttachmentMut<'a, A> {
    /// Changes the context type of the [`ReportAttachmentMut`] to [`Dynamic`].
    ///
    /// TODO
    #[must_use]
    pub fn into_dynamic(self) -> ReportAttachmentMut<'a, Dynamic> {
        let raw = self.into_raw_mut();

        // SAFETY:
        // 1. Trivially true.
        // 2. Not `Sized`.
        unsafe {
            // @add-unsafe-context: Dynamic
            ReportAttachmentMut::<Dynamic>::from_raw(raw)
        }
    }

    /// TODO
    #[must_use]
    pub fn as_ref(&self) -> ReportAttachmentRef<'_, A> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by invariants of this type.
        // 2. Guaranteed by invariants of this type.
        unsafe { ReportAttachmentRef::<A>::from_raw(raw) }
    }

    /// TODO
    #[must_use]
    pub fn into_ref(self) -> ReportAttachmentRef<'a, A> {
        let raw = self.into_raw_mut();

        let raw = raw.into_ref();

        // SAFETY:
        // 1. Guaranteed by invariants of this type.
        // 2. Guaranteed by invariants of this type.
        unsafe { ReportAttachmentRef::<A>::from_raw(raw) }
    }

    /// Reborrows the [`ReportAttachmentMut`] to return a new [`ReportAttachmentMut`] with a shorter
    /// lifetime
    ///
    /// # Examples
    ///
    /// TODO, see [`ReportMut::as_mut`](crate::report::mut_::ReportMut::as_mut)
    #[must_use]
    pub fn as_mut(&mut self) -> ReportAttachmentMut<'_, A> {
        let raw = self.as_raw_mut();

        // SAFETY:
        // 1. Guaranteed by invariants of this type.
        // 2. Guaranteed by invariants of this type.
        unsafe { ReportAttachmentMut::from_raw(raw) }
    }

    /// TODO see [`crate::report::mut_::ReportMut::preformat`]
    #[track_caller]
    #[must_use]
    pub fn preformat(&self) -> ReportAttachment<PreformattedAttachment, SendSync> {
        self.as_ref().preformat()
    }
}
