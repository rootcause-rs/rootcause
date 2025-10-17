//! This module encapsulates the fields of the [`ReportData`]. Since this is the only place
//! they are visible, this means that the type of the [`ReportVtable`] is guaranteed to always be in sync
//! with the type of the actual context. This follows from the fact that they are in sync
//! when created and that the API offers no way to change the [`ReportVtable`] or context type after
//! creation.

use alloc::vec::Vec;
use core::ptr::NonNull;

use crate::{
    attachment::RawAttachment,
    handlers::ContextHandler,
    report::{
        raw::{RawReport, RawReportMut, RawReportRef},
        vtable::ReportVtable,
    },
    util::Erased,
};

/// Type-erased report data structure with vtable-based dispatch.
///
/// This struct uses `#[repr(C)]` to enable safe field access in type-erased contexts,
/// allowing access to the vtable and other fields even when the concrete context type `C` is unknown.
#[repr(C)]
pub(super) struct ReportData<C: 'static> {
    /// Reference to the vtable of this report
    vtable: &'static ReportVtable,
    /// The children of this report
    children: Vec<RawReport>,
    /// The attachments of this report
    attachments: Vec<RawAttachment>,
    /// The context data of this report
    context: C,
}

impl<C: 'static> ReportData<C> {
    /// Creates a new [`ReportData`] with the specified handler, context, children and attachments.
    ///
    /// This method creates the vtable for type-erased dispatch and pairs it with the report data.
    pub(super) fn new<H: ContextHandler<C>>(
        context: C,
        children: Vec<RawReport>,
        attachments: Vec<RawAttachment>,
    ) -> Self {
        Self {
            vtable: ReportVtable::new::<C, H>(),
            children,
            attachments,
            context,
        }
    }
}

impl RawReport {
    /// # Safety
    ///
    /// - The caller must ensure that the type `C` matches the actual context type stored in the [`ReportData`].
    /// - The caller must ensure that this is the only existing reference pointing to
    ///   the inner [`ReportData`].
    pub unsafe fn into_parts<C: 'static>(self) -> (C, Vec<RawReport>, Vec<RawAttachment>) {
        let ptr: NonNull<ReportData<Erased>> = self.into_non_null();
        let ptr: NonNull<ReportData<C>> = ptr.cast::<ReportData<C>>();
        let ptr: *const ReportData<C> = ptr.as_ptr();

        // SAFETY: The requirements to this
        // - The given pointer must be a valid pointer to `T` that came from [`Arc::into_raw`].
        // - After `from_raw`, the pointer must not be accessed.
        //
        // Both of these are guaranteed by our caller
        let arc: triomphe::Arc<ReportData<C>> = unsafe { triomphe::Arc::from_raw(ptr) };

        match triomphe::Arc::try_unique(arc) {
            Ok(unique) => {
                let data = triomphe::UniqueArc::into_inner(unique);
                (data.context, data.children, data.attachments)
            }
            Err(_) => {
                if cfg!(debug_assertions) {
                    unreachable!("Caller did not fulfill the guarantee that pointer is unique")
                } else {
                    // SAFETY: This unsafe block *will* cause Undefined Behavior. However our caller guarantees
                    // that the pointer must be unique. This match statement can only be reached when
                    // our caller has broken that requirement, so it is valid to cause Undefined Behavior in this case.
                    unsafe { core::hint::unreachable_unchecked() }
                }
            }
        }
    }
}

impl<'a> RawReportRef<'a> {
    /// Returns a reference to the [`ReportVtable`] of the [`ReportData`] instance.
    pub(super) fn vtable(self) -> &'static ReportVtable {
        let ptr = self.as_ptr();
        // SAFETY: We don't know the actual inner context type, but we do know
        // that it points to an instance of `ReportData<C>` for some specific `C`.
        // Since `ReportData<C>` is `#[repr(C)]`, that means we can access
        // the fields before the actual context.
        //
        // We need to take care to avoid creating an actual reference to
        // the `ReportData` itself though, as that would still be undefined behavior
        // since we don't have the right type.
        let vtable_ptr: *const &'static ReportVtable = unsafe { &raw const (*ptr).vtable };

        // SAFETY: Deferencing the pointer and getting out the `&'static ReportVtable` is valid
        // for the same reasons
        unsafe { *vtable_ptr }
    }

    /// Returns the child reports of this report.
    pub fn children(self) -> &'a [RawReport] {
        let ptr = self.as_ptr();
        // SAFETY: We don't know the actual inner context type, but we do know
        // that it points to an instance of `ReportData<C>` for some specific `C`.
        // Since `ReportData<C>` is `#[repr(C)]`, that means we can access
        // the fields before the actual context.
        //
        // We need to take care to avoid creating an actual reference to
        // the `ReportData` itself though, as that would still be undefined behavior
        // since we don't have the right type.
        let children_ptr: *const Vec<RawReport> = unsafe { &raw const (*ptr).children };

        // SAFETY: We turn the `*const` pointer into a `&'a` reference. This is valid because the
        // existence of the `RawReportMut<'a>` already implied that we had readable access
        // to the report for the 'a lifetime.
        unsafe { &*children_ptr }
    }

    /// Returns the attachments of this report.
    pub fn attachments(self) -> &'a [RawAttachment] {
        let ptr = self.as_ptr();
        // SAFETY: We don't know the actual inner context type, but we do know
        // that it points to an instance of `ReportData<C>` for some specific `C`.
        // Since `ReportData<C>` is `#[repr(C)]`, that means we can access
        // the fields before the actual context.
        //
        // We need to take care to avoid creating an actual reference to
        // the `ReportData` itself though, as that would still be undefined behavior
        // since we don't have the right type.
        let attachments_ptr: *const Vec<RawAttachment> = unsafe { &raw const (*ptr).attachments };

        // SAFETY: We turn the `*const` pointer into a `&'a` reference. This is valid because the
        // existence of the `RawReportMut<'a>` already implied that we had readable access
        // to the report for the 'a lifetime.
        unsafe { &*attachments_ptr }
    }

    /// Accesses the inner context of the [`ReportData`] instance as a reference to the specified type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type `C` matches the actual context type stored in the [`ReportData`].
    pub unsafe fn context_downcast_unchecked<C: 'static>(self) -> &'a C {
        // SAFETY: The inner function requires that `C` matches the type stored, but that is guaranteed by our caller.
        let this = unsafe { self.cast_inner::<C>() };
        &this.context
    }
}

impl<'a> RawReportMut<'a> {
    /// Gets a mutable reference to the child reports.
    pub fn into_children_mut(self) -> &'a mut Vec<RawReport> {
        let ptr = self.into_mut_ptr();

        // SAFETY: We don't know the actual inner context type, but we do know
        // that it points to an instance of `ReportData<C>` for some specific `C`.
        // Since `ReportData<C>` is `#[repr(C)]`, that means we can access
        // the fields before the actual context.
        //
        // We need to take care to avoid creating an actual reference to
        // the `ReportData` itself though, as that would still be undefined behavior
        // since we don't have the right type.
        let children_ptr: *mut Vec<RawReport> = unsafe { &raw mut (*ptr).children };

        // SAFETY: We turn the `*mut` pointer into a `&'a mut` reference. This is valid because the
        // existence of the `RawReportMut<'a>` already implied that nobody else has mutable access
        // to the report for the 'a lifetime.
        unsafe { &mut *children_ptr }
    }

    /// Gets a mutable reference to the attachments.
    pub fn into_attachments_mut(self) -> &'a mut Vec<RawAttachment> {
        let ptr = self.into_mut_ptr();

        // SAFETY: We don't know the actual inner context type, but we do know
        // that it points to an instance of `ReportData<C>` for some specific `C`.
        // Since `ReportData<C>` is `#[repr(C)]`, that means we can access
        // the fields before the actual context.
        //
        // We need to take care to avoid creating an actual reference to
        // the `ReportData` itself though, as that would still be undefined behavior
        // since we don't have the right type.
        let attachments_ptr: *mut Vec<RawAttachment> = unsafe { &raw mut (*ptr).attachments };

        // SAFETY: We turn the `*mut` pointer into a `&'a mut` reference. This is valid because the
        // existence of the `RawReportMut<'a>` already implied that nobody else has mutable access
        // to the report for the 'a lifetime.
        unsafe { &mut *attachments_ptr }
    }

    /// Accesses the inner context of the [`ReportData`] instance as a mutable reference to the specified type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type `C` matches the actual context type stored in the [`ReportData`].
    pub unsafe fn into_context_downcast_unchecked<C: 'static>(self) -> &'a mut C {
        // SAFETY: The inner function requires that `C` matches the type stored, but that is guaranteed by our caller.
        let this = unsafe { self.cast_inner::<C>() };
        &mut this.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_data_field_offsets() {
        // Test that fields are accessible in the expected order for type-erased access
        use core::mem::{offset_of, size_of};

        fn check<T>() {
            // Verify field order: vtable, children, attachments, context
            assert_eq!(offset_of!(ReportData<T>, vtable), 0);
            assert_eq!(
                offset_of!(ReportData<T>, children),
                size_of::<&'static ReportVtable>()
            );
            assert_eq!(
                offset_of!(ReportData<T>, attachments),
                size_of::<&'static ReportVtable>() + size_of::<Vec<RawAttachment>>()
            );
            assert!(
                offset_of!(ReportData<T>, context)
                    >= size_of::<&'static ReportVtable>()
                        + size_of::<Vec<RawAttachment>>()
                        + size_of::<Vec<RawReport>>()
            );
        }

        #[repr(align(32))]
        struct LargeAlignment {
            _value: u8,
        }

        check::<u8>();
        check::<i32>();
        check::<[u64; 4]>();
        check::<i32>();
        check::<LargeAlignment>();
    }
}
