//! Vtable for type-erased attachment operations.
//!
//! This module contains the [`AttachmentVtable`] which enables calling handler
//! methods on attachments when their concrete attachment type `A` and handler
//! type `H` have been erased. The vtable stores function pointers that dispatch
//! to the correct typed implementations.
//!
//! This module encapsulates the fields of the [`AttachmentVtable`] so that they
//! cannot be accessed directly without going through the proper methods which
//! specifies which safety invariants are required to call them safely.

use alloc::boxed::Box;
use core::{any::TypeId, ptr::NonNull};

use crate::{
    attachment::{data::AttachmentData, raw::RawAttachmentRef},
    handlers::{AttachmentFormattingStyle, AttachmentHandler, FormattingFunction},
    util::Erased,
};

/// Vtable for type-erased attachment operations.
///
/// Contains function pointers for performing operations on attachments without
/// knowing their concrete type at compile time.
pub(super) struct AttachmentVtable {
    /// Gets the [`TypeId`] of the attachment type that was used to create this
    /// [`AttachmentVtable`].
    type_id: fn() -> TypeId,
    /// Gets the [`TypeId`] of the handler that was used to create this
    /// [`AttachmentVtable`].
    handler_type_id: fn() -> TypeId,
    /// Drops the [`Box<AttachmentData<A>>`] instance pointed to by this
    /// pointer.
    drop: unsafe fn(NonNull<AttachmentData<Erased>>),
    /// Formats the report using the `display` method on the handler.
    display: unsafe fn(RawAttachmentRef<'_>, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    /// Formats the report using the `debug` method on the handler.
    debug: unsafe fn(RawAttachmentRef<'_>, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    /// Get the formatting style preferred by the context when formatted as part
    /// of a report.
    preferred_formatting_style:
        unsafe fn(RawAttachmentRef<'_>, FormattingFunction) -> AttachmentFormattingStyle,
}

impl AttachmentVtable {
    /// Creates a new [`AttachmentVtable`] for the attachment type `A` and the
    /// handler type `H`.
    pub(super) const fn new<A: 'static, H: AttachmentHandler<A>>() -> &'static Self {
        const {
            &Self {
                type_id: TypeId::of::<A>,
                handler_type_id: TypeId::of::<H>,
                drop: drop::<A>,
                display: display::<A, H>,
                debug: debug::<A, H>,
                preferred_formatting_style: preferred_formatting_style::<A, H>,
            }
        }
    }

    /// Gets the [`TypeId`] of the attachment type that was used to create this
    /// [`AttachmentVtable`].
    pub(super) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Gets the [`TypeId`] of the handler that was used to create this
    /// [`AttachmentVtable`].
    pub(super) fn handler_type_id(&self) -> TypeId {
        (self.handler_type_id)()
    }

    /// Drops the `Box<AttachmentData<A>>` instance pointed to by this pointer.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the pointer comes from a
    ///   [`Box<AttachmentData<A>>`], which was turned into a pointer using
    ///   [`Box::into_raw`].
    /// - The attachment type `A` stored in the [`AttachmentData`] must match
    ///   the `A` used when creating this [`AttachmentVtable`].
    /// - After calling this method, the pointer must no longer be used.
    pub(super) unsafe fn drop(&self, ptr: NonNull<AttachmentData<Erased>>) {
        // SAFETY: We know that `self.drop` points to the function `drop::<A>` below.
        // That function has three requirements, all of which are guaranteed by our
        // caller:
        // - The pointer must come from `Box::into_raw`
        // - The attachment type `A` must match the stored type
        // - The pointer must not be used after calling
        unsafe {
            (self.drop)(ptr);
        }
    }

    /// Formats the attachment using the [`H::display`] function
    /// used when creating this [`AttachmentVtable`].
    ///
    /// [`H::display`]: AttachmentHandler::display
    ///
    /// # Safety
    ///
    /// The attachment type `A` used when creating this [`AttachmentVtable`]
    /// must match the type stored in the [`RawAttachmentRef`].
    pub(super) unsafe fn display(
        &self,
        ptr: RawAttachmentRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that the `self.display` field points to the function
        // `display::<A, H>` below. That function requires that the attachment
        // type `A` matches the actual attachment type stored in the `AttachmentData`,
        // which is guaranteed by our caller.
        unsafe { (self.display)(ptr, formatter) }
    }

    /// Formats the attachment using the [`H::debug`] function
    /// used when creating this [`AttachmentVtable`].
    ///
    /// [`H::debug`]: AttachmentHandler::debug
    ///
    /// # Safety
    ///
    /// The attachment type `A` used when creating this [`AttachmentVtable`]
    /// must match the type stored in the [`RawAttachmentRef`].
    pub(super) unsafe fn debug(
        &self,
        ptr: RawAttachmentRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that the `self.debug` field points to the function
        // `debug::<A, H>` below. That function requires that the attachment
        // type `A` matches the actual attachment type stored in the `AttachmentData`,
        // which is guaranteed by our caller.
        unsafe { (self.debug)(ptr, formatter) }
    }

    /// Gets the preferred formatting style using the
    /// [`H::preferred_formatting_style`] function used when creating this
    /// [`AttachmentVtable`].
    ///
    /// [`H::preferred_formatting_style`]: AttachmentHandler::preferred_formatting_style
    ///
    /// # Safety
    ///
    /// The attachment type `A` used when creating this [`AttachmentVtable`]
    /// must match the type stored in the [`RawAttachmentRef`].
    pub(super) unsafe fn preferred_formatting_style(
        &self,
        ptr: RawAttachmentRef<'_>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        // SAFETY: We know that the `self.preferred_formatting_style` field points to
        // the function `preferred_formatting_style::<A, H>` below.
        // That function requires that the attachment type `A` matches the actual
        // attachment type stored in the `AttachmentData`, which is guaranteed
        // by our caller.
        unsafe { (self.preferred_formatting_style)(ptr, report_formatting_function) }
    }
}

/// Drops the [`Box<AttachmentData<A>>`] instance pointed to by this pointer.
///
/// # Safety
///
/// - The caller must ensure that the pointer comes from a
///   [`Box<AttachmentData<A>>`], which was turned into a pointer using
///   [`Box::into_raw`].
/// - The attachment type `A` must match the actual attachment type stored in
///   the [`AttachmentData`].
/// - After calling this method, the pointer must no longer be used.
unsafe fn drop<A: 'static>(ptr: NonNull<AttachmentData<Erased>>) {
    let ptr: NonNull<AttachmentData<A>> = ptr.cast();
    let ptr = ptr.as_ptr();
    // SAFETY: Our pointer has the correct type as guaranteed by the caller, and it
    // came from a call to [`Box::into_raw`] as also guaranteed by our caller.
    let boxed = unsafe { Box::from_raw(ptr) };
    core::mem::drop(boxed);
}

/// Formats an attachment using its handler's display implementation.
///
/// # Safety
///
/// The caller must ensure that the type `A` matches the actual attachment type
/// stored in the [`AttachmentData`].
unsafe fn display<A: 'static, H: AttachmentHandler<A>>(
    ptr: RawAttachmentRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY: Our caller guarantees that the type `A` matches the actual attachment
    // type stored in the `AttachmentData`.
    let context: &A = unsafe { ptr.attachment_downcast_unchecked::<A>() };
    H::display(context, formatter)
}

/// Formats an attachment using its handler's debug implementation.
///
/// # Safety
///
/// The caller must ensure that the type `A` matches the actual attachment type
/// stored in the [`AttachmentData`].
unsafe fn debug<A: 'static, H: AttachmentHandler<A>>(
    ptr: RawAttachmentRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY: Our caller guarantees that the type `A` matches the actual attachment
    // type stored in the `AttachmentData`.
    let context: &A = unsafe { ptr.attachment_downcast_unchecked::<A>() };
    H::debug(context, formatter)
}

/// Gets the preferred formatting style using the
/// [`H::preferred_formatting_style`] function.
///
/// [`H::preferred_formatting_style`]: AttachmentHandler::preferred_formatting_style
///
/// # Safety
///
/// The caller must ensure that the type `A` matches the actual attachment type
/// stored in the [`AttachmentData`].
unsafe fn preferred_formatting_style<A: 'static, H: AttachmentHandler<A>>(
    ptr: RawAttachmentRef<'_>,
    report_formatting_function: FormattingFunction,
) -> AttachmentFormattingStyle {
    // SAFETY: Our caller guarantees that the type `A` matches the actual attachment
    // type stored in the `AttachmentData`.
    let context: &A = unsafe { ptr.attachment_downcast_unchecked::<A>() };
    H::preferred_formatting_style(context, report_formatting_function)
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_attachment_vtable_eq() {
        // Test that vtables have proper static lifetime and can be safely shared
        let vtable1 = AttachmentVtable::new::<i32, HandlerI32>();
        let vtable2 = AttachmentVtable::new::<i32, HandlerI32>();

        // Both should be the exact same static instance
        assert!(core::ptr::eq(vtable1, vtable2));
    }

    #[test]
    fn test_attachment_type_id() {
        let vtable = AttachmentVtable::new::<i32, HandlerI32>();
        assert_eq!(vtable.type_id(), TypeId::of::<i32>());
    }
}
