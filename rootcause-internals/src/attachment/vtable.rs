//! Vtable for type-erased attachment operations.
//!
//! This module contains the [`AttachmentVtable`] which enables calling handler
//! methods on attachments when their concrete attachment type `A` and handler
//! type `H` have been erased. The vtable stores function pointers that dispatch
//! to the correct typed implementations.
//!
//! This module encapsulates the fields of [`AttachmentVtable`] so they cannot
//! be accessed directly. This visibility restriction guarantees the safety
//! invariant: **the vtable's type parameters must match the actual attachment
//! type and handler stored in the [`AttachmentData`]**.
//!
//! # Safety Invariant
//!
//! This invariant is maintained because vtables are created as `&'static`
//! references via [`AttachmentVtable::new`], which pairs the function pointers
//! with specific types `A` and `H` at compile time.

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
///
/// # Safety Invariant
///
/// The fields `drop`, `display`, `debug`, and `preferred_formatting_style` are
/// guaranteed to point to the functions defined below instantiated with the
/// attachment type `A` and handler type `H` that were used to create this
/// [`AttachmentVtable`].
pub(crate) struct AttachmentVtable {
    /// Gets the [`TypeId`] of the attachment type that was used to create this
    /// [`AttachmentVtable`].
    type_id: fn() -> TypeId,
    /// Gets the [`TypeId`] of the handler that was used to create this
    /// [`AttachmentVtable`].
    handler_type_id: fn() -> TypeId,
    /// Drops the [`Box<AttachmentData<A>>`] instance pointed to by this
    /// pointer.
    drop: unsafe fn(NonNull<AttachmentData<Erased>>),
    /// Formats the attachment using the `display` method on the handler.
    display: unsafe fn(RawAttachmentRef<'_>, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    /// Formats the attachment using the `debug` method on the handler.
    debug: unsafe fn(RawAttachmentRef<'_>, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    /// Get the formatting style preferred by the attachment when formatted as
    /// part of a report.
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
    #[inline]
    pub(super) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Gets the [`TypeId`] of the handler that was used to create this
    /// [`AttachmentVtable`].
    #[inline]
    pub(super) fn handler_type_id(&self) -> TypeId {
        (self.handler_type_id)()
    }

    /// Drops the `Box<AttachmentData<A>>` instance pointed to by this pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The pointer comes from [`Box<AttachmentData<A>>`] via
    ///    [`Box::into_raw`]
    /// 2. This [`AttachmentVtable`] must be a vtable for the attachment type
    ///    stored in the [`AttachmentData`].
    /// 3. This method drops the [`Box<AttachmentData<A>>`], so the caller must
    ///    ensure that the pointer has not previously been dropped, that it is
    ///    able to transfer ownership of the pointer, and that it will not use
    ///    the pointer after calling this method.
    #[inline]
    pub(super) unsafe fn drop(&self, ptr: NonNull<AttachmentData<Erased>>) {
        // SAFETY: We know that `self.drop` points to the function `drop::<A>` below.
        // That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        // 2. Guaranteed by the caller
        // 3. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: drop
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
    /// The caller must ensure:
    ///
    /// 1. This [`AttachmentVtable`] must be a vtable for the attachment type
    ///    stored in the [`RawAttachmentRef`].
    #[inline]
    pub(super) unsafe fn display(
        &self,
        ptr: RawAttachmentRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that the `self.display` field points to the function
        // `display::<A, H>` below. That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: display
            (self.display)(ptr, formatter)
        }
    }

    /// Formats the attachment using the [`H::debug`] function
    /// used when creating this [`AttachmentVtable`].
    ///
    /// [`H::debug`]: AttachmentHandler::debug
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. This [`AttachmentVtable`] must be a vtable for the attachment type
    ///    stored in the [`RawAttachmentRef`].
    #[inline]
    pub(super) unsafe fn debug(
        &self,
        ptr: RawAttachmentRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that the `self.debug` field points to the function
        // `debug::<A, H>` below. That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: debug
            (self.debug)(ptr, formatter)
        }
    }

    /// Gets the preferred formatting style using the
    /// [`H::preferred_formatting_style`] function used when creating this
    /// [`AttachmentVtable`].
    ///
    /// [`H::preferred_formatting_style`]: AttachmentHandler::preferred_formatting_style
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. This [`AttachmentVtable`] must be a vtable for the attachment type
    ///    stored in the [`RawAttachmentRef`].
    #[inline]
    pub(super) unsafe fn preferred_formatting_style(
        &self,
        ptr: RawAttachmentRef<'_>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        // SAFETY: We know that the `self.preferred_formatting_style` field points to
        // the function `preferred_formatting_style::<A, H>` below. That
        // function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: preferred_formatting_style
            (self.preferred_formatting_style)(ptr, report_formatting_function)
        }
    }
}

/// Drops the [`Box<AttachmentData<A>>`] instance pointed to by this pointer.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The pointer comes from [`Box<AttachmentData<A>>`] via [`Box::into_raw`]
/// 2. The attachment type `A` matches the actual attachment type stored in the
///    [`AttachmentData`]
/// 3. This method drops the [`Box<AttachmentData<A>>`], so the caller must
///    ensure that the pointer has not previously been dropped, that it is able
///    to transfer ownership of the pointer, and that it will not use the
///    pointer after calling this method.
unsafe fn drop<A: 'static>(ptr: NonNull<AttachmentData<Erased>>) {
    let ptr: NonNull<AttachmentData<A>> = ptr.cast();
    let ptr = ptr.as_ptr();
    // SAFETY: Our pointer has the correct type as guaranteed by the caller, and it
    // came from a call to `Box::into_raw` as also guaranteed by our caller.
    let boxed = unsafe {
        // @add-unsafe-context: AttachmentData
        Box::from_raw(ptr)
    };
    core::mem::drop(boxed);
}

/// Formats an attachment using its handler's display implementation.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The type `A` matches the actual attachment type stored in the
///    [`AttachmentData`]
unsafe fn display<A: 'static, H: AttachmentHandler<A>>(
    ptr: RawAttachmentRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY:
    // 1. Guaranteed by the caller
    let attachment: &A = unsafe { ptr.attachment_downcast_unchecked::<A>() };
    H::display(attachment, formatter)
}

/// Formats an attachment using its handler's debug implementation.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The type `A` matches the actual attachment type stored in the
///    [`AttachmentData`]
unsafe fn debug<A: 'static, H: AttachmentHandler<A>>(
    ptr: RawAttachmentRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY:
    // 1. Guaranteed by the caller
    let attachment: &A = unsafe { ptr.attachment_downcast_unchecked::<A>() };
    H::debug(attachment, formatter)
}

/// Gets the preferred formatting style using the
/// [`H::preferred_formatting_style`] function.
///
/// [`H::preferred_formatting_style`]: AttachmentHandler::preferred_formatting_style
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The type `A` matches the actual attachment type stored in the
///    [`AttachmentData`]
unsafe fn preferred_formatting_style<A: 'static, H: AttachmentHandler<A>>(
    ptr: RawAttachmentRef<'_>,
    report_formatting_function: FormattingFunction,
) -> AttachmentFormattingStyle {
    // SAFETY:
    // 1. Guaranteed by the caller
    let attachment: &A = unsafe { ptr.attachment_downcast_unchecked::<A>() };
    H::preferred_formatting_style(attachment, report_formatting_function)
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
