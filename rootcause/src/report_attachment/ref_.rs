use alloc::fmt;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use rootcause_internals::{
    RawAttachmentRef,
    handlers::{AttachmentFormattingStyle, FormattingFunction},
};

use crate::{hooks::AttachmentParent, markers, util::format_helper};

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
    /// The thread safety marker must match the contents of the attachment. More specifically if the marker is `SendSync`, then
    /// the inner attachment must be `Send+Sync`
    pub(crate) unsafe fn from_raw(raw: RawAttachmentRef<'a>) -> Self {
        ReportAttachmentRef {
            raw,
            _attachment: PhantomData,
        }
    }

    pub(crate) fn as_raw_ref(self) -> RawAttachmentRef<'a> {
        self.raw
    }

    pub fn inner_type_id(self) -> TypeId {
        self.as_raw_ref().attachment_type_id()
    }

    pub fn inner_handler_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_handler_type_id()
    }

    pub fn format_inner_unhooked(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.as_raw_ref(),
            |attachment, formatter| attachment.attachment_display(formatter),
            |attachment, formatter| attachment.attachment_debug(formatter),
        )
    }

    pub fn format_inner(self) -> impl core::fmt::Display + core::fmt::Debug {
        let attachment: ReportAttachmentRef<'a, dyn Any> =
            unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) };
        format_helper(
            attachment,
            |attachment, formatter| crate::hooks::display_attachment(attachment, None, formatter),
            |attachment, formatter| crate::hooks::debug_attachment(attachment, None, formatter),
        )
    }

    /// Changes the inner attachment type of the [`ReportAttachmentRef`] to [`dyn Any`].
    ///
    /// Calling this method is equivalent to calling `attachment.into()`, however this method
    /// has been restricted to only change the attachment type to `dyn Any`.
    ///
    /// This method can be useful to help with type inference or to improve code readability,
    /// as it more clearly communicates intent.
    ///
    /// This method does not actually modify the attachment in any way. It only has the effect of "forgetting" that
    /// that the inner attachment actually has the type `A`.
    ///
    /// To get back the attachment with a concrete `A` you can use the method [`ReportAttachmentRef::downcast_attachment`].
    pub fn into_dyn_any(self) -> ReportAttachmentRef<'a, dyn Any> {
        unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) }
    }

    pub fn inner(self) -> &'a A
    where
        A: Sized,
    {
        unsafe { self.downcast_inner_unchecked() }
    }

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

    pub unsafe fn downcast_attachment_unchecked<B>(self) -> ReportAttachmentRef<'a, B>
    where
        B: markers::ObjectMarker + ?Sized,
    {
        unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) }
    }

    pub fn downcast_inner<B>(self) -> Option<&'a B>
    where
        B: markers::ObjectMarker,
    {
        self.as_raw_ref().attachment_downcast()
    }

    pub unsafe fn downcast_inner_unchecked<B>(self) -> &'a B
    where
        B: markers::ObjectMarker,
    {
        unsafe { self.as_raw_ref().attachment_downcast_unchecked() }
    }

    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this attachment will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    /// - `report_formatting_alternate`: Whether the report in which this attachment will be embedded is being formatted using the [`alternate`] mode
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`alternate`]: core::fmt::Formatter::alternate
    pub fn preferred_formatting_style(
        &self,
        attachment_parent: AttachmentParent<'_>,
        report_formatting_function: FormattingFunction,
        report_formatting_alternate: bool,
    ) -> AttachmentFormattingStyle {
        crate::hooks::get_preferred_formatting_style(
            self.into_dyn_any(),
            attachment_parent,
            report_formatting_function,
            report_formatting_alternate,
        )
    }

    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this attachment will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    /// - `report_formatting_alternate`: Whether the report in which this attachment will be embedded is being formatted using the [`alternate`] mode
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`alternate`]: core::fmt::Formatter::alternate
    pub fn preferred_formatting_style_unhooked(
        &self,
        report_formatting_function: FormattingFunction,
        report_formatting_alternate: bool,
    ) -> AttachmentFormattingStyle {
        self.as_raw_ref()
            .preferred_formatting_style(report_formatting_function, report_formatting_alternate)
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
