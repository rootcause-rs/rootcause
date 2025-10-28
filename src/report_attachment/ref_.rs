use alloc::fmt;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use rootcause_internals::{
    RawAttachmentRef,
    handlers::{AttachmentFormattingStyle, FormattingFunction},
};

use crate::{markers, util::format_helper};

/// A reference to a [`ReportAttachment`].
///
/// # Examples
/// ```
/// # use core::any::Any;
/// use rootcause::{
///     prelude::*,
///     report_attachment::{ReportAttachment, ReportAttachmentRef},
/// };
///
/// let attachment: ReportAttachment<&str> = ReportAttachment::new("An important attachment");
/// let attachment_ref: ReportAttachmentRef<'_, &str> = attachment.as_ref();
///
/// let mut report = report!("An error occurred");
/// report.attachments_mut().push(attachment.into_dyn_any());
///
/// // You can also get an attachment reference through the attachments on a report
/// let attachment_ref: ReportAttachmentRef<'_, dyn Any> = report.attachments().get(0).unwrap();
/// ```
///
/// [`ReportAttachment`]: crate::report_attachment::ReportAttachment
#[repr(transparent)]
pub struct ReportAttachmentRef<'a, Attachment = dyn Any>
where
    Attachment: markers::ObjectMarker + ?Sized,
{
    raw: RawAttachmentRef<'a>,
    _attachment: PhantomData<Attachment>,
}

impl<'a, A> Copy for ReportAttachmentRef<'a, A> where A: markers::ObjectMarker + ?Sized {}
impl<'a, A> Clone for ReportAttachmentRef<'a, A>
where
    A: markers::ObjectMarker + ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, A> ReportAttachmentRef<'a, A>
where
    A: markers::ObjectMarker + ?Sized,
{
    /// Creates a new AttachmentRef from a raw attachment reference
    ///
    /// # Safety
    ///
    /// To call this method you must ensure the following:
    ///
    /// - The attachment embedded in the RawAttachmentRef must match the `C` of
    ///   the output type, or the `C` of the output type must be `dyn Any`
    /// - The thread safety marker must match the contents of the attachment.
    ///   More specifically if the marker is [`SendSync`], then the inner
    ///   attachment must be `Send+Sync`
    ///
    /// [`SendSync`]: crate::markers::SendSync
    pub(crate) unsafe fn from_raw(raw: RawAttachmentRef<'a>) -> Self {
        ReportAttachmentRef {
            raw,
            _attachment: PhantomData,
        }
    }

    pub(crate) fn as_raw_ref(self) -> RawAttachmentRef<'a> {
        self.raw
    }

    /// Returns the [`TypeId`] of the inner attachment.
    ///
    /// # Examples
    /// ```
    /// # use core::any::Any;
    /// use std::any::TypeId;
    ///
    /// use rootcause::{
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<dyn Any> = attachment.into_dyn_any();
    /// let attachment_ref: ReportAttachmentRef<'_, dyn Any> = attachment.as_ref();
    /// assert_eq!(attachment_ref.inner_type_id(), TypeId::of::<&str>());
    /// ```
    pub fn inner_type_id(self) -> TypeId {
        self.as_raw_ref().attachment_type_id()
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
    pub fn inner_handler_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_handler_type_id()
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
    pub fn format_inner_unhooked(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.as_raw_ref(),
            |attachment, formatter| attachment.attachment_display(formatter),
            |attachment, formatter| attachment.attachment_debug(formatter),
        )
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
    pub fn format_inner(self) -> impl core::fmt::Display + core::fmt::Debug {
        let attachment: ReportAttachmentRef<'a, dyn Any> =
            unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) };
        format_helper(
            attachment,
            |attachment, formatter| crate::hooks::display_attachment(attachment, None, formatter),
            |attachment, formatter| crate::hooks::debug_attachment(attachment, None, formatter),
        )
    }

    /// Changes the inner attachment type of the [`ReportAttachmentRef`] to
    /// [`dyn Any`].
    ///
    /// Calling this method is equivalent to calling `attachment.into()`,
    /// however this method has been restricted to only change the
    /// attachment type to `dyn Any`.
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the attachment in any way. It only
    /// has the effect of "forgetting" that that the inner attachment
    /// actually has the type `A`.
    ///
    /// To get back the attachment with a concrete `A` you can use the method
    /// [`ReportAttachmentRef::downcast_attachment`].
    pub fn into_dyn_any(self) -> ReportAttachmentRef<'a, dyn Any> {
        unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) }
    }

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
    pub fn inner(self) -> &'a A
    where
        A: Sized,
    {
        unsafe { self.downcast_inner_unchecked() }
    }

    /// Attempts to downcast the attachment reference to a different type `B`.
    ///
    /// This method performs a safe type cast, returning [`Some`] if the
    /// attachment actually contains data of type `B`, or [`None`] if the
    /// types don't match.
    ///
    /// This method is most useful when going from a `dyn Any` to a concrete
    /// `B`.
    ///
    /// It is possible to use this when you have a concrete type `A`, however
    /// this is unlikely to be useful outside of very generic or macro-heavy
    /// code.
    ///
    /// # Examples
    /// ```
    /// # use core::any::Any;
    /// use rootcause::{
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<dyn Any> = attachment.into_dyn_any();
    /// let attachment_ref: ReportAttachmentRef<'_, dyn Any> = attachment.as_ref();
    ///
    /// // Try to downcast to the correct type
    /// let typed_ref: Option<ReportAttachmentRef<'_, &str>> = attachment_ref.downcast_attachment();
    /// assert!(typed_ref.is_some());
    ///
    /// // Try to downcast to an incorrect type
    /// let wrong_ref: Option<ReportAttachmentRef<'_, i32>> = attachment_ref.downcast_attachment();
    /// assert!(wrong_ref.is_none());
    /// ```
    pub fn downcast_attachment<B>(self) -> Option<ReportAttachmentRef<'a, B>>
    where
        B: markers::ObjectMarker + ?Sized,
    {
        if TypeId::of::<B>() == TypeId::of::<dyn Any>() || TypeId::of::<B>() == self.inner_type_id()
        {
            Some(unsafe { self.downcast_attachment_unchecked() })
        } else {
            None
        }
    }

    /// Performs an unchecked downcast of the attachment reference to type `B`.
    ///
    /// This method bypasses type checking and performs the cast without
    /// verifying that the attachment actually contains data of type `B`. It
    /// is the caller's responsibility to ensure the cast is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the inner attachment is actually of type
    /// `A`. This can be verified by calling [`inner_type_id()`] first.
    ///
    /// [`inner_type_id()`]: ReportAttachmentRef::inner_type_id
    ///
    /// # Examples
    /// ```
    /// # use core::any::Any;
    /// use rootcause::{
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<dyn Any> = attachment.into_dyn_any();
    /// let attachment_ref: ReportAttachmentRef<'_, dyn Any> = attachment.as_ref();
    ///
    /// // SAFETY: We know the attachment contains &str data
    /// let typed_ref: ReportAttachmentRef<'_, &str> =
    ///     unsafe { attachment_ref.downcast_attachment_unchecked() };
    /// ```
    pub unsafe fn downcast_attachment_unchecked<B>(self) -> ReportAttachmentRef<'a, B>
    where
        B: markers::ObjectMarker + ?Sized,
    {
        unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) }
    }

    /// Attempts to downcast the inner attachment data to a reference of type
    /// `B`.
    ///
    /// This method performs a safe type cast, returning [`Some`] with a
    /// reference to the data if the attachment actually contains data of
    /// type `B`, or [`None`] if the types don't match. Unlike
    /// [`downcast_attachment`], this method returns a direct reference to
    /// the data rather than a [`ReportAttachmentRef`].
    ///
    /// # Examples
    /// ```
    /// # use core::any::Any;
    /// use rootcause::{
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<dyn Any> = attachment.into_dyn_any();
    /// let attachment_ref: ReportAttachmentRef<'_, dyn Any> = attachment.as_ref();
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
    pub fn downcast_inner<B>(self) -> Option<&'a B>
    where
        B: markers::ObjectMarker,
    {
        self.as_raw_ref().attachment_downcast()
    }

    /// Performs an unchecked downcast of the inner attachment data to a
    /// reference of type `B`.
    ///
    /// This method bypasses type checking and performs the cast without
    /// verifying that the attachment actually contains data of type `B`. It
    /// returns a direct reference to the data. It is the caller's
    /// responsibility to ensure the cast is valid.
    ///
    /// # Safety
    /// The caller must guarantee that the attachment actually contains data of
    /// type `B`. Violating this requirement leads to undefined behavior.
    ///
    /// # Type Parameters
    /// - `B`: The target type to downcast to. Must implement
    ///   [`markers::ObjectMarker`] and be [`Sized`].
    ///
    /// # Examples
    /// ```
    /// # use core::any::Any;
    /// use rootcause::{
    ///     prelude::*,
    ///     report_attachment::{ReportAttachment, ReportAttachmentRef},
    /// };
    ///
    /// let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
    /// let attachment: ReportAttachment<dyn Any> = attachment.into_dyn_any();
    /// let attachment_ref: ReportAttachmentRef<'_, dyn Any> = attachment.as_ref();
    ///
    /// // SAFETY: We know the attachment contains &str data
    /// let data: &&str = unsafe { attachment_ref.downcast_inner_unchecked() };
    /// assert_eq!(*data, "text data");
    /// ```
    pub unsafe fn downcast_inner_unchecked<B>(self) -> &'a B
    where
        B: markers::ObjectMarker,
    {
        unsafe { self.as_raw_ref().attachment_downcast_unchecked() }
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
    pub fn preferred_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        crate::hooks::get_preferred_formatting_style(
            self.into_dyn_any(),
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
    pub fn preferred_formatting_style_unhooked(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.as_raw_ref()
            .preferred_formatting_style(report_formatting_function)
    }
}

impl<'a, A> core::fmt::Display for ReportAttachmentRef<'a, A>
where
    A: markers::ObjectMarker + ?Sized,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let report: ReportAttachmentRef<'_, dyn Any> = self.into_dyn_any();
        crate::hooks::display_attachment(report, None, formatter)
    }
}

impl<'a, A> core::fmt::Debug for ReportAttachmentRef<'a, A>
where
    A: markers::ObjectMarker + ?Sized,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let report: ReportAttachmentRef<'_, dyn Any> = self.into_dyn_any();
        crate::hooks::debug_attachment(report, None, formatter)
    }
}

impl<'a, A> From<ReportAttachmentRef<'a, A>> for ReportAttachmentRef<'a, dyn Any>
where
    A: markers::ObjectMarker,
{
    fn from(value: ReportAttachmentRef<'a, A>) -> Self {
        value.into_dyn_any()
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
        static_assertions::assert_not_impl_any!(ReportAttachmentRef<'static, dyn Any>: Send, Sync);
    }

    #[test]
    fn test_attachment_ref_copy_clone() {
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, ()>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, String>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, NonSend>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportAttachmentRef<'static, dyn Any>: Copy, Clone);
    }
}
