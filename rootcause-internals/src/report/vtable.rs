//! Vtable for type-erased report operations.
//!
//! This module contains the [`ReportVtable`] which enables calling handler
//! methods on reports when their concrete context type `C` and handler type `H`
//! have been erased. The vtable stores function pointers that dispatch to the
//! correct typed implementations.
//!
//! This module encapsulates the fields of the [`ReportVtable`] so that they
//! cannot be accessed directly without going through the proper methods which
//! specifies which safety invariants are required to call them safely.

use core::{any::TypeId, ptr::NonNull};

use crate::{
    handlers::{ContextFormattingStyle, ContextHandler, FormattingFunction},
    report::{
        data::ReportData,
        raw::{RawReport, RawReportRef},
    },
    util::Erased,
};

/// Vtable for type-erased report operations.
///
/// Contains function pointers for performing operations on reports without
/// knowing their concrete type at compile time.
pub(super) struct ReportVtable {
    /// Gets the [`TypeId`] of the context type that was used to create this
    /// [`ReportVtable`].
    type_id: fn() -> TypeId,
    /// Gets the [`TypeId`] of the handler that was used to create this
    /// [`ReportVtable`].
    handler_type_id: fn() -> TypeId,
    /// Method to drop the [`triomphe::Arc<ReportData<C>>`] instance pointed to
    /// by this pointer.
    drop: unsafe fn(NonNull<ReportData<Erased>>),
    /// Clones the `triomphe::Arc<ReportData<C>>` pointed to by this pointer.
    clone_arc: unsafe fn(NonNull<ReportData<Erased>>) -> RawReport,
    /// Gets the strong count of the [`triomphe::Arc<ReportData<C>>`] pointed to
    /// by this pointer.
    strong_count: unsafe fn(NonNull<ReportData<Erased>>) -> usize,
    /// Returns a reference to the source of the error using the `source` method
    /// on the handler.
    source: unsafe fn(RawReportRef<'_>) -> Option<&(dyn core::error::Error + 'static)>,
    /// Formats the report using the `display` method on the handler.
    display: unsafe fn(RawReportRef<'_>, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    /// Formats the report using the `debug method on the handler.
    debug: unsafe fn(RawReportRef<'_>, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    /// Get the formatting style preferred by the context when formatted as part
    /// of a report.
    preferred_context_formatting_style:
        unsafe fn(RawReportRef<'_>, FormattingFunction) -> ContextFormattingStyle,
}

impl ReportVtable {
    /// Creates a new [`ReportVtable`] for the context type `C` and the handler
    /// type `H`.
    pub(super) const fn new<C: 'static, H: ContextHandler<C>>() -> &'static Self {
        const {
            &Self {
                type_id: TypeId::of::<C>,
                handler_type_id: TypeId::of::<H>,
                drop: drop::<C>,
                clone_arc: clone_arc::<C>,
                strong_count: strong_count::<C>,
                source: source::<C, H>,
                display: display::<C, H>,
                debug: debug::<C, H>,
                preferred_context_formatting_style: preferred_context_formatting_style::<C, H>,
            }
        }
    }

    /// Gets the [`TypeId`] of the context type that was used to create this
    /// [`ReportVtable`].
    #[inline]
    pub(super) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Gets the [`TypeId`] of the handler that was used to create this
    /// [`ReportVtable`].
    #[inline]
    pub(super) fn handler_type_id(&self) -> TypeId {
        (self.handler_type_id)()
    }

    /// Drops the `triomphe::Arc<ReportData<C>>` instance pointed to by this
    /// pointer.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the pointer comes from an
    ///   [`triomphe::Arc<ReportData<C>>`], which was turned into a pointer
    ///   using [`triomphe::Arc::into_raw`].
    /// - The context type `C` stored in the [`ReportData`] must match the `C`
    ///   used when creating this [`ReportVtable`].
    /// - After calling this method, the pointer must no longer be used.
    #[inline]
    pub(super) unsafe fn drop(&self, ptr: NonNull<ReportData<Erased>>) {
        // SAFETY: We know that `self.drop` points to the function `drop::<C>` below.
        // That function has three requirements, all of which are guaranteed by our
        // caller:
        // - The pointer must come from `triomphe::Arc::into_raw`
        // - The context type `C` must match the stored type
        // - The pointer must not be used after calling
        unsafe {
            (self.drop)(ptr);
        }
    }

    /// Clones the [`triomphe::Arc<ReportData<C>>`] pointed to by this pointer.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the pointer comes from an
    ///   [`triomphe::Arc<ReportData<C>>`], which was turned into a pointer
    ///   using [`triomphe::Arc::into_raw`].
    /// - The context type `C` stored in the [`ReportData`] must match the `C`
    ///   used when creating this [`ReportVtable`].
    /// - There must be no external assumptions that there is a unique ownership
    ///   of the [`triomphe::Arc`].
    #[inline]
    pub(super) unsafe fn clone_arc(&self, ptr: NonNull<ReportData<Erased>>) -> RawReport {
        // SAFETY: We know that `self.clone_arc` points to the function `clone_arc::<C>`
        // below. That function has three requirements, all of which are
        // guaranteed by our caller:
        // - The pointer must come from `triomphe::Arc::into_raw`
        // - The context type `C` must match the stored type
        // - There must be no external assumptions about pointer uniqueness
        unsafe { (self.clone_arc)(ptr) }
    }

    /// Gets the strong count of the [`triomphe::Arc<ReportData<C>>`] pointed to
    /// by this pointer.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the pointer comes from an
    ///   [`triomphe::Arc<ReportData<C>>`], which was turned into a pointer
    ///   using [`triomphe::Arc::into_raw`].
    /// - The context type `C` stored in the [`ReportData`] must match the `C`
    ///   used when creating this [`ReportVtable`].
    #[inline]
    pub(super) unsafe fn strong_count(&self, ptr: NonNull<ReportData<Erased>>) -> usize {
        // SAFETY: We know that `self.strong_count` points to the function
        // `strong_count::<C>` below. That function has three requirements, all
        // of which are guaranteed by our caller:
        // - The pointer must come from `triomphe::Arc::into_raw`
        // - The context type `C` must match the stored type
        unsafe { (self.strong_count)(ptr) }
    }

    /// Returns a reference to the source of the error using the [`H::source`]
    /// function used when creating this [`ReportVtable`].
    ///
    /// # Safety
    ///
    /// The context type `C` used when creating this [`ReportVtable`] must match
    /// the type of the `C` stored in the [`ReportData`] pointed to by the
    /// [`RawReportRef`].
    ///
    /// [`H::source`]: ContextHandler::source
    #[inline]
    pub(super) unsafe fn source<'a>(
        &self,
        ptr: RawReportRef<'a>,
    ) -> Option<&'a (dyn core::error::Error + 'static)> {
        // SAFETY: We know that the `self.source` field points to the function
        // `source::<C>` below. The safety requirement of that function is that
        // the `C` matches the one stored in the `ReportData` pointed to by
        // `ptr`. This is guaranteed by the caller of this method.
        unsafe { (self.source)(ptr) }
    }

    /// Formats the report using the [`H::display`] function
    /// used when creating this [`ReportVtable`].
    ///
    /// [`H::display`]: ContextHandler::display
    ///
    /// # Safety
    ///
    /// The context type `C` used when creating this [`ReportVtable`] must match
    /// the type stored in the [`ReportData`].
    #[inline]
    pub(super) unsafe fn display(
        &self,
        ptr: RawReportRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that the `self.display` field points to the function
        // `display::<C, H>` below. That function requires that the context type
        // `C` matches the actual context type stored in the `ReportData`, which
        // is guaranteed by our caller.
        unsafe { (self.display)(ptr, formatter) }
    }

    /// Formats the given `RawReportRef` using the [`H::debug`] function
    /// used when creating this [`ReportVtable`].
    ///
    /// [`H::debug`]: ContextHandler::debug
    ///
    /// # Safety
    ///
    /// The context type `C` used when creating this [`ReportVtable`] must match
    /// the type stored in the [`ReportData`].
    #[inline]
    pub(super) unsafe fn debug(
        &self,
        ptr: RawReportRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that the `self.debug` field points to the function
        // `debug::<C, H>` below. That function requires that the context type
        // `C` matches the actual context type stored in the `ReportData`, which
        // is guaranteed by our caller.
        unsafe { (self.debug)(ptr, formatter) }
    }

    /// Calls the [`H::preferred_formatting_style`] function to get the
    /// formatting style preferred by the context when formatted as part of
    /// a report.
    ///
    /// [`H::preferred_formatting_style`]: ContextHandler::preferred_formatting_style
    ///
    /// # Safety
    ///
    /// The context type `C` used when creating this [`ReportVtable`] must match
    /// the type stored in the [`ReportData`].
    #[inline]
    pub(super) unsafe fn preferred_context_formatting_style(
        &self,
        ptr: RawReportRef<'_>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        // SAFETY: We know that the `self.preferred_context_formatting_style` field
        // points to the function `preferred_context_formatting_style::<C, H>` below.
        // That function requires that the context type `C` matches the actual context
        // type stored in the `ReportData`, which is guaranteed by our caller.
        unsafe { (self.preferred_context_formatting_style)(ptr, report_formatting_function) }
    }
}

/// Drops the [`triomphe::Arc<ReportData<C>>`] instance pointed to by this
/// pointer.
///
/// # Safety
///
/// - The caller must ensure that the pointer comes from an
///   [`triomphe::Arc<ReportData<C>>`], which was turned into a pointer using
///   [`triomphe::Arc::into_raw`].
/// - The context type `C` must match the actual context type stored in the
///   [`ReportData`].
/// - After calling this method, the pointer must no longer be used.
pub(super) unsafe fn drop<C: 'static>(ptr: NonNull<ReportData<Erased>>) {
    let ptr: NonNull<ReportData<C>> = ptr.cast();
    let ptr = ptr.as_ptr();
    // SAFETY: Triomphe has two requirements:
    // - The given pointer must be of the correct type and have come from a call to
    //   `Arc::into_raw`.
    // - After `from_raw`, the pointer must not be accessed.
    //
    // The first requirement is guaranteed by the fact that we created the pointer
    // using `Arc::into_raw` and the caller guarantees that the type `C` matches
    // the context type stored in the `ReportData`.
    //
    // The second requirement is guaranteed by the fact that there are no existing
    // references to the same `ReportData` instance, as this method consumes the
    // pointer.
    let arc = unsafe { triomphe::Arc::from_raw(ptr) };
    core::mem::drop(arc);
}

/// Gets the strong count of the [`triomphe::Arc<ReportData<C>>`] pointed to by
/// this pointer.
///
/// # Safety
///
/// - The caller must ensure that the pointer comes from an
///   [`triomphe::Arc<ReportData<C>>`], which was turned into a pointer using
///   [`triomphe::Arc::into_raw`].
/// - The context type `C` stored in the [`ReportData`] must match the `C` used
///   when creating this [`ReportVtable`].
unsafe fn clone_arc<C: 'static>(ptr: NonNull<ReportData<Erased>>) -> RawReport {
    let ptr: *const ReportData<C> = ptr.cast::<ReportData<C>>().as_ptr();

    // SAFETY: Our caller guarantees that we point to a `ReportData<C>` and that
    // this pointer came from a `triomphe::Arc::into_raw`.
    //
    // This fulfills the safety docs for `ArcBorrow::from_ptr`, which explicitly
    // mentions the `as_ptr` (which is called from `into_raw`) is safe.
    let arc_borrow = unsafe { triomphe::ArcBorrow::from_ptr(ptr) };

    let arc = arc_borrow.clone_arc();
    RawReport::from_arc(arc)
}

/// Gets the source error from a report using its handler's source
/// implementation.
///
/// # Safety
///
/// The caller must ensure that the type `C` matches the actual context type
/// stored in the [`ReportData`].
unsafe fn source<'a, C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'a>,
) -> Option<&'a (dyn core::error::Error + 'static)> {
    // SAFETY: Our caller guarantees that the type `C` matches the actual context
    // type stored in the `ReportData`.
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::source(context)
}

/// Formats a report using its handler's display implementation.
///
/// # Safety
///
/// The caller must ensure that the type `C` matches the actual context type
/// stored in the [`ReportData`].
unsafe fn display<C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY: Our caller guarantees that the type `C` matches the actual context
    // type stored in the `ReportData`.
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::display(context, formatter)
}

/// Formats a report using its handler's debug implementation.
///
/// # Safety
///
/// The caller must ensure that the type `C` matches the actual context type
/// stored in the [`ReportData`].
unsafe fn debug<C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY: Our caller guarantees that the type `C` matches the actual context
    // type stored in the `ReportData`.
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::debug(context, formatter)
}

/// Gets the preferred formatting style using the
/// [`H::preferred_formatting_style`] function.
///
/// [`H::preferred_formatting_style`]: ContextHandler::preferred_formatting_style
///
/// # Safety
///
/// The caller must ensure that the type `A` matches the actual attachment type
/// stored in the [`AttachmentData`].
unsafe fn preferred_context_formatting_style<C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'_>,
    report_formatting_function: FormattingFunction,
) -> ContextFormattingStyle {
    // SAFETY: Our caller guarantees that the type `C` matches the actual attachment
    // type stored in the `AttachmentData`.
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::preferred_formatting_style(context, report_formatting_function)
}

/// Gets the preferred formatting style using the
/// [`H::preferred_formatting_style`] function.
///
/// [`H::preferred_formatting_style`]: ContextHandler::preferred_formatting_style
///
/// # Safety
///
/// The caller must ensure that the type `A` matches the actual attachment type
/// stored in the [`AttachmentData`].
unsafe fn strong_count<C: 'static>(ptr: NonNull<ReportData<Erased>>) -> usize {
    let ptr: *const ReportData<C> = ptr.cast::<ReportData<C>>().as_ptr();

    // SAFETY: Our caller guarantees that we point to a `ReportData<C>` and that
    // this pointer came from a `triomphe::Arc::into_raw`.
    //
    // This fulfills the safety docs for `ArcBorrow::from_ptr`, which explicitly
    // mentions the `as_ptr` (which is called from `into_raw`) is safe.
    let arc_borrow = unsafe { triomphe::ArcBorrow::from_ptr(ptr) };

    triomphe::ArcBorrow::strong_count(&arc_borrow)
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use core::{error::Error, fmt};

    use super::*;
    use crate::{handlers::ContextHandler, report::RawReport};

    struct HandlerI32;
    impl ContextHandler<i32> for HandlerI32 {
        fn source(_value: &i32) -> Option<&(dyn Error + 'static)> {
            None
        }

        fn display(value: &i32, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(value, formatter)
        }

        fn debug(value: &i32, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(value, formatter)
        }
    }

    #[test]
    fn test_report_vtable_eq() {
        // Test that vtables have proper static lifetime and can be safely shared
        let vtable1 = ReportVtable::new::<i32, HandlerI32>();
        let vtable2 = ReportVtable::new::<i32, HandlerI32>();

        // Both should be the exact same static instance
        assert!(core::ptr::eq(vtable1, vtable2));
    }

    #[test]
    fn test_report_type_id() {
        let vtable = ReportVtable::new::<i32, HandlerI32>();
        assert_eq!(vtable.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn test_report_clone_eq() {
        let report = RawReport::new::<_, HandlerI32>(42, vec![], vec![]);

        // SAFETY: There are no assumptions about single ownership
        let cloned_report = unsafe { report.as_ref().clone_arc() };

        // Both reports should be the same after
        assert!(core::ptr::eq(
            report.as_ref().as_ptr(),
            cloned_report.as_ref().as_ptr()
        ));
    }
}
