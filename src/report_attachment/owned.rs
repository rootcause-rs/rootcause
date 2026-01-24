use core::any::TypeId;

use rootcause_internals::{
    RawAttachment,
    handlers::{AttachmentFormattingStyle, FormattingFunction},
};

use crate::{
    handlers::{self, AttachmentHandler},
    markers::{self, Dynamic, Local, SendSync},
    report_attachment::ReportAttachmentRef,
    util::format_helper,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::{RawAttachment, RawAttachmentRef};

    use crate::markers::{Dynamic, SendSync};

    /// An attachment to be attached to a [`Report`](crate::Report).
    ///
    /// Attachments can hold any type of data, and can be formatted using custom
    /// handlers. The attachment can be marked as either [`SendSync`] or
    /// [`Local`], indicating whether it is safe to send the attachment
    /// across threads or not.
    ///
    /// # Type Parameters
    /// - `Attachment`: The type of the attachment. This can either be a
    ///   concrete type, or [`Dynamic`].
    /// - `ThreadSafety`: The thread safety marker for the attachment. This can
    ///   either be [`SendSync`] or [`Local`].
    ///
    /// [`SendSync`]: crate::markers::SendSync
    /// [`Local`]: crate::markers::Local
    #[repr(transparent)]
    pub struct ReportAttachment<
        Attachment: ?Sized + 'static = Dynamic,
        ThreadSafety: 'static = SendSync,
    > {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. `T` must either be `SendSync` or `Local`.
        /// 3. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachment`] must be of type `A`.
        /// 4. If `T = SendSync`: The attachment embedded in the
        ///    [`RawAttachment`] must be `Send + Sync`.
        raw: RawAttachment,
        _attachment: PhantomData<Attachment>,
        _thread_safety: PhantomData<ThreadSafety>,
    }

    impl<A: ?Sized, T> ReportAttachment<A, T> {
        /// Creates a new Attachment from a raw attachment
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. `A` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. `T` must either be `SendSync` or `Local`.
        /// 3. If `A` is a `Sized` type: The attachment embedded in the
        ///    [`RawAttachment`] must be of type `A`.
        /// 4. If `T = SendSync`: The attachment embedded in the
        ///    [`RawAttachment`] must be `Send + Sync`.
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawAttachment) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by caller
            // 2. Guaranteed by caller
            // 3. Guaranteed by caller
            // 4. Guaranteed by caller
            ReportAttachment {
                raw,
                _attachment: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Consumes the [`ReportAttachment`] and returns the inner
        /// [`RawAttachment`].
        #[must_use]
        pub(crate) fn into_raw(self) -> RawAttachment {
            // SAFETY: We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }

        /// Creates a lifetime-bound [`RawAttachmentRef`] from the inner
        /// [`RawAttachment`].
        #[must_use]
        pub(crate) fn as_raw_ref(&self) -> RawAttachmentRef<'_> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. No mutation is possible through the `RawAttachmentRef`
            // 4. No mutation is possible through the `RawAttachmentRef`
            let raw = &self.raw;

            raw.as_ref()
        }
    }
}
pub use limit_field_access::ReportAttachment;

impl<A: Sized, T> ReportAttachment<A, T> {
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
    /// report.attachments_mut().push(attachment.into_dynamic());
    /// ```
    #[must_use]
    pub fn new(attachment: A) -> Self
    where
        A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
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
    /// report.attachments_mut().push(attachment.into_dynamic());
    /// ```
    #[must_use]
    pub fn new_custom<H>(attachment: A) -> Self
    where
        A: markers::ObjectMarkerFor<T>,
        H: AttachmentHandler<A>,
    {
        let raw = RawAttachment::new::<A, H>(attachment);

        // SAFETY:
        // 1. `A` is bounded by `Sized` in this impl, so this is trivially true.
        // 2. `A` is bounded by `markers::ObjectMarkerFor<T>` and this can only be
        //    implemented for `T=Local` and `T=SendSync`, so this is
        //   upheld.
        // 3. We just created the `RawAttachment` and it does indeed have an attachment
        //    of type `A`.
        // 4. If `T=Local`, then this is trivially true. If `T=SendSync`, then the bound
        //    `A: ObjectMarkerFor<SendSync>` guarantees that the attachment is
        //    `Send+Sync`.
        unsafe {
            // @add-unsafe-context: markers::ObjectMarkerFor
            ReportAttachment::from_raw(raw)
        }
    }

    /// Returns a reference to the inner attachment.
    ///
    /// This method is only available when the attachment type is a specific
    /// type, and not [`Dynamic`].
    #[must_use]
    pub fn inner(&self) -> &A {
        self.as_ref().inner()
    }
}

impl<A: ?Sized, T> ReportAttachment<A, T> {
    /// Changes the inner attachment type of the [`ReportAttachment`] to
    /// [`Dynamic`]
    ///
    /// Calling this method is equivalent to calling `attachment.into()`,
    /// however this method has been restricted to only change the
    /// attachment type to [`Dynamic`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the attachment in any way. It only
    /// has the effect of "forgetting" that the inner attachment actually has
    /// the type `A`.
    ///
    /// To get back the attachment with a concrete `A` you can use the method
    /// [`ReportAttachment::downcast_attachment`].
    #[must_use]
    pub fn into_dynamic(self) -> ReportAttachment<Dynamic, T> {
        let raw = self.into_raw();

        // SAFETY:
        // 1. `A=Dynamic`, so this is trivially true.
        // 2. Guaranteed by the invariants of this type.
        // 3. `A=Dynamic`, so this is trivially true.
        // 4. Guaranteed by the invariants of this type.
        unsafe {
            // @add-unsafe-context: Dynamic
            ReportAttachment::<Dynamic, T>::from_raw(raw)
        }
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
    #[must_use]
    pub fn into_local(self) -> ReportAttachment<A, Local> {
        let raw = self.into_raw();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. `T=Local`, so this is trivially true.
        // 3. Guaranteed by the invariants of this type.
        // 4. `T=Local`, so this is trivially true.
        unsafe { ReportAttachment::from_raw(raw) }
    }

    /// Returns the [`TypeId`] of the inner attachment.
    #[must_use]
    pub fn inner_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_type_id()
    }

    /// Returns the [`core::any::type_name`] of the inner attachment.
    #[must_use]
    pub fn inner_type_name(&self) -> &'static str {
        self.as_raw_ref().attachment_type_name()
    }

    /// Returns the [`TypeId`] of the handler used when creating this
    /// attachment.
    #[must_use]
    pub fn inner_handler_type_id(&self) -> TypeId {
        self.as_raw_ref().attachment_handler_type_id()
    }

    /// Formats the attachment with hook processing.
    #[must_use]
    pub fn format_inner(&self) -> impl core::fmt::Display + core::fmt::Debug {
        let attachment: ReportAttachmentRef<'_, Dynamic> = self.as_ref().into_dynamic();
        format_helper(
            attachment,
            |attachment, formatter| {
                crate::hooks::attachment_formatter::display_attachment(attachment, None, formatter)
            },
            |attachment, formatter| {
                crate::hooks::attachment_formatter::debug_attachment(attachment, None, formatter)
            },
        )
    }

    /// Formats the attachment without hook processing.
    #[must_use]
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
    #[must_use]
    pub fn preferred_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        crate::hooks::attachment_formatter::get_preferred_formatting_style(
            self.as_ref().into_dynamic(),
            report_formatting_function,
        )
    }

    /// Gets the preferred formatting style for the attachment without hook
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
    #[must_use]
    pub fn preferred_formatting_style_unhooked(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.as_raw_ref()
            .preferred_formatting_style(report_formatting_function)
    }

    /// Returns a reference to the attachment.
    #[must_use]
    pub fn as_ref(&self) -> ReportAttachmentRef<'_, A> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. Guaranteed by the invariants of this type.
        unsafe { ReportAttachmentRef::from_raw(raw) }
    }
}

impl<A: Sized + Send + Sync> ReportAttachment<A, SendSync> {
    /// Creates a new [`ReportAttachment`] with [`SendSync`] thread safety.
    ///
    /// This is a convenience method that calls [`ReportAttachment::new`] with
    /// explicit [`SendSync`] thread safety. Use this method when you're
    /// having trouble with type inference for the thread safety parameter.
    ///
    /// The attachment will use the [`handlers::Display`] handler to format the
    /// attachment.
    #[must_use]
    pub fn new_sendsync(attachment: A) -> Self
    where
        A: core::fmt::Display + core::fmt::Debug,
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
    #[must_use]
    pub fn new_sendsync_custom<H>(attachment: A) -> Self
    where
        H: AttachmentHandler<A>,
    {
        Self::new_custom::<H>(attachment)
    }
}

impl<A: Sized> ReportAttachment<A, Local> {
    /// Creates a new [`ReportAttachment`] with [`Local`] thread safety.
    ///
    /// This is a convenience method that calls [`ReportAttachment::new`] with
    /// explicit [`Local`] thread safety. Use this method when you're having
    /// trouble with type inference for the thread safety parameter.
    ///
    /// The attachment will use the [`handlers::Display`] handler to format the
    /// attachment.
    #[must_use]
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
    #[must_use]
    pub fn new_local_custom<H>(attachment: A) -> Self
    where
        H: AttachmentHandler<A>,
    {
        Self::new_custom::<H>(attachment)
    }
}

impl<T> ReportAttachment<Dynamic, T> {
    /// Attempts to downcast the inner attachment to a specific type.
    ///
    /// Returns `Some(&A)` if the inner attachment is of type `A`, otherwise
    /// returns `None`.
    #[must_use]
    pub fn downcast_inner<A>(&self) -> Option<&A>
    where
        A: Sized + 'static,
    {
        self.as_ref().downcast_inner()
    }

    /// Downcasts the inner attachment to a specific type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The inner attachment is actually of type `A` (can be verified by
    ///    calling [`inner_type_id()`] first)
    ///
    /// [`inner_type_id()`]: ReportAttachment::inner_type_id
    #[must_use]
    pub unsafe fn downcast_inner_unchecked<A>(&self) -> &A
    where
        A: Sized + 'static,
    {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by the caller
        unsafe { raw.attachment_downcast_unchecked() }
    }

    /// Attempts to downcast the [`ReportAttachment`] to a specific attachment
    /// type.
    ///
    /// Returns `Ok(attachment)` if the inner attachment is of type `A`,
    /// otherwise returns `Err(self)` with the original [`ReportAttachment`].
    pub fn downcast_attachment<A>(self) -> Result<ReportAttachment<A, T>, Self>
    where
        A: Sized + 'static,
    {
        if TypeId::of::<A>() == self.inner_type_id() {
            // SAFETY:
            // 1. We just checked that the type IDs match
            let attachment = unsafe { self.downcast_unchecked() };

            Ok(attachment)
        } else {
            Err(self)
        }
    }

    /// Downcasts the [`ReportAttachment`] to a specific attachment type without
    /// checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The inner attachment is actually of type `A` (can be verified by
    ///    calling [`inner_type_id()`] first)
    ///
    /// [`inner_type_id()`]: ReportAttachment::inner_type_id
    #[must_use]
    pub unsafe fn downcast_unchecked<A>(self) -> ReportAttachment<A, T>
    where
        A: Sized + 'static,
    {
        let raw = self.into_raw();

        // SAFETY:
        // 1. `A` is bounded by `Sized`, so this is trivially true.
        // 2. Guaranteed by the invariants of this type.
        // 3. Guaranteed by the caller
        // 4. Guaranteed by the invariants of this type.
        unsafe { ReportAttachment::<A, T>::from_raw(raw) }
    }
}

impl<A: Sized, T> From<A> for ReportAttachment<A, T>
where
    A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
{
    fn from(attachment: A) -> Self {
        ReportAttachment::new_custom::<handlers::Display>(attachment)
    }
}

impl<A: Sized, T> From<A> for ReportAttachment<Dynamic, T>
where
    A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
{
    fn from(attachment: A) -> Self {
        ReportAttachment::new_custom::<handlers::Display>(attachment).into_dynamic()
    }
}

// SAFETY: The `SendSync` marker indicates that the inner attachment
// is `Send`+`Sync`. Therefore it is safe to implement `Send`+`Sync` for the
// attachment itself.
unsafe impl<A: ?Sized> Send for ReportAttachment<A, SendSync> {}

// SAFETY: The `SendSync` marker indicates that the inner attachment
// is `Send`+`Sync`. Therefore it is safe to implement `Send`+`Sync` for the
// attachment itself.
unsafe impl<A: ?Sized> Sync for ReportAttachment<A, SendSync> {}

impl<A: ?Sized, T> Unpin for ReportAttachment<A, T> {}

macro_rules! from_impls {
    ($(
        <
            $($param:ident),*
        >:
        $attachment1:ty => $attachment2:ty,
        $thread_safety1:ty => $thread_safety2:ty,
        [$($op:ident),*]
    ),* $(,)?) => {
        $(
            impl<$($param),*> From<ReportAttachment<$attachment1, $thread_safety1>> for ReportAttachment<$attachment2, $thread_safety2>
            {
                fn from(attachment: ReportAttachment<$attachment1, $thread_safety1>) -> Self {
                    attachment
                        $(
                            .$op()
                        )*
                }
            }
        )*
    };
}

from_impls!(
    <A>: A => A, SendSync => Local, [into_local],
    <A>: A => Dynamic, SendSync => SendSync, [into_dynamic],
    <A>: A => Dynamic, SendSync => Local, [into_dynamic, into_local],
    <A>: A => Dynamic, Local => Local, [into_dynamic],
    <>:  Dynamic => Dynamic, SendSync => Local, [into_local],
);

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
        static_assertions::assert_impl_all!(ReportAttachment<Dynamic, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportAttachment<(), Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachment<String, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachment<NonSend, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportAttachment<Dynamic, Local>: Send, Sync);
    }

    #[test]
    fn test_attachment_unpin() {
        static_assertions::assert_impl_all!(ReportAttachment<(), SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachment<String, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachment<NonSend, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachment<Dynamic, SendSync>: Unpin);

        static_assertions::assert_impl_all!(ReportAttachment<(), Local>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachment<String, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachment<NonSend, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportAttachment<Dynamic, Local>: Unpin);
    }

    #[test]
    fn test_attachment_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportAttachment<(), SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<String, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<NonSend, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<Dynamic, SendSync>: Copy, Clone);

        static_assertions::assert_not_impl_any!(ReportAttachment<(), Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<String, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<NonSend, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportAttachment<Dynamic, Local>: Copy, Clone);
    }
}
