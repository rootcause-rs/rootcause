use core::any::TypeId;

use rootcause_internals::handlers::{AttachmentFormattingStyle, FormattingFunction};

use crate::{
    markers::{Dynamic, SendSync},
    preformatted::{self, PreformattedAttachment},
    report_attachment::ReportAttachment,
    util::format_helper,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::RawAttachmentRef;

    use crate::markers::Dynamic;

    /// A reference to a [`ReportAttachment`].
    ///
    /// # Examples
    /// ```
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("An important attachment");
    /// let attachment_ref: ReportAttachmentRef<'_, &str> = attachment.as_ref();
    ///
    /// let mut report = report!("An error occurred");
    /// report.attachments_mut().push(attachment.into_dynamic());
    ///
    /// // You can also get an attachment reference through the attachments on a report
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = report.attachments().get(0).unwrap();
    /// ```
    ///
    /// [`ReportAttachment`]: crate::report_attachment::ReportAttachment
    #[repr(transparent)]
    pub struct ReportAttachmentRef<'a, Attachment: ?Sized + 'static = Dynamic> {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachmentRef`] must be of type `A`.
        raw: RawAttachmentRef<'a>,
        _attachment: PhantomData<Attachment>,
    }

    impl<'a, A: ?Sized> ReportAttachmentRef<'a, A> {
        /// Creates a new AttachmentRef from a raw attachment reference
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachmentRef`] must be of type `A`.
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawAttachmentRef<'a>) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            ReportAttachmentRef {
                raw,
                _attachment: PhantomData,
            }
        }

        /// Returns the underlying raw attachment reference
        #[must_use]
        pub(crate) fn as_raw_ref(self) -> RawAttachmentRef<'a> {
            // We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }
    }

    // SAFETY: We must uphold the safety invariants of the raw field for both the
    // original and the copy:
    // 1. This remains true for both the original and the copy
    // 2. This remains true for both the original and the copy
    impl<'a, A: ?Sized> Copy for ReportAttachmentRef<'a, A> {}
}

pub use limit_field_access::ReportAttachmentRef;

impl<'a, A: ?Sized> Clone for ReportAttachmentRef<'a, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, A: Sized> ReportAttachmentRef<'a, A> {
    /// Returns a reference to the inner attachment data.
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
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment_ref: ReportAttachmentRef<'_, &str> = attachment.as_ref();
    ///
    /// let text: &&str = attachment_ref.inner();
    /// assert_eq!(*text, "text data");
    /// ```
    ///
    /// [`downcast_inner`]: Self::downcast_inner
    #[must_use]
    pub fn inner(self) -> &'a A {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        unsafe { raw.attachment_downcast_unchecked() }
    }
}

impl<'a, A: ?Sized> ReportAttachmentRef<'a, A> {
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
    pub fn inner_type_id(self) -> TypeId {
        self.as_raw_ref().attachment_type_id()
    }

    /// Returns the [`core::any::type_name`] of the inner attachment.
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
    pub fn inner_type_name(self) -> &'static str {
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
    pub fn inner_handler_type_id(self) -> TypeId {
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
    pub fn format_inner(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.into_dynamic(),
            |attachment, formatter| {
                crate::hooks::attachment_formatter::display_attachment(attachment, None, formatter)
            },
            |attachment, formatter| {
                crate::hooks::attachment_formatter::debug_attachment(attachment, None, formatter)
            },
        )
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
    pub fn format_inner_unhooked(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.as_raw_ref(),
            |attachment, formatter| attachment.attachment_display(formatter),
            |attachment, formatter| attachment.attachment_debug(formatter),
        )
    }

    /// Changes the inner attachment type of the [`ReportAttachmentRef`] to
    /// [`Dynamic`].
    ///
    /// Calling this method is equivalent to calling `attachment.into()`,
    /// however this method has been restricted to only change the
    /// attachment type to [`Dynamic`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the attachment in any way. It only
    /// has the effect of "forgetting" that the inner attachment
    /// actually has the type `A`.
    ///
    /// To get back the attachment with a concrete `A` you can use the method
    /// [`ReportAttachmentRef::downcast_attachment`].
    #[must_use]
    pub fn into_dynamic(self) -> ReportAttachmentRef<'a, Dynamic> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. `A=Dynamic`, so this is trivially satisfied
        // 2. `A=Dynamic`, so this is trivially satisfied
        unsafe {
            // @add-unsafe-context: Dynamic
            ReportAttachmentRef::<Dynamic>::from_raw(raw)
        }
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
        self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        crate::hooks::attachment_formatter::get_preferred_formatting_style(
            self.into_dynamic(),
            report_formatting_function,
        )
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
    pub fn preformat(self) -> ReportAttachment<PreformattedAttachment, SendSync> {
        ReportAttachment::new_custom::<preformatted::PreformattedHandler>(
            PreformattedAttachment::new_from_attachment(self),
        )
    }
}

impl<'a> ReportAttachmentRef<'a, Dynamic> {
    /// Attempts to downcast the attachment reference to a different type `A`.
    ///
    /// This method performs a safe type cast, returning [`Some`] if the
    /// attachment actually contains data of type `A`, or [`None`] if the
    /// types don't match.
    ///
    /// This method is most useful when going from a [`Dynamic`] to a concrete
    /// `A`.
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
    /// let attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = attachment.as_ref();
    ///
    /// // Try to downcast to the correct type
    /// let typed_ref: Option<ReportAttachmentRef<'_, &str>> = attachment_ref.downcast_attachment();
    /// assert!(typed_ref.is_some());
    ///
    /// // Try to downcast to an incorrect type
    /// let wrong_ref: Option<ReportAttachmentRef<'_, i32>> = attachment_ref.downcast_attachment();
    /// assert!(wrong_ref.is_none());
    /// ```
    #[must_use]
    pub fn downcast_attachment<A>(self) -> Option<ReportAttachmentRef<'a, A>>
    where
        A: Sized + 'static,
    {
        if TypeId::of::<A>() == self.inner_type_id() {
            // SAFETY:
            // 1. We just checked that the types match
            let attachment = unsafe { self.downcast_attachment_unchecked() };

            Some(attachment)
        } else {
            None
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
    /// [`inner_type_id()`]: ReportAttachmentRef::inner_type_id
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
    /// let attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = attachment.as_ref();
    ///
    /// // SAFETY: We know the attachment contains &str data
    /// let typed_ref: ReportAttachmentRef<'_, &str> =
    ///     unsafe { attachment_ref.downcast_attachment_unchecked() };
    /// ```
    #[must_use]
    pub unsafe fn downcast_attachment_unchecked<A>(self) -> ReportAttachmentRef<'a, A>
    where
        A: Sized + 'static,
    {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. `A` is bounded by `Sized` in the function signature, so this is satisfied.
        // 2. Guaranteed by the caller
        unsafe { ReportAttachmentRef::from_raw(raw) }
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
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = attachment.as_ref();
    ///
    /// // Try to downcast to the correct type
    /// let data: Option<&&str> = attachment_ref.downcast_inner();
    /// assert_eq!(data, Some(&"text data"));
    ///
    /// // Try to downcast to an incorrect type
    /// let wrong_data: Option<&i32> = attachment_ref.downcast_inner();
    /// assert!(wrong_data.is_none());
    /// ```
    ///
    /// [`downcast_attachment`]: Self::downcast_attachment
    #[must_use]
    pub fn downcast_inner<A>(self) -> Option<&'a A>
    where
        A: Sized + 'static,
    {
        Some(self.downcast_attachment()?.inner())
    }

    /// Performs an unchecked downcast of the inner attachment data to a
    /// reference of type `A`.
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
    /// use rootcause::{
    ///     markers::Dynamic,
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<Dynamic> = attachment.into_dynamic();
    /// let attachment_ref: ReportAttachmentRef<'_, Dynamic> = attachment.as_ref();
    ///
    /// // SAFETY: We know the attachment contains &str data
    /// let data: &&str = unsafe { attachment_ref.downcast_inner_unchecked() };
    /// assert_eq!(*data, "text data");
    /// ```
    #[must_use]
    pub unsafe fn downcast_inner_unchecked<A>(self) -> &'a A
    where
        A: Sized + 'static,
    {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by the caller
        unsafe { raw.attachment_downcast_unchecked() }
    }
}

impl<'a, A: ?Sized> core::fmt::Display for ReportAttachmentRef<'a, A> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let report: ReportAttachmentRef<'_, Dynamic> = self.into_dynamic();
        crate::hooks::attachment_formatter::display_attachment(report, None, formatter)
    }
}

impl<'a, A: ?Sized> core::fmt::Debug for ReportAttachmentRef<'a, A> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let report: ReportAttachmentRef<'_, Dynamic> = self.into_dynamic();
        crate::hooks::attachment_formatter::debug_attachment(report, None, formatter)
    }
}

impl<'a, A: ?Sized> Unpin for ReportAttachmentRef<'a, A> {}

impl<'a, A: Sized> From<ReportAttachmentRef<'a, A>> for ReportAttachmentRef<'a, Dynamic> {
    fn from(value: ReportAttachmentRef<'a, A>) -> Self {
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
    fn test_attachment_ref_send_sync() {
        static_assertions::assert_not_impl_any!(ReportAttachmentRef<'static, ()>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentRef<'static, String>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentRef<'static, NonSend>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachmentRef<'static, Dynamic>: Send, Sync);
    }

    #[test]
    fn test_attachment_ref_unpin() {
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, ()>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, String>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, NonSend>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, Dynamic>: Unpin);
    }

    #[test]
    fn test_attachment_ref_copy_clone() {
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, ()>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, String>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, NonSend>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, Dynamic>: Copy, Clone);
    }
}
