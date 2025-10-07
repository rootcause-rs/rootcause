//! This module encapsulates the fields of the [`RawAttachment`] and [`RawAttachmentRef`].
//! Since this is the only place they are visible, this means that the `ptr` field of both types is always
//! guaranteed to come from [`Box<AttachmentData<A>>`]. This follows from the fact that there are no places where the
//! `ptr` field is altered after creation (besides invalidating it after it should no longer be used).

use alloc::boxed::Box;
use core::{any::TypeId, marker::PhantomData, ptr::NonNull};

use crate::{
    attachment::data::AttachmentData,
    handlers::{AttachmentFormattingStyle, AttachmentHandler, FormattingFunction},
    util::{CastTo, Erased},
};

/// A pointer to an [`AttachmentData`] that is guaranteed to point to an initialized instance
/// of an [`AttachmentData<A>`] for some specific `A`, though we do not know which actual `A` it is.
///
/// However, the pointer is allowed to transition into a non-initialized state inside the
/// [`RawAttachment::drop`] method.
///
/// The pointer is guaranteed to have been created using [`Box::into_raw`].
///
/// We cannot use a [`Box<AttachmentData<A>>`] directly, because that does not allow
/// us to type-erase the `A`.
#[repr(transparent)]
pub struct RawAttachment {
    ptr: NonNull<AttachmentData<Erased>>,
    _marker: core::marker::PhantomData<AttachmentData<Erased>>,
}

impl RawAttachment {
    /// Creates a new [`RawAttachment`] with the specified handler and attachment.
    pub fn new<A, H>(attachment: A) -> Self
    where
        A: 'static,
        H: AttachmentHandler<A>,
    {
        let ptr = Box::new(AttachmentData::new::<H>(attachment));
        let ptr: *const AttachmentData<A> = Box::into_raw(ptr);
        let ptr: *mut AttachmentData<Erased> = ptr as _;

        // SAFETY: `Box::into_raw` returns a non-null pointer
        let ptr: NonNull<AttachmentData<Erased>> = unsafe { NonNull::new_unchecked(ptr) };

        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to the [`AttachmentData`] instance.
    pub fn as_ref<'a>(&'a self) -> RawAttachmentRef<'a> {
        RawAttachmentRef {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }
}

impl core::ops::Drop for RawAttachment {
    fn drop(&mut self) {
        let vtable = self.as_ref().vtable();

        // SAFETY: The vtable drop method has three safety requirements:
        // - The pointer must come from `Box<AttachmentData<A>>` via `Box::into_raw`
        // - The `A` type in `AttachmentData<A>` must match the vtable's `A` type
        // - The pointer must not be used after this call
        //
        // These are satisfied because:
        // - The only way to construct or alter a `RawAttachment` is through the `RawAttachment::new` method
        // - The only way to construct or alter an `AttachmentData` is through the `AttachmentData::new` method
        // - This is guaranteed by the fact that we are in the `drop()` function
        unsafe {
            vtable.drop(self.ptr);
        }
    }
}

/// A lifetime-bound pointer to an [`AttachmentData`] that is guaranteed to point
/// to an initialized instance of an [`AttachmentData<A>`] for some specific `A`, though
/// we do not know which actual `A` it is.
///
/// We cannot use a [`&'a AttachmentData<A>`] directly, because that would require
/// us to know the actual type of the attachment, which we do not.
///
/// [`&'a AttachmentData<A>`]: AttachmentData
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RawAttachmentRef<'a> {
    ptr: NonNull<AttachmentData<Erased>>,
    _marker: core::marker::PhantomData<&'a AttachmentData<Erased>>,
}

impl<'a> RawAttachmentRef<'a> {
    /// Casts the [`RawAttachmentRef`] to an [`AttachmentData<A>`] reference.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type `A` matches the actual attachment type stored in the [`AttachmentData`].
    pub(super) unsafe fn cast_inner<A: CastTo>(self) -> &'a AttachmentData<A::Target> {
        // Debug assertion to catch type mismatches in case of bugs
        debug_assert_eq!(self.vtable().type_id(), TypeId::of::<A>());

        let this = self.ptr.cast::<AttachmentData<A::Target>>();
        // SAFETY: Our caller guarantees that we point to an AttachmentData<A>, so it is safe to turn
        // the NonNull pointer into a reference with the same lifetime
        unsafe { this.as_ref() }
    }

    /// Returns a [`NonNull`] pointer to the [`AttachmentData`] instance.
    pub(super) fn as_ptr(self) -> *const AttachmentData<Erased> {
        self.ptr.as_ptr()
    }

    /// Returns the [`TypeId`] of the attachment.
    pub fn attachment_type_id(self) -> TypeId {
        self.vtable().type_id()
    }

    /// Returns the [`TypeId`] of the attachment.
    pub fn attachment_handler_type_id(self) -> TypeId {
        self.vtable().handler_type_id()
    }

    /// Checks if the type of the attachment matches the specified type and returns a reference to it if it does.
    pub fn attachment_downcast<A: 'static>(self) -> Option<&'a A> {
        if self.attachment_type_id() == core::any::TypeId::of::<A>() {
            // SAFETY: We must ensure that the `A` in the AttachmentData matches the `A` we are using as an argument.
            // However, we have just checked that the types match, so that is fine.
            unsafe { Some(self.attachment_downcast_unchecked::<A>()) }
        } else {
            None
        }
    }

    /// Formats the attachment by using the [`AttachmentHandler::display`] method
    /// specified by the handler used to create the [`AttachmentData`].
    pub fn attachment_display(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `A` of the `AttachmentData` matches the `A` of the `AttachmentVtable`.
        // However, the only way to construct an `AttachmentData` is through the `AttachmentData::new` method,
        // which ensures this fact.
        unsafe { vtable.display(self, formatter) }
    }

    /// Formats the attachment by using the [`AttachmentHandler::debug`] method
    /// specified by the handler used to create the [`AttachmentData`].
    pub fn attachment_debug(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `A` of the `AttachmentData` matches the `A` of the `AttachmentVtable`.
        // However, the only way to construct an `AttachmentData` is through the `AttachmentData::new` method,
        // which ensures this fact.
        unsafe { vtable.debug(self, formatter) }
    }

    /// The formatting style preferred by the attachment when formatted as part of a
    /// report.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this attachment will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    /// - `report_formatting_alternate`: Whether the report in which this attachment will be embedded is being formatted using the [`alternate`] mode
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`alternate`]: core::fmt::Formatter::alternate
    pub fn preferred_formatting_style(
        self,
        report_formatting_function: FormattingFunction,
        report_formatting_alternate: bool,
    ) -> AttachmentFormattingStyle {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `A` of the `AttachmentData` matches the `A` of the `AttachmentVtable`.
        // However, the only way to construct an `AttachmentData` is through the `AttachmentData::new` method,
        // which ensures this fact.
        unsafe {
            vtable.preferred_formatting_style(
                self,
                report_formatting_function,
                report_formatting_alternate,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;
    use crate::handlers::AttachmentHandler;

    struct HandlerI32;
    impl AttachmentHandler<i32> for HandlerI32 {
        fn display(value: &i32, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            core::fmt::Display::fmt(value, formatter)
        }

        fn debug(value: &i32, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            core::fmt::Debug::fmt(value, formatter)
        }
    }

    struct HandlerString;
    impl AttachmentHandler<String> for HandlerString {
        fn display(value: &String, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            core::fmt::Display::fmt(value, formatter)
        }
        fn debug(value: &String, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            core::fmt::Debug::fmt(value, formatter)
        }
    }

    #[test]
    fn test_raw_attachment_size() {
        assert_eq!(
            core::mem::size_of::<RawAttachment>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<RawAttachment>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<(), RawAttachment>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<String, RawAttachment>>(),
            core::mem::size_of::<String>()
        );
        assert_eq!(
            core::mem::size_of::<Option<Option<RawAttachment>>>(),
            core::mem::size_of::<Option<usize>>()
        );

        assert_eq!(
            core::mem::size_of::<RawAttachmentRef<'_>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<RawAttachmentRef<'_>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<(), RawAttachmentRef<'_>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<String, RawAttachmentRef<'_>>>(),
            core::mem::size_of::<String>()
        );
        assert_eq!(
            core::mem::size_of::<Option<Option<RawAttachmentRef<'_>>>>(),
            core::mem::size_of::<Option<usize>>()
        );
    }

    #[test]
    fn test_raw_attachment_get_refs() {
        let attachment = RawAttachment::new::<i32, HandlerI32>(100);
        let attachment_ref = attachment.as_ref();

        // Accessing the pointer multiple times should be safe and consistent
        let ptr1 = attachment_ref.as_ptr();
        let ptr2 = attachment_ref.as_ptr();
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_raw_attachment_downcast() {
        let int_attachment = RawAttachment::new::<i32, HandlerI32>(42);
        let string_attachment = RawAttachment::new::<String, HandlerString>(String::from("test"));

        let int_ref = int_attachment.as_ref();
        let string_ref = string_attachment.as_ref();

        // Are TypeIds what we expect?
        assert_eq!(int_ref.attachment_type_id(), TypeId::of::<i32>());
        assert_eq!(string_ref.attachment_type_id(), TypeId::of::<String>());

        // The vtables should be different
        assert!(!core::ptr::eq(int_ref.vtable(), string_ref.vtable()));

        // Cross-type downcasting should fail safely
        assert!(int_ref.attachment_downcast::<String>().is_none());
        assert!(string_ref.attachment_downcast::<i32>().is_none());

        // Correct downcasting should work
        assert_eq!(int_ref.attachment_downcast::<i32>().unwrap(), &42);
        assert_eq!(string_ref.attachment_downcast::<String>().unwrap(), "test");
    }

    #[test]
    fn test_raw_attachment_display_debug() {
        use alloc::format;

        let int_attachment = RawAttachment::new::<i32, HandlerI32>(42);
        let string_attachment = RawAttachment::new::<String, HandlerString>(String::from("test"));

        let int_ref = int_attachment.as_ref();
        let string_ref = string_attachment.as_ref();

        // Test display formatting
        let display_int = format!(
            "{}",
            TestDisplayFormatter::new(|f| int_ref.attachment_display(f))
        );
        let display_string = format!(
            "{}",
            TestDisplayFormatter::new(|f| string_ref.attachment_display(f))
        );

        assert_eq!(display_int, "42");
        assert_eq!(display_string, "test");

        // Test debug formatting
        let debug_int = format!(
            "{}",
            TestDisplayFormatter::new(|f| int_ref.attachment_debug(f))
        );
        let debug_string = format!(
            "{}",
            TestDisplayFormatter::new(|f| string_ref.attachment_debug(f))
        );

        assert_eq!(debug_int, "42");
        assert_eq!(debug_string, "\"test\"");
    }

    // Helper struct for testing display/debug formatting
    struct TestDisplayFormatter<F> {
        formatter_fn: F,
    }

    impl<F> TestDisplayFormatter<F>
    where
        F: Fn(&mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    {
        fn new(formatter_fn: F) -> Self {
            Self { formatter_fn }
        }
    }

    impl<F> core::fmt::Display for TestDisplayFormatter<F>
    where
        F: Fn(&mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            (self.formatter_fn)(f)
        }
    }
}
