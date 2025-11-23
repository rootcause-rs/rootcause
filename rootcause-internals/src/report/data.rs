//! `ReportData<C>` wrapper and field access.
//!
//! This module encapsulates the fields of [`ReportData`], ensuring they are
//! only visible within this module. This visibility restriction guarantees the
//! safety invariant: **the vtable type must always match the actual context
//! type**.
//!
//! # Safety Invariant
//!
//! Since [`ReportData`] can only be constructed via [`ReportData::new`] (which
//! creates matching vtable and context), and fields cannot be modified after
//! construction (no `pub` or `pub(crate)` fields), the types remain in sync
//! throughout the value's lifetime.
//!
//! # `#[repr(C)]` Layout
//!
//! The `#[repr(C)]` attribute enables safe field projection even when the type
//! parameter `C` is erased. This allows accessing the vtable, children, and
//! attachments fields from a pointer to `ReportData<Erased>` without
//! constructing an invalid reference to the full struct.

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
/// This struct uses `#[repr(C)]` to enable safe field access in type-erased
/// contexts, allowing access to the vtable and other fields even when the
/// concrete context type `C` is unknown.
#[repr(C)]
pub(crate) struct ReportData<C: 'static> {
    /// Reference to the vtable of this report
    ///
    /// # Safety
    ///
    /// The following safety invariants are guaranteed to be upheld as long as
    /// this struct exists:
    ///
    /// 1. The vtable must always point to a `ReportVtable` created for the
    ///    actual context type `C` stored below. This is true even when accessed
    ///    via type-erased pointers.
    vtable: &'static ReportVtable,
    /// The children of this report
    children: Vec<RawReport>,
    /// The attachments of this report
    attachments: Vec<RawAttachment>,
    /// The context data of this report
    context: C,
}

impl<C: 'static> ReportData<C> {
    /// Creates a new [`ReportData`] with the specified handler, context,
    /// children and attachments.
    ///
    /// This method creates the vtable for type-erased dispatch and pairs it
    /// with the report data.
    #[inline]
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
    /// Deconstructs this report into its context, children, and attachments.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `C` matches the actual context type stored in the
    ///    [`ReportData`]
    /// 2. This is the only existing reference pointing to the inner
    ///    [`ReportData`]
    pub unsafe fn into_parts<C: 'static>(self) -> (C, Vec<RawReport>, Vec<RawAttachment>) {
        let ptr: NonNull<ReportData<Erased>> = self.into_non_null();
        let ptr: NonNull<ReportData<C>> = ptr.cast::<ReportData<C>>();
        let ptr: *const ReportData<C> = ptr.as_ptr();

        // SAFETY:
        // 1. The pointer is valid and came from `Arc::into_raw` (guaranteed by
        //    RawReport construction)
        // 2. After `from_raw` the `ptr` is not accessed.
        let arc = unsafe { triomphe::Arc::<ReportData<C>>::from_raw(ptr) };

        match triomphe::Arc::try_unique(arc) {
            Ok(unique) => {
                let data = triomphe::UniqueArc::into_inner(unique);
                (data.context, data.children, data.attachments)
            }
            Err(_) => {
                // Note: We could use `unreachable_unchecked` here in release builds for
                // performance, but `into_parts` is not expected to be used in
                // performance-critical paths, so a normal panic is preferable for
                // better debugging.
                unreachable!("Caller did not fulfill the guarantee that pointer is unique")
            }
        }
    }
}

impl<'a> RawReportRef<'a> {
    /// Returns a reference to the [`ReportVtable`] of this report.
    ///
    /// The returned vtable is guaranteed to match the context type stored in
    /// the [`ReportData`].
    #[inline]
    pub(super) fn vtable(self) -> &'static ReportVtable {
        let ptr = self.as_ptr();
        // SAFETY: The safety requirements for `&raw const (*ptr).vtable` are upheld:
        // 1. `ptr` is a valid pointer to a live `ReportData<C>` (for some `C`) as
        //    guaranteed by `RawReportRef`'s invariants
        // 2. `ReportData<C>` is `#[repr(C)]`, so the `vtable` field is at a consistent
        //    offset regardless of the type parameter `C`
        // 3. We avoid creating a reference to the full `ReportData` struct, which would
        //    be UB since we don't know the correct type parameter
        let vtable_ptr: *const &'static ReportVtable = unsafe {
            // @add-unsafe-context: ReportData
            &raw const (*ptr).vtable
        };

        // SAFETY: The safety requirements for dereferencing `vtable_ptr` are upheld:
        // 1. The pointer is valid and properly aligned because it points to the first
        //    field of a valid `ReportData<C>` instance
        // 2. The `vtable` field is initialized in `ReportData::new` and never modified,
        //    so it contains a valid `&'static ReportVtable` value
        unsafe { *vtable_ptr }
    }

    /// Returns the child reports of this report.
    #[inline]
    pub fn children(self) -> &'a Vec<RawReport> {
        let ptr: *const ReportData<Erased> = self.as_ptr();

        // SAFETY: The safety requirements for `&raw const (*ptr).children` are upheld:
        // 1. `ptr` is a valid pointer to a live `ReportData<C>` (for some `C`) as
        //    guaranteed by `RawReportRef`'s invariants
        // 2. `ReportData<C>` is `#[repr(C)]`, so the `children` field is at a
        //    consistent offset regardless of the type parameter `C`
        // 3. We avoid creating a reference to the full `ReportData` struct, which would
        //    be UB since we don't know the correct type parameter
        let children_ptr: *const Vec<RawReport> = unsafe {
            // @add-unsafe-context: ReportData
            &raw const (*ptr).children
        };

        // SAFETY: We turn the `*const` pointer into a `&'a` reference. This is valid
        // because the existence of the `RawReportRef<'a>` already implies that
        // we have readable access to the report for the 'a lifetime.
        unsafe { &*children_ptr }
    }

    /// Returns the attachments of this report.
    #[inline]
    pub fn attachments(self) -> &'a Vec<RawAttachment> {
        let ptr = self.as_ptr();

        // SAFETY: The safety requirements for `&raw const (*ptr).attachments` are
        // upheld:
        // 1. `ptr` is a valid pointer to a live `ReportData<C>` (for some `C`) as
        //    guaranteed by `RawReportRef`'s invariants
        // 2. `ReportData<C>` is `#[repr(C)]`, so the `attachments` field is at a
        //    consistent offset regardless of the type parameter `C`
        // 3. We avoid creating a reference to the full `ReportData` struct, which would
        //    be UB since we don't know the correct type parameter
        let attachments_ptr: *const Vec<RawAttachment> = unsafe {
            // @add-unsafe-context: ReportData
            &raw const (*ptr).attachments
        };

        // SAFETY: We turn the `*const` pointer into a `&'a` reference. This is valid
        // because the existence of the `RawReportRef<'a>` already implies that
        // we have readable access to the report for the 'a lifetime.
        unsafe { &*attachments_ptr }
    }

    /// Downcasts the context to the specified type and returns a reference.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `C` matches the actual context type stored in the
    ///    [`ReportData`]
    #[inline]
    pub unsafe fn context_downcast_unchecked<C: 'static>(self) -> &'a C {
        // SAFETY:
        // 1. Guaranteed by the caller
        let this = unsafe { self.cast_inner::<C>() };
        &this.context
    }
}

impl<'a> RawReportMut<'a> {
    /// Gets a mutable reference to the child reports.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. In case there are other references to the same report and they make
    ///    assumptions about the report children being `Send+Sync`, then those
    ///    assumptions must be upheld when modifying the children.
    #[inline]
    pub unsafe fn into_children_mut(self) -> &'a mut Vec<RawReport> {
        let ptr = self.into_mut_ptr();

        // SAFETY: The safety requirements for `&raw mut (*ptr).children` are upheld:
        // 1. `ptr` is a valid pointer to a live `ReportData<C>` (for some `C`) as
        //    guaranteed by `RawReportMut`'s invariants
        // 2. `ReportData<C>` is `#[repr(C)]`, so the `children` field is at a
        //    consistent offset regardless of the type parameter `C`
        // 3. We avoid creating a reference to the full `ReportData` struct, which would
        //    be UB since we don't know the correct type parameter
        let children_ptr: *mut Vec<RawReport> = unsafe {
            // @add-unsafe-context: ReportData
            &raw mut (*ptr).children
        };

        // SAFETY: We turn the `*mut` pointer into a `&'a mut` reference. This is valid
        // because the existence of the `RawReportMut<'a>` already implied that
        // nobody else has mutable access to the report for the 'a lifetime.
        unsafe { &mut *children_ptr }
    }

    /// Deconstructs the `RawReportMut` and returns a mutable reference to the
    /// attachments vector.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. In case there are other references to the same report and they make
    ///    assumptions about the report attachments being `Send+Sync`, then
    ///    those assumptions must be upheld when modifying the attachments.
    #[inline]
    pub unsafe fn into_attachments_mut(self) -> &'a mut Vec<RawAttachment> {
        let ptr = self.into_mut_ptr();

        // SAFETY: The safety requirements for `&raw mut (*ptr).attachments` are upheld:
        // 1. `ptr` is a valid pointer to a live `ReportData<C>` (for some `C`) as
        //    guaranteed by `RawReportMut`'s invariants
        // 2. `ReportData<C>` is `#[repr(C)]`, so the `attachments` field is at a
        //    consistent offset regardless of the type parameter `C`
        // 3. We avoid creating a reference to the full `ReportData` struct, which would
        //    be UB since we don't know the correct type parameter
        let attachments_ptr: *mut Vec<RawAttachment> = unsafe {
            // @add-unsafe-context: ReportData
            &raw mut (*ptr).attachments
        };

        // SAFETY: We turn the `*mut` pointer into a `&'a mut` reference. This is valid
        // because the existence of the `RawReportMut<'a>` already implied that
        // nobody else has mutable access to the report for the 'a lifetime.
        unsafe { &mut *attachments_ptr }
    }

    /// Downcasts the context to the specified type and returns a mutable
    /// reference.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `C` matches the actual context type stored in the
    ///    [`ReportData`]
    #[inline]
    pub unsafe fn into_context_downcast_unchecked<C: 'static>(self) -> &'a mut C {
        // SAFETY:
        // 1. Guaranteed by the caller
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
