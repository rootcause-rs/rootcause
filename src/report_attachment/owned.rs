use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use rootcause_internals::{
    RawAttachment, RawAttachmentRef,
    handlers::{AttachmentFormattingStyle, FormattingFunction},
};

use crate::{
    handlers::{self, AttachmentHandler},
    markers::{self, Local, ObjectMarker, SendSync},
    report_attachment::ReportAttachmentRef,
    util::format_helper,
};

/// An attachment to be attached to a [`Report`](crate::Report).
///
/// Attachments can hold any type of data, and can be formatted using custom
/// handlers. The attachment can be marked as either `SendSync` or `Local`,
/// indicating whether it is safe to send the attachment across threads or not.
///
/// # Type Parameters
/// - `Attachment`: The type of the attachment. This can either be a concrete
///   type, or `dyn Any`.
/// - `ThreadSafety`: The thread safety marker for the attachment. This can
///   either be `SendSync` or `Local`.
#[repr(transparent)]
pub struct ReportAttachment<Attachment = dyn Any, ThreadSafety = SendSync>
where
    Attachment: markers::ObjectMarker + ?Sized,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: RawAttachment,
    _attachment: PhantomData<Attachment>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<A, T> ReportAttachment<A, T>
where
    A: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new Attachment from a raw attachment
    ///
    /// # Safety
    ///
    /// - The attachment embedded in the [`RawAttachment`] must either be the
    ///   type `A`, or `A` must be the type `dyn Any`.
    /// - The thread safety marker must match the contents of the attachment.
    ///   More specifically if the marker is [`SendSync`], then the inner
    ///   attachment must be `Send+Sync`
    pub(crate) unsafe fn from_raw(raw: RawAttachment) -> Self {
        ReportAttachment {
            raw,
            _attachment: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    /// Consumes the [`ReportAttachment`] and returns the inner
    /// [`RawAttachment`].
    pub(crate) fn into_raw(self) -> RawAttachment {
        self.raw
    }

    /// Creates a lifetime-bound [`RawAttachmentRef`] from the inner
    /// [`RawAttachment`].
    pub(crate) fn as_raw_ref(&self) -> RawAttachmentRef<'_> {
        self.raw.as_ref()
    }

    /// Allocates a new [`ReportAttachment`] with the given attachment as the
    /// data.
    ///
    /// The new attachment will use the [`handlers::Display`] handler to format
    /// the attachment. See [`ReportAttachment::new_custom`] if you want to
    /// control the handler used.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{prelude::*, report_attachment::ReportAttachment};
    ///
    /// let attachment = ReportAttachment::new("This is an attachment");
    /// let mut report = report!("An error occurred");
    /// report.attachments_mut().push(attachment.into_dyn_any());
    /// ```
    pub fn new(attachment: A) -> Self
    where
        A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug + Sized,
    {
        Self::new_custom::<handlers::Display>(attachment)
    }

    /// Allocates a new [`ReportAttachment`] with the given attachment as the
    /// data and the given handler to format it.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{prelude::*, report_attachment::ReportAttachment};
    ///
    /// #[derive(Debug)]
    /// struct MyAttachmentType {
    ///     data: String,
    /// }
    /// let attachment = ReportAttachment::new_custom::<handlers::Debug>(MyAttachmentType {
    ///     data: "Important data".to_string(),
    /// });
    /// let mut report = report!("An error occurred");
    /// report.attachments_mut().push(attachment.into_dyn_any());
    /// ```
    pub fn new_custom<H>(attachment: A) -> Self
    where
        A: markers::ObjectMarkerFor<T> + Sized,
        H: AttachmentHandler<A>,
    {
        let raw = RawAttachment::new::<A, H>(attachment);
        // SAFETY: The inner attachment is of type `A`, which is `Send+Sync` because of
        // the bounds on this function
        unsafe { ReportAttachment::from_raw(raw) }
    }

    /// Changes the inner attachment type of the [`ReportAttachment`] to [`dyn
    /// Any`].
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
    /// [`ReportAttachment::downcast_attachment`].
    pub fn into_dyn_any(self) -> ReportAttachment<dyn Any, T> {
        unsafe { ReportAttachment::from_raw(self.into_raw()) }
    }

    /// Changes the thread safety mode of the [`ReportAttachment`] to [`Local`].
    ///
    /// Calling this method is equivalent to calling `attachment.into()`,
    /// however this method has been restricted to only change the thread
    /// safety mode to [`Local`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the attachment in any way. It only
    /// has the effect of "forgetting" that the object in the
    /// [`ReportAttachment`] might actually be [`Send`] and [`Sync`].
    pub fn into_local(self) -> ReportAttachment<A, Local> {
        unsafe { ReportAttachment::from_raw(self.into_raw()) }
    }

    /// Returns a reference to the inner attachment.
    ///
    /// This method is only available when the attachment type is a specific
    /// type, and not `dyn Any`.
    pub fn inner(&self) -> &A
    where
        A: Sized,
    {
        unsafe { self.as_raw_ref().attachment_downcast_unchecked() }
    }

    /// Returns the [`TypeId`] of the inner attachment.
    pub fn inner_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_type_id()
    }

    /// Returns the [`TypeId`] of the handler used when creating this
    /// attachment.
    pub fn inner_handler_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_handler_type_id()
    }

    /// Formats the attachment with hook processing.
    pub fn format_inner(&self) -> impl core::fmt::Display + core::fmt::Debug {
        let attachment: ReportAttachmentRef<'_, dyn Any> = self.as_ref().into_dyn_any();
        format_helper(
            attachment,
            |attachment, formatter| {
                crate::hooks::formatting_overrides::display_attachment(attachment, None, formatter)
            },
            |attachment, formatter| {
                crate::hooks::formatting_overrides::debug_attachment(attachment, None, formatter)
            },
        )
    }

    /// Formats the attachment without hook processing.
    pub fn format_inner_unhooked(&self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.as_raw_ref(),
            |attachment, formatter| attachment.attachment_display(formatter),
            |attachment, formatter| attachment.attachment_debug(formatter),
        )
    }

    /// Gets the preferred formatting style for the attachment with hook
    /// processing.
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
        crate::hooks::formatting_overrides::get_preferred_formatting_style(
            self.as_ref().into_dyn_any(),
            report_formatting_function,
        )
    }

    /// Gets the preferred formatting style for the attachment with hook
    /// processing.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this
    ///   attachment will be embedded is being formatted using [`Display`]
    ///   formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    pub fn preferred_formatting_style_unhooked(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.as_raw_ref()
            .preferred_formatting_style(report_formatting_function)
    }

    /// Returns an reference to the attachment.
    pub fn as_ref(&self) -> ReportAttachmentRef<'_, A> {
        unsafe { ReportAttachmentRef::from_raw(self.as_raw_ref()) }
    }
}

impl<A> ReportAttachment<A, SendSync>
where
    A: markers::ObjectMarker,
{
    /// Creates a new [`ReportAttachment`] with [`SendSync`] thread safety.
    ///
    /// This is a convenience method that calls [`ReportAttachment::new`] with
    /// explicit [`SendSync`] thread safety. Use this method when you're
    /// having trouble with type inference for the thread safety parameter.
    ///
    /// The context will use the [`handlers::Display`] handler to format the
    /// attachment.
    pub fn new_sendsync(attachment: A) -> Self
    where
        A: core::fmt::Display + core::fmt::Debug + Send + Sync,
    {
        Self::new(attachment)
    }

    /// Creates a new [`ReportAttachment`] with [`SendSync`] thread safety and
    /// the given handler.
    ///
    /// This is a convenience method that calls [`ReportAttachment::new_custom`]
    /// with explicit [`SendSync`] thread safety. Use this method when
    /// you're having trouble with type inference for the thread safety
    /// parameter.
    pub fn new_sendsync_custom<H>(attachment: A) -> Self
    where
        A: Send + Sync + 'static,
        H: AttachmentHandler<A>,
    {
        Self::new_custom::<H>(attachment)
    }
}

impl<A> ReportAttachment<A, Local>
where
    A: markers::ObjectMarker,
{
    /// Creates a new [`ReportAttachment`] with [`Local`] thread safety.
    ///
    /// This is a convenience method that calls [`ReportAttachment::new`] with
    /// explicit [`Local`] thread safety. Use this method when you're having
    /// trouble with type inference for the thread safety parameter.
    ///
    /// The context will use the [`handlers::Display`] handler to format the
    /// attachment.
    pub fn new_local(attachment: A) -> Self
    where
        A: core::fmt::Display + core::fmt::Debug,
    {
        Self::new_custom::<handlers::Display>(attachment)
    }

    /// Creates a new [`ReportAttachment`] with [`Local`] thread safety and the
    /// given handler.
    ///
    /// This is a convenience method that calls [`ReportAttachment::new_custom`]
    /// with explicit [`Local`] thread safety. Use this method when you're
    /// having trouble with type inference for the thread safety parameter.
    pub fn new_local_custom<H>(attachment: A) -> Self
    where
        H: AttachmentHandler<A>,
    {
        Self::new_custom::<H>(attachment)
    }
}

impl<T> ReportAttachment<dyn Any, T>
where
    T: markers::ThreadSafetyMarker,
{
    /// Attempts to downcast the inner attachment to a specific type.
    ///
    /// Returns `Some(&A)` if the current context is of type `A`, otherwise
    /// returns `None`.
    pub fn downcast_inner<A>(&self) -> Option<&A>
    where
        A: ObjectMarker,
    {
        self.as_raw_ref().attachment_downcast()
    }

    /// Downcasts the current context to a specific type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the inner attachment is actually of type
    /// `A`. This can be verified by calling [`inner_type_id()`] first.
    ///
    /// [`inner_type_id()`]: ReportAttachment::inner_type_id
    pub unsafe fn downcast_inner_unchecked<A>(&self) -> &A
    where
        A: ObjectMarker,
    {
        unsafe { self.as_raw_ref().attachment_downcast_unchecked() }
    }

    /// Attempts to downcast the [`ReportAttachment`] to a specific attachment
    /// type.
    ///
    /// Returns `Ok(attachment)` if the inner attachment is of type `A`,
    /// otherwise returns `Err(self)` with the original [`ReportAttachment`].
    pub fn downcast_attachment<A>(self) -> Result<ReportAttachment<A, T>, Self>
    where
        A: markers::ObjectMarker + ?Sized,
    {
        if TypeId::of::<A>() == TypeId::of::<dyn Any>() || TypeId::of::<A>() == self.inner_type_id()
        {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }

    /// Downcasts the [`ReportAttachment`] to a specific attachment type without
    /// checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the inner attachment is actually of type
    /// `A`. This can be verified by calling [`inner_type_id()`] first.
    ///
    /// [`inner_type_id()`]: ReportAttachment::inner_type_id
    pub unsafe fn downcast_unchecked<A>(self) -> ReportAttachment<A, T>
    where
        A: markers::ObjectMarker + ?Sized,
    {
        unsafe { ReportAttachment::from_raw(self.into_raw()) }
    }
}

impl<A, T> From<A> for ReportAttachment<A, T>
where
    A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
    T: markers::ThreadSafetyMarker,
{
    fn from(attachment: A) -> Self {
        ReportAttachment::new_custom::<handlers::Display>(attachment)
    }
}

impl<A, T> From<A> for ReportAttachment<dyn Any, T>
where
    A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
    T: markers::ThreadSafetyMarker,
{
    fn from(attachment: A) -> Self {
        ReportAttachment::new_custom::<handlers::Display>(attachment).into_dyn_any()
    }
}

unsafe impl<A> Send for ReportAttachment<A, SendSync> where A: markers::ObjectMarker + ?Sized {}
unsafe impl<A> Sync for ReportAttachment<A, SendSync> where A: markers::ObjectMarker + ?Sized {}

mod from_impls {
    use super::*;

    macro_rules! unsafe_attachment_to_attachment {
        ($(
            <
                $($param:ident),*
            >:
            $context1:ty => $context2:ty,
            $thread_safety1:ty => $thread_safety2:ty
        ),* $(,)?) => {
            $(
                impl<$($param),*> From<ReportAttachment<$context1, $thread_safety1>> for ReportAttachment<$context2, $thread_safety2>
                    where $(
                        $param: markers::ObjectMarker,
                    )*
                {
                    fn from(attachment: ReportAttachment<$context1, $thread_safety1>) -> Self {
                        // SAFETY:
                        // - The attachment type is valid, because it either doesn't change or goes from a known `A` to `dyn Any`.
                        // - The thread marker is valid, because it either does not change or it goes from `SendSync` to `Local`.
                        unsafe { ReportAttachment::from_raw(attachment.into_raw()) }
                    }
                }
            )*
        };
}

    unsafe_attachment_to_attachment!(
        <C>: C => C, SendSync => Local,
        <C>: C => dyn Any, SendSync => SendSync,
        <C>: C => dyn Any, SendSync => Local,
        <C>: C => dyn Any, Local => Local,
        <>:  dyn Any => dyn Any, SendSync => Local,
    );
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[allow(dead_code)]
    struct NonSend(*const ());
    static_assertions::assert_not_impl_any!(NonSend: Send, Sync);

    #[test]
    fn test_attachment_send_sync() {
        static_assertions::assert_impl_all!(ReportAttachment<(), SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportAttachment<String, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportAttachment<NonSend, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportAttachment<dyn Any, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportAttachment<(), Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachment<String, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachment<NonSend, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachment<dyn Any, Local>: Send, Sync);
    }

    #[test]
    fn test_attachment_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportAttachment<(), SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<String, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<NonSend, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<dyn Any, SendSync>: Copy, Clone);

        static_assertions::assert_not_impl_any!(ReportAttachment<(), Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<String, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<NonSend, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<dyn Any, Local>: Copy, Clone);
    }
}
