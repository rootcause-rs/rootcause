use alloc::fmt;
use core::any::TypeId;

use rootcause_internals::{
    RawAttachmentMut,
    handlers::{AttachmentFormattingStyle, FormattingFunction},
};

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
        ///    [`RawAttachmentMut`] must be of type `A`.
        raw: RawAttachmentMut<'a>,
        _attachment: PhantomData<&'a mut Attachment>,
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
            ReportAttachmentMut {
                raw,
                _attachment: PhantomData,
            }
        }

        /// Returns the underlying raw attachment reference
        #[must_use]
        pub(crate) fn into_raw_mut(self) -> RawAttachmentMut<'a> {
            // We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }
    }
}

pub use limit_field_access::ReportAttachmentMut;

impl<'a, A: Sized + 'static> ReportAttachmentMut<'a, A> {
    /// Returns a mutable reference to the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # let mut report: Report<String> = report!("An error occurred".to_string());
    /// let mut report_mut: ReportMut<'_, String> = report.as_mut();
    /// let context: &mut String = report_mut.current_context_mut();
    /// context.push_str(" and that's bad");
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
    /// Reborrows the [`ReportAttachmentMut`] to return a new [`ReportAttachmentMut`] with a shorter
    /// lifetime
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let mut report_mut: ReportMut<'_, MyError> = report.as_mut();
    /// {
    ///     // Create a new mutable reference with a shorter lifetime
    ///     let mut borrowed_report_mut: ReportMut<'_, MyError> = report_mut.as_mut();
    /// }
    /// // After dropping the inner reference report, we can still use the outer one
    /// let _context: &MyError = report_mut.current_context();
    /// ```
    #[must_use]
    pub fn as_mut(&mut self) -> ReportMut<'_, C, T> {
        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T=SendSync`, then we are
        //    not allowed to mutate the returned raw report in a way that adds
        //    non-`Send+Sync` objects. We do not mutate the report here and the
        //    invariants of the created `ReportMut` guarantee that no such mutation can
        //    occur in the future either.
        let raw = unsafe { self.as_raw_mut() };

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. This is guaranteed by the invariants of this type.
        // 3. If `C` is a `Sized` type: This is guaranteed by the invariants of this
        //    type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        // 6. If `T = SendSync`: This is guaranteed by the invariants of this type.
        // 7. If `T = Local`: This is guaranteed by the invariants of this type.
        unsafe { ReportMut::from_raw(raw) }
    }

    /// Changes the context type of the [`ReportAttachmentMut`] to [`Dynamic`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the context mode to
    /// [`Dynamic`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that the context actually has the
    /// type `C`.
    ///
    /// To get back the report with a concrete `C` you can use the method
    /// [`ReportMut::downcast_report`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, markers::Dynamic};
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let report: ReportMut<'_, MyError> = report.as_mut();
    /// let local_report: ReportMut<'_, Dynamic> = report.into_dynamic();
    /// ```
    #[must_use]
    pub fn into_dynamic(self) -> ReportAttachmentMut<'a, Dynamic> {
        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T=SendSync`, then we are
        //    not allowed to mutate the returned raw attachmenht in a way that adds
        //    non-`Send+Sync` objects. We do not mutate the attachment here and the
        //    invariants of the created `ReportAttachmentMut` guarantee that no such mutation can
        //    occur in the future either.
        let raw = unsafe { self.into_raw_mut() };

        // SAFETY:
        // 1. `C=Dynamic`, so this is trivially true.
        // 2. This is guaranteed by the invariants of this type.
        // 3. `C=Dynamic`, so this is trivially true.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        // 6. This is guaranteed by the invariants of this type.
        // 7. This is guaranteed by the invariants of this type.
        unsafe {
            // @add-unsafe-context: Dynamic
            ReportAttachmentMut::<Dynamic>::from_raw(raw)
        }
    }
}
