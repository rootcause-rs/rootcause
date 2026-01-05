use alloc::fmt;
use core::any::TypeId;

use rootcause_internals::handlers::{AttachmentFormattingStyle, FormattingFunction};

use crate::{markers::Dynamic, util::format_helper};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::RawAttachmentMut;

    use crate::markers::Dynamic;

    #[repr(transparent)]
    pub struct ReportAttachmentMut<'a, Attachment: ?Sized + 'static = Dynamic> {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachmentRef`] must be of type `A`.
        raw: RawAttachmentMut<'a>,
        _attachment: PhantomData<Attachment>,
    }

    impl<'a, A: ?Sized> ReportAttachmentMut<'a, A> {
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
            ReportAttachmentMut {
                raw,
                _attachment: PhantomData,
            }
        }

        /// Returns the underlying raw attachment reference
        #[must_use]
        pub(crate) fn into_raw_ref(self) -> RawAttachmentMut<'a> {
            // We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }
    }
}

pub use limit_field_access::ReportAttachmentMut;

impl<'a, A: Sized> ReportAttachmentMut<'a, A> {
    /// Obtain the reference to the inner attachment data.
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
    pub fn into_inner(self) -> &'a mut A {
        let raw = self.into_raw_ref();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        unsafe { raw.into_attachment_downcast_unchecked() }
    }
}
