//! Vtable for type-erased report operations.
//!
//! This module contains the [`ReportVtable`] which enables calling handler
//! methods on reports when their concrete context type `C` and handler type `H`
//! have been erased. The vtable stores function pointers that dispatch to the
//! correct typed implementations.
//!
//! This module encapsulates the fields of [`ReportVtable`] so they cannot be
//! accessed directly. This visibility restriction guarantees the safety
//! invariant: **the vtable's type parameters must match the actual report
//! context type and handler stored in the `ReportData`**.
//!
//! # Safety Invariant
//!
//! This invariant is maintained because vtables are created as `&'static`
//! references via [`ReportVtable::new`], which pairs the function pointers
//! with specific types `C` and `H` at compile time.

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
///
/// # Safety
///
/// The following safety invariants are guaranteed to be upheld as long as this
/// struct exists:
///
/// * The fields `drop`, `clone_arc`, `strong_count`, `source`, `display`,
///   `debug`, and `preferred_context_formatting_style` all point to the
///   functions defined below
/// * The concrete pointers are all instantiated with the same context type `C`
///   and handler type `H` that were used to create this `ReportVtable`.
pub(crate) struct ReportVtable {
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
    /// Formats the report using the `debug` method on the handler.
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
    /// The caller must ensure:
    ///
    /// 1. The pointer comes from a [`triomphe::Arc<ReportData<C>>`] turned into
    ///    a pointer via [`triomphe::Arc::into_raw`]
    /// 2. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`ReportData`].
    /// 3. The pointer is not used after calling this method. Storing the
    ///    pointer in structures that claim ownership of it, such as another
    ///    `Arc` counts as using after calling this method.
    #[inline]
    pub(super) unsafe fn drop(&self, ptr: NonNull<ReportData<Erased>>) {
        // SAFETY: We know that `self.drop` points to the function `drop::<C>` below.
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

    /// Clones the [`triomphe::Arc<ReportData<C>>`] pointed to by this pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The pointer comes from a [`triomphe::Arc<ReportData<C>>`] turned into
    ///    a pointer via [`triomphe::Arc::into_raw`]
    /// 2. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`ReportData`].
    /// 3. All other references to this report are compatible with shared
    ///    ownership. Specifically none of them assume that the strong_count is
    ///    `1`.
    #[inline]
    pub(super) unsafe fn clone_arc(&self, ptr: NonNull<ReportData<Erased>>) -> RawReport {
        // SAFETY: We know that `self.clone_arc` points to the function `clone_arc::<C>`
        // below. That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        // 2. Guaranteed by the caller
        // 3. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: clone_arc
            (self.clone_arc)(ptr)
        }
    }

    /// Gets the strong count of the [`triomphe::Arc<ReportData<C>>`] pointed to
    /// by this pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The pointer comes from [`triomphe::Arc<ReportData<C>>`] via
    ///    [`triomphe::Arc::into_raw`]
    /// 2. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`ReportData`].
    #[inline]
    pub(super) unsafe fn strong_count(&self, ptr: NonNull<ReportData<Erased>>) -> usize {
        // SAFETY: We know that `self.strong_count` points to the function
        // `strong_count::<C>` below. That function's safety requirements are
        // upheld:
        // 1. Guaranteed by the caller
        // 2. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: strong_count
            (self.strong_count)(ptr)
        }
    }

    /// Returns a reference to the source of the error using the [`H::source`]
    /// function used when creating this [`ReportVtable`].
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`RawReportRef`].
    ///
    /// [`H::source`]: ContextHandler::source
    #[inline]
    pub(super) unsafe fn source<'a>(
        &self,
        ptr: RawReportRef<'a>,
    ) -> Option<&'a (dyn core::error::Error + 'static)> {
        // SAFETY: We know that `self.source` points to the function `source::<C, H>`
        // below. That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: source
            (self.source)(ptr)
        }
    }

    /// Formats the report using the [`H::display`] function
    /// used when creating this [`ReportVtable`].
    ///
    /// [`H::display`]: ContextHandler::display
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`RawReportRef`].
    #[inline]
    pub(super) unsafe fn display(
        &self,
        ptr: RawReportRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that `self.display` points to the function `display::<C, H>`
        // below. That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: display
            (self.display)(ptr, formatter)
        }
    }

    /// Formats the given `RawReportRef` using the [`H::debug`] function
    /// used when creating this [`ReportVtable`].
    ///
    /// [`H::debug`]: ContextHandler::debug
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`RawReportRef`].
    #[inline]
    pub(super) unsafe fn debug(
        &self,
        ptr: RawReportRef<'_>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        // SAFETY: We know that `self.debug` points to the function `debug::<C, H>`
        // below. That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: debug
            (self.debug)(ptr, formatter)
        }
    }

    /// Calls the [`H::preferred_formatting_style`] function to get the
    /// formatting style preferred by the context when formatted as part of
    /// a report.
    ///
    /// [`H::preferred_formatting_style`]: ContextHandler::preferred_formatting_style
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. This [`ReportVtable`] must be a vtable for the context type stored in
    ///    the [`RawReportRef`].
    #[inline]
    pub(super) unsafe fn preferred_context_formatting_style(
        &self,
        ptr: RawReportRef<'_>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        // SAFETY: We know that `self.preferred_context_formatting_style` points to the
        // function `preferred_context_formatting_style::<C, H>` below.
        // That function's safety requirements are upheld:
        // 1. Guaranteed by the caller
        unsafe {
            // See https://github.com/rootcause-rs/rootcause-unsafe-analysis for details
            // @add-unsafe-context: preferred_context_formatting_style
            (self.preferred_context_formatting_style)(ptr, report_formatting_function)
        }
    }
}

/// Drops the [`triomphe::Arc<ReportData<C>>`] instance pointed to by this
/// pointer.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The pointer comes from [`triomphe::Arc<ReportData<C>>`] via
///    [`triomphe::Arc::into_raw`]
/// 2. The context type `C` matches the actual context type stored in the
///    [`ReportData`]
/// 3. The pointer is not used after calling this method. Storing the
///    pointer in structures that claim ownership of it, such as another
///    `Arc` counts as using after calling this method.
pub(super) unsafe fn drop<C: 'static>(ptr: NonNull<ReportData<Erased>>) {
    let ptr: NonNull<ReportData<C>> = ptr.cast();
    let ptr = ptr.as_ptr();
    // SAFETY:
    // 1. The pointer has the correct type and came from `Arc::into_raw` (guaranteed
    //    by caller)
    // 2. After `from_raw`, the pointer is consumed and not accessed again
    let arc = unsafe {
        // @add-unsafe-context: ReportData
        triomphe::Arc::from_raw(ptr)
    };
    core::mem::drop(arc);
}

/// Clones the [`triomphe::Arc<ReportData<C>>`] pointed to by this pointer.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The pointer comes from a [`triomphe::Arc<ReportData<C>>`] turned into a
///    pointer via [`triomphe::Arc::into_raw`]
/// 2. The context type `C` matches the actual context type stored in the
///    [`ReportData`]
/// 3. All other references to this report are compatible with shared ownership.
///    Specifically none of them assume that the strong_count is `1`.
unsafe fn clone_arc<C: 'static>(ptr: NonNull<ReportData<Erased>>) -> RawReport {
    let ptr: *const ReportData<C> = ptr.cast::<ReportData<C>>().as_ptr();

    // SAFETY: The pointer is valid and came from `Arc::into_raw` with the correct
    // type (guaranteed by the caller), which fulfills the requirements for
    // `ArcBorrow::from_ptr`.
    let arc_borrow = unsafe {
        // @add-unsafe-context: ReportData
        triomphe::ArcBorrow::from_ptr(ptr)
    };

    let arc = arc_borrow.clone_arc();
    RawReport::from_arc(arc)
}

/// Gets the strong count of the [`triomphe::Arc<ReportData<C>>`] pointed to by
/// this pointer.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The pointer comes from [`triomphe::Arc<ReportData<C>>`] via
///    [`triomphe::Arc::into_raw`]
/// 2. The context type `C` matches the actual context type stored in the
///    [`ReportData`]
unsafe fn strong_count<C: 'static>(ptr: NonNull<ReportData<Erased>>) -> usize {
    let ptr: *const ReportData<C> = ptr.cast::<ReportData<C>>().as_ptr();

    // SAFETY: The pointer is valid and came from `Arc::into_raw` with the correct
    // type (guaranteed by the caller), which fulfills the requirements for
    // `ArcBorrow::from_ptr`.
    let arc_borrow = unsafe {
        // @add-unsafe-context: ReportData
        triomphe::ArcBorrow::from_ptr(ptr)
    };

    triomphe::ArcBorrow::strong_count(&arc_borrow)
}

/// Gets the source error from a report using its handler's source
/// implementation.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The type `C` matches the actual context type stored in the [`ReportData`]
unsafe fn source<'a, C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'a>,
) -> Option<&'a (dyn core::error::Error + 'static)> {
    // SAFETY:
    // 1. Guaranteed by the caller
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::source(context)
}

/// Formats a report using its handler's display implementation.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The type `C` matches the actual context type stored in the [`ReportData`]
unsafe fn display<C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY:
    // 1. Guaranteed by the caller
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::display(context, formatter)
}

/// Formats a report using its handler's debug implementation.
///
/// # Safety
///
/// The caller must ensure:
///
/// 1. The type `C` matches the actual context type stored in the [`ReportData`]
unsafe fn debug<C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'_>,
    formatter: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    // SAFETY:
    // 1. Guaranteed by the caller
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
/// The caller must ensure:
///
/// 1. The type `C` matches the actual context type stored in the [`ReportData`]
unsafe fn preferred_context_formatting_style<C: 'static, H: ContextHandler<C>>(
    ptr: RawReportRef<'_>,
    report_formatting_function: FormattingFunction,
) -> ContextFormattingStyle {
    // SAFETY:
    // 1. Guaranteed by the caller
    let context: &C = unsafe { ptr.context_downcast_unchecked::<C>() };
    H::preferred_formatting_style(context, report_formatting_function)
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

        // Both reports should point to the same underlying data
        assert!(core::ptr::eq(
            report.as_ref().as_ptr(),
            cloned_report.as_ref().as_ptr()
        ));
    }
}
