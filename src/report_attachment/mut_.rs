use core::any::TypeId;

use rootcause_internals::handlers::{AttachmentFormattingStyle, FormattingFunction};

use crate::{
    markers::{Dynamic, SendSync},
    preformatted::PreformattedAttachment,
    report_attachment::{ReportAttachment, ReportAttachmentRef},
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::{RawAttachmentMut, RawAttachmentRef};

    use crate::markers::Dynamic;

    /// A mutable reference to a [`ReportAttachment`], enabling mutation of the
    /// underlying attachment.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_attachment::ReportAttachment};
    /// let mut attachment = ReportAttachment::new_sendsync(41);
    /// {
    ///     let mut attachment_mut = attachment.as_mut();
    ///     *attachment_mut.inner_mut() += 1;
    /// }
    /// println!("The answer: {}", attachment.format_inner());
    /// ```
    ///
    /// [`ReportAttachment`]: crate::report_attachment::ReportAttachment
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
        _attachment: PhantomData<Attachment>,
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

        // Creates a raw reference to the underlying report.
        //
        // This returns a raw reference to the underlying report.
        #[must_use]
        pub(crate) fn as_raw_ref<'b>(&'b self) -> RawAttachmentRef<'b> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. Upheld as the type parameter does not change.
            // 2. Upheld as the type parameter does not change.
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
            let raw = &mut self.raw;

            raw.reborrow()
        }
    }
}

pub use limit_field_access::ReportAttachmentMut;

impl<'a, A: Sized> ReportAttachmentMut<'a, A> {
    /// Returns a reference to the attachment data.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_attachment::*};
    /// let mut attachment = ReportAttachment::new_sendsync(41i32);
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
    /// # use rootcause::{prelude::*, report_attachment::*};
    /// let mut attachment = ReportAttachment::new_sendsync(41i32);
    /// let mut attachment_mut = attachment.as_mut();
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
    ///     let attachment_mut: ReportAttachmentMut<'_, i32> = attachment.as_mut();
    ///     let number: &mut i32 = attachment_mut.into_inner_mut();
    ///     *number += 2;
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
    /// Changes the attachment type of the [`ReportAttachmentMut`] to
    /// [`Dynamic`].
    ///
    /// Calling this method is equivalent to calling `attachment.into()`,
    /// however this method has been restricted to only change the
    /// attachment to [`Dynamic`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the attachment in any way. It only
    /// has the effect of "forgetting" that the context actually has the
    /// type `A`.
    ///
    /// To get back the report with a concrete `A` you can use the method
    /// [`ReportAttachmentMut::downcast_attachment`].
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

    /// Returns an immutable reference to the attachment.
    #[must_use]
    pub fn as_ref(&self) -> ReportAttachmentRef<'_, A> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by invariants of this type.
        // 2. Guaranteed by invariants of this type.
        unsafe {
            // @add-unsafe-context: RawAttachmentRef
            ReportAttachmentRef::<A>::from_raw(raw)
        }
    }

    /// Consumes the [`ReportAttachmentMut`] and returns a
    /// [`ReportAttachmentRef`] with same lifetime.
    #[must_use]
    pub fn into_ref(self) -> ReportAttachmentRef<'a, A> {
        let raw = self.into_raw_mut();

        let raw = raw.into_ref();

        // SAFETY:
        // 1. Guaranteed by invariants of this type.
        // 2. Guaranteed by invariants of this type.
        unsafe { ReportAttachmentRef::<A>::from_raw(raw) }
    }

    /// Reborrows the [`ReportAttachmentMut`] to return a new
    /// [`ReportAttachmentMut`] with a shorter lifetime.
    #[must_use]
    pub fn as_mut(&mut self) -> ReportAttachmentMut<'_, A> {
        let raw = self.as_raw_mut();

        // SAFETY:
        // 1. Guaranteed by invariants of this type.
        // 2. Guaranteed by invariants of this type.
        unsafe {
            // @add-unsafe-context: RawAttachmentMut
            ReportAttachmentMut::from_raw(raw)
        }
    }

    /// Returns the [`TypeId`] of the inner attachment.
    ///
    /// # Examples
    /// ```
    /// use std::any::TypeId;
    ///
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = attachment.as_ref();
    /// assert_eq!(attachment_ref.inner_type_id(), TypeId::of::<&str>());
    /// ```
    #[must_use]
    pub fn inner_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_type_id()
    }

    /// Returns the [`TypeId`] of the inner attachment.
    ///
    /// # Examples
    /// ```
    /// use std::any::TypeId;
    ///
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = attachment.as_ref();
    /// assert_eq!(
    ///     attachment_ref.inner_type_name(),
    ///     core::any::type_name::<&str>()
    /// );
    /// ```
    #[must_use]
    pub fn inner_type_name(&self) -> &'static str {
        self.as_raw_ref().attachment_type_name()
    }

    /// Returns the [`TypeId`] of the handler used when creating this
    /// attachment.
    ///
    /// Each attachment is associated with a specific handler (like
    /// [`handlers::Display`] or [`handlers::Debug`]) that determines how it
    /// should be formatted when included in a report. This method allows
    /// you to inspect which handler is being used.
    ///
    /// [`handlers::Display`]: crate::handlers::Display
    /// [`handlers::Debug`]: crate::handlers::Debug
    #[must_use]
    pub fn inner_handler_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_handler_type_id()
    }

    /// Formats the inner attachment data with formatting hooks applied.
    ///
    /// This method formats the attachment using both its handler and any global
    /// formatting hooks that have been registered. The hooks allow for
    /// custom formatting behaviors such as filtering, transforming, or
    /// decorating the output. The returned object implements both
    /// [`Display`] and [`Debug`] traits.
    ///
    /// For direct formatting without hooks, use [`format_inner_unhooked`]
    /// instead.
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`format_inner_unhooked`]: Self::format_inner_unhooked
    #[must_use]
    pub fn format_inner(&self) -> impl core::fmt::Display + core::fmt::Debug {
        self.as_ref().format_inner()
    }

    /// Formats the inner attachment data without applying any formatting hooks.
    ///
    /// This method provides direct access to the attachment's formatting
    /// capabilities as defined by its handler, bypassing any global
    /// formatting hooks that might modify the output. The returned object
    /// implements both [`Display`] and [`Debug`] traits for flexible
    /// formatting options.
    ///
    /// For formatted output that includes formatting hooks, use
    /// [`format_inner`] instead.
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`format_inner`]: Self::format_inner
    #[must_use]
    pub fn format_inner_unhooked(&self) -> impl core::fmt::Display + core::fmt::Debug {
        self.as_ref().format_inner_unhooked()
    }

    /// Returns the preferred formatting style for this attachment with
    /// formatting hooks applied.
    ///
    /// This method determines how the attachment should be formatted when
    /// included in a report, taking into account both the attachment's
    /// handler preferences and any global formatting hooks that might
    /// modify the behavior.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this
    ///   attachment will be embedded is being formatted using [`Display`]
    ///   formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    #[must_use]
    pub fn preferred_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.as_ref()
            .preferred_formatting_style(report_formatting_function)
    }

    /// Returns the preferred formatting style for this attachment without
    /// formatting hooks.
    ///
    /// This method determines how the attachment should be formatted based
    /// solely on its handler's preferences, bypassing any global formatting
    /// hooks that might modify the behavior. For formatting that includes
    /// hooks, use [`preferred_formatting_style`] instead.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this
    ///   attachment will be embedded is being formatted using [`Display`]
    ///   formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`preferred_formatting_style`]: Self::preferred_formatting_style
    #[must_use]
    pub fn preferred_formatting_style_unhooked(
        self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.as_raw_ref()
            .preferred_formatting_style(report_formatting_function)
    }

    /// See [`crate::report_attachment::owned::ReportAttachment::preformat`]
    #[track_caller]
    #[must_use]
    pub fn preformat(&self) -> ReportAttachment<PreformattedAttachment, SendSync> {
        self.as_ref().preformat()
    }
}

impl<'a> ReportAttachmentMut<'a, Dynamic> {
    /// Attempts to downcast the attachment reference to a different type `A`.
    ///
    /// This method performs a safe type cast, returning [`Ok`] if the
    /// attachment actually contains data of type `A`, or [`Err`] with the
    /// original reference if the types don't match.
    ///
    /// This method is most useful when going from a [`Dynamic`] to a concrete
    /// `A`.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentMut},
    /// };
    ///
    /// let mut attachment: ReportAttachment<Dynamic> = ReportAttachment::new(41i32).into_dynamic();
    /// let attachment_mut: ReportAttachmentMut<'_, Dynamic> = attachment.as_mut();
    ///
    /// // Try to downcast to an incorrect type
    /// let wrong_ref: Result<ReportAttachmentMut<'_, String>, _> =
    ///     attachment_mut.downcast_attachment();
    /// assert!(wrong_ref.is_err());
    ///
    /// let attachment_mut = wrong_ref.unwrap_err();
    ///
    /// // Try to downcast to the correct type
    /// let typed_ref: Result<ReportAttachmentMut<'_, i32>, _> = attachment_mut.downcast_attachment();
    /// assert!(typed_ref.is_ok());
    /// ```
    pub fn downcast_attachment<A>(self) -> Result<ReportAttachmentMut<'a, A>, Self>
    where
        A: Sized + 'static,
    {
        if TypeId::of::<A>() == self.inner_type_id() {
            // SAFETY:
            // 1. We just checked that the types match
            let attachment = unsafe { self.downcast_attachment_unchecked() };
            Ok(attachment)
        } else {
            Err(self)
        }
    }

    /// Performs an unchecked downcast of the attachment reference to type `A`.
    ///
    /// This method bypasses type checking and performs the cast without
    /// verifying that the attachment actually contains data of type `A`. It
    /// is the caller's responsibility to ensure the cast is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The inner attachment is actually of type `A`. This can be verified by
    ///    calling [`inner_type_id()`] first.
    ///
    /// [`inner_type_id()`]: ReportAttachmentMut::inner_type_id
    ///
    /// # Examples
    /// ```
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let mut attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_mut: ReportAttachmentMut<'_, Dynamic> = attachment.as_mut();
    ///
    /// // SAFETY: We know the attachment contains &str data
    /// let typed_mut: ReportAttachmentMut<'_, &str> =
    ///     unsafe { attachment_mut.downcast_attachment_unchecked() };
    /// ```
    #[must_use]
    pub unsafe fn downcast_attachment_unchecked<A>(self) -> ReportAttachmentMut<'a, A>
    where
        A: Sized + 'static,
    {
        let raw = self.into_raw_mut();

        // SAFETY:
        // 1. `A` is bounded by `Sized` in the function signature, so this is satisfied.
        // 2. Guaranteed by the caller
        unsafe { ReportAttachmentMut::from_raw(raw) }
    }

    /// Attempts to downcast the inner attachment data to a reference of type
    /// `A`.
    ///
    /// See [`ReportAttachmentRef::downcast_inner`] for more info.
    pub fn downcast_inner<A>(&self) -> Option<&A>
    where
        A: Sized + 'static,
    {
        self.as_ref().downcast_inner()
    }

    /// Performs an unchecked downcast of the inner attachment data to a
    /// reference of type `A`.
    ///
    /// See [`ReportAttachmentRef::downcast_inner_unchecked`]  for more info.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The inner attachment is actually of type `A`. This can be verified by
    ///    calling [`inner_type_id()`] first.
    ///
    /// [`inner_type_id()`]: ReportAttachmentRef::inner_type_id
    #[must_use]
    pub unsafe fn downcast_inner_unchecked<A>(&self) -> &A
    where
        A: Sized + 'static,
    {
        let ref_ = self.as_ref();
        // SAFETY:
        // 1. Guaranteed by the caller
        unsafe { ref_.downcast_inner_unchecked() }
    }

    /// Attempts to downcast the inner attachment data to a reference of type
    /// `A`.
    ///
    /// This method performs a safe type cast, returning [`Some`] with a
    /// reference to the data if the attachment actually contains data of
    /// type `A`, or [`None`] if the types don't match. Unlike
    /// [`downcast_attachment`], this method returns a direct reference to
    /// the data rather than a [`ReportAttachmentRef`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{
    /// #     markers::Dynamic,
    /// #     prelude::*,
    /// #     report_attachment::{ReportAttachment, ReportAttachmentMut},
    /// # };
    ///
    /// let mut attachment: ReportAttachment<Dynamic> =
    ///     ReportAttachment::new_sendsync(42).into_dynamic();
    /// let mut attachment_mut: ReportAttachmentMut<'_, Dynamic> = attachment.as_mut();
    ///
    /// // Try to downcast to the correct type
    /// let data: Option<&mut i32> = attachment_mut.downcast_inner_mut();
    /// assert_eq!(data, Some(&mut 42));
    ///
    /// // Try to downcast to an incorrect type
    /// let wrong_data: Option<&mut String> = attachment_mut.downcast_inner_mut();
    /// assert!(wrong_data.is_none());
    /// ```
    ///
    /// [`downcast_attachment`]: Self::downcast_attachment
    #[must_use]
    pub fn downcast_inner_mut<A>(&mut self) -> Option<&mut A>
    where
        A: Sized + 'static,
    {
        if TypeId::of::<A>() == self.inner_type_id() {
            // SAFETY:
            // 1. We just checked that the types match
            let attachment = unsafe { self.downcast_inner_mut_unchecked() };
            Some(attachment)
        } else {
            None
        }
    }

    /// Performs an unchecked downcast of the inner attachment data to a
    /// mutable reference of type `A`.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The inner attachment is actually of type `A`. This can be verified by
    ///    calling [`inner_type_id()`] first.
    ///
    /// [`inner_type_id()`]: ReportAttachmentRef::inner_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{
    /// #    markers::Dynamic,
    /// #    prelude::*,
    /// #    report_attachment::*,
    /// # };
    ///
    /// let mut attachment: ReportAttachment<Dynamic> = ReportAttachment::new(41i32).into_dynamic();
    /// let mut attachment_mut: ReportAttachmentMut<'_, Dynamic> = attachment.as_mut();
    ///
    /// // SAFETY: We know the attachment contains i32 data
    /// let data: &mut i32 = unsafe { attachment_mut.downcast_inner_mut_unchecked() };
    /// *data += 1;
    /// assert_eq!(*data, 42);
    /// ```
    #[must_use]
    pub unsafe fn downcast_inner_mut_unchecked<A>(&mut self) -> &mut A
    where
        A: Sized + 'static,
    {
        let raw = self.as_raw_mut();

        // SAFETY:
        // 1. Ensured by the caller.
        unsafe { raw.into_attachment_downcast_unchecked() }
    }
}

impl<'a, A: ?Sized> core::fmt::Display for ReportAttachmentMut<'a, A> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.as_ref(), formatter)
    }
}

impl<'a, A: ?Sized> core::fmt::Debug for ReportAttachmentMut<'a, A> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), formatter)
    }
}

impl<'a, A: ?Sized> Unpin for ReportAttachmentMut<'a, A> {}

impl<'a, A: Sized> From<ReportAttachmentMut<'a, A>> for ReportAttachmentMut<'a, Dynamic> {
    fn from(value: ReportAttachmentMut<'a, A>) -> Self {
        value.into_dynamic()
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[allow(dead_code)]
    struct NonSend(*const ());
    static_assertions::assert_not_impl_any!(NonSend: Send, Sync);

    #[test]
    fn report_attachment_mut_is_never_send_sync() {
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, ()>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, String>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, NonSend>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, Dynamic>: Send, Sync);
    }

    #[test]
    fn report_attachment_mut_is_always_unpin() {
        static_assertions::assert_impl_all!(ReportAttachmentMut<'static, ()>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachmentMut<'static, String>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachmentMut<'static, NonSend>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachmentMut<'static, Dynamic>: Unpin);
    }

    #[test]
    fn test_report_mut_is_never_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, ()>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, String>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, NonSend>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachmentMut<'static, Dynamic>: Copy, Clone);
    }
}
