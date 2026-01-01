//! Type-erased attachment pointer types.
//!
//! This module encapsulates the `ptr` field of [`RawAttachment`] and
//! [`RawAttachmentRef`], ensuring it is only visible within this module. This
//! visibility restriction guarantees the safety invariant: **the pointer always
//! comes from `Box<AttachmentData<A>>`**.
//!
//! # Safety Invariant
//!
//! Since the `ptr` field can only be set via [`RawAttachment::new`] (which
//! creates it from `Box::into_raw`), and cannot be modified afterward (no `pub`
//! or `pub(crate)` fields), the pointer provenance remains valid throughout the
//! value's lifetime.
//!
//! The [`RawAttachment::drop`] implementation relies on this invariant to
//! safely reconstruct the `Box` and deallocate the memory.
//!
//! # Type Erasure
//!
//! The concrete type parameter `A` is erased by casting to
//! `AttachmentData<Erased>`. The vtable stored within the `AttachmentData`
//! provides the runtime type information needed to safely downcast and format
//! attachments.

use alloc::boxed::Box;
use core::{any::TypeId, ptr::NonNull};

use crate::{
    attachment::data::AttachmentData,
    handlers::{AttachmentFormattingStyle, AttachmentHandler, FormattingFunction},
    util::Erased,
};

/// A pointer to an [`AttachmentData`] that is guaranteed to point to an
/// initialized instance of an [`AttachmentData<A>`] for some specific `A`,
/// though we do not know which actual `A` it is.
///
/// However, the pointer is allowed to transition into a non-initialized state
/// inside the [`RawAttachment::drop`] method.
///
/// The pointer is guaranteed to have been created using [`Box::into_raw`].
///
/// We cannot use a [`Box<AttachmentData<A>>`] directly, because that does not
/// allow us to type-erase the `A`.
#[repr(transparent)]
pub struct RawAttachment {
    /// Pointer to the inner attachment data
    ///
    /// # Safety
    ///
    /// The following safety invariants are guaranteed to be upheld as long as
    /// this struct exists:
    ///
    /// 1. The pointer must have been created from a `Box<AttachmentData<A>>`
    ///    for some `A` using `Box::into_raw`.
    /// 2. The pointer will point to the same `AttachmentData<A>` for the entire
    ///    lifetime of this object.
    /// 3. The pointee is properly initialized for the entire lifetime of this
    ///    object, except during the execution of the `Drop` implementation.
    ptr: NonNull<AttachmentData<Erased>>,
}

impl RawAttachment {
    /// Creates a new [`RawAttachment`] with the specified handler and
    /// attachment.
    ///
    /// The returned attachment will embed the specified attachment and use the
    /// specified handler for all operations.
    #[inline]
    pub fn new<A, H>(attachment: A) -> Self
    where
        A: 'static,
        H: AttachmentHandler<A>,
    {
        let ptr = Box::new(AttachmentData::new::<H>(attachment));
        let ptr: *mut AttachmentData<A> = Box::into_raw(ptr);
        let ptr: *mut AttachmentData<Erased> = ptr.cast::<AttachmentData<Erased>>();

        // SAFETY: `Box::into_raw` returns a non-null pointer
        let ptr: NonNull<AttachmentData<Erased>> = unsafe {
            // @add-unsafe-context: Erased
            NonNull::new_unchecked(ptr)
        };

        Self { ptr }
    }

    /// Returns a reference to the [`AttachmentData`] instance.
    #[inline]
    pub fn as_ref(&self) -> RawAttachmentRef<'_> {
        RawAttachmentRef {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }
}

impl core::ops::Drop for RawAttachment {
    #[inline]
    fn drop(&mut self) {
        let vtable = self.as_ref().vtable();

        // SAFETY:
        // 1. The pointer comes from `Box::into_raw` (guaranteed by
        //    `RawAttachment::new`)
        // 2. The vtable returned by `self.as_ref().vtable()` is guaranteed to match the
        //    data in the `AttachmentData`.
        // 3. The pointer is initialized and has not been previously free as guaranteed
        //    by the invariants on this type. We are correctly transferring ownership
        //    here and the pointer is not used afterwards, as we are in the drop
        //    function.
        unsafe {
            // @add-unsafe-context: AttachmentData
            vtable.drop(self.ptr);
        }
    }
}

/// A lifetime-bound pointer to an [`AttachmentData`] that is guaranteed to
/// point to an initialized instance of an [`AttachmentData<A>`] for some
/// specific `A`, though we do not know which actual `A` it is.
///
/// We cannot use a [`&'a AttachmentData<A>`] directly, because that would
/// require us to know the actual type of the attachment, which we do not.
///
/// [`&'a AttachmentData<A>`]: AttachmentData
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RawAttachmentRef<'a> {
    /// Pointer to the inner attachment data
    ///
    /// # Safety
    ///
    /// The following safety invariants are guaranteed to be upheld as long as
    /// this struct exists:
    ///
    /// 1. The pointer must have been created from a `Box<AttachmentData<A>>`
    ///    for some `A` using `Box::into_raw`.
    /// 2. The pointer will point to the same `AttachmentData<A>` for the entire
    ///    lifetime of this object.
    ptr: NonNull<AttachmentData<Erased>>,

    /// Marker to tell the compiler that we should
    /// behave the same as a `&'a AttachmentData<Erased>`
    _marker: core::marker::PhantomData<&'a AttachmentData<Erased>>,
}

impl<'a> RawAttachmentRef<'a> {
    /// Casts the [`RawAttachmentRef`] to an [`AttachmentData<A>`] reference.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `A` matches the actual attachment type stored in the
    ///    [`AttachmentData`].
    #[inline]
    pub(super) unsafe fn cast_inner<A>(self) -> &'a AttachmentData<A> {
        // Debug assertion to catch type mismatches in case of bugs
        debug_assert_eq!(self.vtable().type_id(), TypeId::of::<A>());

        let this = self.ptr.cast::<AttachmentData<A>>();
        // SAFETY: Converting the NonNull pointer to a reference is sound because:
        // - The pointer is non-null, properly aligned, and dereferenceable (guaranteed
        //   by RawAttachmentRef's type invariants)
        // - The pointee is properly initialized (RawAttachmentRef's doc comment
        //   guarantees it points to an initialized AttachmentData<A> for some A)
        // - The type `A` matches the actual attachment type (guaranteed by caller)
        // - Shared access is allowed
        // - The reference lifetime 'a is valid (tied to RawAttachmentRef<'a>'s
        //   lifetime)
        unsafe { this.as_ref() }
    }

    /// Returns a [`NonNull`] pointer to the [`AttachmentData`] instance.
    #[inline]
    pub(super) fn as_ptr(self) -> *const AttachmentData<Erased> {
        self.ptr.as_ptr()
    }

    /// Returns the [`TypeId`] of the attachment.
    #[inline]
    pub fn attachment_type_id(self) -> TypeId {
        self.vtable().type_id()
    }

    /// Returns the [`core::any::type_name`] of the attachment.
    #[inline]
    pub fn attachment_type_name(self) -> &'static str {
        self.vtable().type_name()
    }

    /// Returns the [`TypeId`] of the attachment.
    #[inline]
    pub fn attachment_handler_type_id(self) -> TypeId {
        self.vtable().handler_type_id()
    }

    /// Formats the attachment by using the [`AttachmentHandler::display`]
    /// method specified by the handler used to create the
    /// [`AttachmentData`].
    #[inline]
    pub fn attachment_display(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vtable = self.vtable();
        // SAFETY:
        // 1. The vtable returned by `self.vtable()` is guaranteed to match the data in
        //    the `AttachmentData`.
        unsafe {
            // @add-unsafe-context: AttachmentData
            vtable.display(self, formatter)
        }
    }

    /// Formats the attachment by using the [`AttachmentHandler::debug`] method
    /// specified by the handler used to create the [`AttachmentData`].
    #[inline]
    pub fn attachment_debug(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vtable = self.vtable();

        // SAFETY:
        // 1. The vtable returned by `self.vtable()` is guaranteed to match the data in
        //    the `AttachmentData`.
        unsafe {
            // @add-unsafe-context: AttachmentData
            vtable.debug(self, formatter)
        }
    }

    /// The formatting style preferred by the attachment when formatted as part
    /// of a report.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this
    ///   attachment will be embedded is being formatted using [`Display`]
    ///   formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    #[inline]
    pub fn preferred_formatting_style(
        self,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let vtable = self.vtable();

        // SAFETY:
        // 1. The vtable returned by `self.vtable()` is guaranteed to match the data in
        //    the `AttachmentData`.
        unsafe {
            // @add-unsafe-context: AttachmentData
            vtable.preferred_formatting_style(self, report_formatting_function)
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

    #[test]
    fn test_send_sync() {
        static_assertions::assert_not_impl_any!(RawAttachment: Send, Sync);
        static_assertions::assert_not_impl_any!(RawAttachmentRef<'_>: Send, Sync);
    }
}
