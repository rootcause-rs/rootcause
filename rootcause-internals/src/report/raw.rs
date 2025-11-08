//! Type-erased report pointer types.
//!
//! This module encapsulates the `ptr` field of [`RawReport`], [`RawReportRef`],
//! and [`RawReportMut`], ensuring it is only visible within this module. This
//! visibility restriction guarantees the safety invariant: **the pointer always
//! comes from `Arc<ReportData<C>>`**.
//!
//! # Safety Invariant
//!
//! Since the `ptr` field can only be set via [`RawReport::new`] or
//! [`RawReport::from_arc`] (which create it from `Arc::into_raw`), and cannot
//! be modified afterward (no `pub` or `pub(crate)` fields), the pointer
//! provenance remains valid throughout the value's lifetime.
//!
//! The [`RawReport::drop`] implementation and reference counting operations
//! rely on this invariant to safely reconstruct the `Arc` and manage memory.
//!
//! # Type Erasure
//!
//! The concrete type parameter `C` is erased by casting to
//! `ReportData<Erased>`. The vtable stored within the `ReportData` provides the
//! runtime type information needed to safely downcast and format reports.
//!
//! # Allocation Strategy
//!
//! Unlike attachments (which use `Box`), reports use `triomphe::Arc` for
//! storage. This enables:
//! - Cheap cloning through reference counting
//! - Shared ownership across multiple report references
//! - Thread-safe sharing when the context type is `Send + Sync`

use alloc::vec::Vec;
use core::{any::TypeId, ptr::NonNull};

use crate::{
    attachment::RawAttachment,
    handlers::{ContextFormattingStyle, ContextHandler, FormattingFunction},
    report::data::ReportData,
    util::{CastTo, Erased},
};

/// A pointer to a [`ReportData`] that is guaranteed to point to an initialized
/// instance of a [`ReportData<C>`] for some specific `C`, though we do not know
/// which actual `C` it is.
///
/// However, the pointer is allowed to transition into a non-initialized state
/// inside the [`RawReport::drop`] method.
///
/// The pointer is guaranteed to have been created using
/// [`triomphe::Arc::into_raw`].
///
/// We cannot use a [`triomphe::OffsetArc<ReportData<C>>`] directly, because
/// that does not allow us to type-erase the `C`.
#[repr(transparent)]
pub struct RawReport {
    /// Pointer to the inner report data
    ptr: NonNull<ReportData<Erased>>,
}

impl RawReport {
    /// Creates a new [`RawReport`] from a [`triomphe::Arc<ReportData<C>>`].
    #[inline]
    pub(super) fn from_arc<C: 'static>(data: triomphe::Arc<ReportData<C>>) -> Self {
        let ptr: *const ReportData<C> = triomphe::Arc::into_raw(data);
        let ptr: *mut ReportData<Erased> = ptr as _;

        // SAFETY: Triomphe guarantees that `Arc::into_raw` returns a non-null pointer
        let ptr: NonNull<ReportData<Erased>> = unsafe { NonNull::new_unchecked(ptr) };

        Self { ptr }
    }

    /// Consumes the RawReport without decrementing the reference count and
    /// returns the inner pointer.
    #[inline]
    pub(super) fn into_non_null(self) -> NonNull<ReportData<Erased>> {
        let ptr = self.ptr;
        core::mem::forget(self);
        ptr
    }

    /// Creates a new [`RawReport`] with the specified handler, context,
    /// children, and attachments.
    #[inline]
    pub fn new<C, H>(context: C, children: Vec<RawReport>, attachments: Vec<RawAttachment>) -> Self
    where
        C: 'static,
        H: ContextHandler<C>,
    {
        let data = triomphe::Arc::new(ReportData::new::<H>(context, children, attachments));
        Self::from_arc(data)
    }

    /// Returns a reference to the [`ReportData`] instance.
    #[inline]
    pub fn as_ref(&self) -> RawReportRef<'_> {
        RawReportRef {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }

    /// Returns a mutable reference to the [`ReportData`] instance.
    ///
    /// # Safety
    ///
    /// The caller must ensure that this is the only existing reference pointing
    /// to the inner [`ReportData`].
    #[inline]
    pub unsafe fn as_mut(&mut self) -> RawReportMut<'_> {
        RawReportMut {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }
}

impl core::ops::Drop for RawReport {
    #[inline]
    fn drop(&mut self) {
        let vtable = self.as_ref().vtable();

        // SAFETY: The vtable drop method has three safety requirements:
        // - The pointer must come from `triomphe::Arc<ReportData<C>>` via
        //   `triomphe::Arc::into_raw`
        // - The `C` type in `ReportData<C>` must match the vtable's `C` type
        // - The pointer must not be used after this call
        //
        // These are satisfied because:
        // - The only way to construct or alter a `RawReport` is through the
        //   `RawReport::new` method
        // - The only way to construct or alter a `ReportData` is through the
        //   `ReportData::new` method
        // - This is guaranteed by the fact that we are in the `drop()` function
        unsafe {
            vtable.drop(self.ptr);
        }
    }
}

/// A lifetime-bound pointer to a [`ReportData`] that is guaranteed to point
/// to an initialized instance of a [`ReportData<C>`] for some specific `C`,
/// though we do not know which actual `C` it is.
///
/// We cannot use a [`&'a ReportData<C>`] directly, because that would require
/// us to know the actual type of the context, which we do not.
///
/// [`&'a ReportData<C>`]: ReportData
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RawReportRef<'a> {
    /// Pointer to the inner report data
    ptr: NonNull<ReportData<Erased>>,
    /// Marker to tell the compiler that we should
    /// behave the same as a `&'a ReportData<Erased>`
    _marker: core::marker::PhantomData<&'a ReportData<Erased>>,
}

impl<'a> RawReportRef<'a> {
    /// Casts the [`RawReportRef`] to a [`ReportData<C>`] reference.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type `C` matches the actual context type
    /// stored in the [`ReportData`].
    #[inline]
    pub(super) unsafe fn cast_inner<C: CastTo>(self) -> &'a ReportData<C::Target> {
        // Debug assertion to catch type mismatches in case of bugs
        debug_assert_eq!(self.vtable().type_id(), TypeId::of::<C>());

        let this = self.ptr.cast::<ReportData<C::Target>>();
        // SAFETY: Our caller guarantees that we point to a ReportData<C>. The cast is
        // safe because:
        // - The pointer originated from Arc::into_raw in RawReport construction
        // - The Arc provides valid pointer provenance and prevents premature
        //   deallocation
        // - Shared access through RawReportRef prevents aliasing violations
        // - The lifetime 'a is tied to RawReportRef<'a>, preventing use-after-free
        unsafe { this.as_ref() }
    }

    /// Returns a [`NonNull`] pointer to the [`ReportData`] instance.
    #[inline]
    pub(super) fn as_ptr(self) -> *const ReportData<Erased> {
        self.ptr.as_ptr()
    }

    /// Returns the [`TypeId`] of the context.
    #[inline]
    pub fn context_type_id(self) -> TypeId {
        self.vtable().type_id()
    }

    /// Returns the [`TypeId`] of the context.
    #[inline]
    pub fn context_handler_type_id(self) -> TypeId {
        self.vtable().handler_type_id()
    }

    /// Checks if the type of the context matches the specified type and returns
    /// a reference to it if it does.
    #[inline]
    pub fn context_downcast<C: 'static>(self) -> Option<&'a C> {
        if self.context_type_id() == core::any::TypeId::of::<C>() {
            // SAFETY: We must ensure that the `C` in the ReportData matches the `C` we are
            // using as an argument. However, we have just checked that the
            // types match, so that is fine.
            unsafe { Some(self.context_downcast_unchecked::<C>()) }
        } else {
            None
        }
    }

    /// Returns the source of the context using the [`ContextHandler::source`]
    /// method specified when the [`ReportData`] was created.
    #[inline]
    pub fn context_source(self) -> Option<&'a (dyn core::error::Error + 'static)> {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `C` of the `ReportData` matches the `C` of
        // the `ReportVtable`. However, the only way to construct a `ReportData`
        // is through the `ReportData::new` method, which ensures this fact.
        unsafe { vtable.source(self) }
    }

    /// Formats the context by using the [`ContextHandler::display`] method
    /// specified by the handler used to create the [`ReportData`].
    #[inline]
    pub fn context_display(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `C` of the `ReportData` matches the `C` of
        // the `ReportVtable`. However, the only way to construct a `ReportData`
        // is through the `ReportData::new` method, which ensures this fact.
        unsafe { vtable.display(self, formatter) }
    }

    /// Formats the context by using the [`ContextHandler::debug`] method
    /// specified by the handler used to create the [`ReportData`].
    #[inline]
    pub fn context_debug(self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `C` of the `ReportData` matches the `C` of
        // the `ReportVtable`. However, the only way to construct a `ReportData`
        // is through the `ReportData::new` method, which ensures this fact.
        unsafe { vtable.debug(self, formatter) }
    }

    /// The formatting style preferred by the context when formatted as part of
    /// a report.
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
    pub fn preferred_context_formatting_style(
        self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `C` of the `ReportData` matches the `C` of
        // the `ReportVtable`. However, the only way to construct a `ReportData`
        // is through the `ReportData::new` method, which ensures this fact.
        unsafe { vtable.preferred_context_formatting_style(self, report_formatting_function) }
    }

    /// Clones the inner [`triomphe::Arc`] and returns a new [`RawReport`]
    /// pointing to the same data.
    ///
    /// # Safety
    ///
    /// There must be no external assumptions that there is unique ownership of
    /// the [`triomphe::Arc`].
    #[inline]
    pub unsafe fn clone_arc(self) -> RawReport {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `C` of the `ReportData` matches the `C` of
        // the `ReportVtable`. However, the only way to construct a `ReportData`
        // is through the `ReportData::new` method, which ensures this fact.
        //
        // We must also ensure that there are no external assumptions that
        // there is unique ownership of the `Arc`. However, this is guaranteed by
        // our caller.
        unsafe { vtable.clone_arc(self.ptr) }
    }

    /// Gets the strong_count of the inner [`triomphe::Arc`].
    #[inline]
    pub fn strong_count(self) -> usize {
        let vtable = self.vtable();
        // SAFETY: We must ensure that the `C` of the `ReportData` matches the `C` of
        // the `ReportVtable`. However, the only way to construct a `ReportData`
        // is through the `ReportData::new` method, which ensures this fact.
        unsafe { vtable.strong_count(self.ptr) }
    }
}

/// A mutable lifetime-bound pointer to a [`ReportData`] that is guaranteed to
/// point to an initialized instance of a [`ReportData<C>`] for some specific
/// `C`, though we do not know which actual `C` it is.
///
/// We cannot use a [`&'a mut ReportData<C>`] directly, because that would
/// require us to know the actual type of the context, which we do not.
///
/// [`&'a mut ReportData<C>`]: ReportData
#[repr(transparent)]
pub struct RawReportMut<'a> {
    /// Pointer to the inner report data
    ptr: NonNull<ReportData<Erased>>,
    /// Marker to tell the compiler that we should
    /// behave the same as a `&'a mut ReportData<Erased>`
    _marker: core::marker::PhantomData<&'a mut ReportData<Erased>>,
}

impl<'a> RawReportMut<'a> {
    /// Casts the [`RawReportMut`] to a mutable [`ReportData<C>`] reference.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type `C` matches the actual context type
    /// stored in the [`ReportData`].
    #[inline]
    pub(super) unsafe fn cast_inner<C: CastTo>(self) -> &'a mut ReportData<C::Target> {
        // Debug assertion to catch type mismatches in case of bugs
        debug_assert_eq!(self.as_ref().vtable().type_id(), TypeId::of::<C>());

        let mut this = self.ptr.cast::<ReportData<C::Target>>();
        // SAFETY: Our caller guarantees that we point to a ReportData<C>. The cast is
        // safe because:
        // - The pointer originated from Arc::into_raw in RawReport construction
        // - The Arc provides valid pointer provenance and prevents premature
        //   deallocation
        // - Exclusive access through RawReportMut prevents aliasing violations
        // - The lifetime 'a is tied to RawReportMut<'a>, preventing use-after-free
        unsafe { this.as_mut() }
    }

    /// Reborrows the mutable reference to the [`ReportData`] with a shorter
    /// lifetime.
    #[inline]
    pub fn reborrow<'b>(&'b mut self) -> RawReportMut<'b> {
        RawReportMut {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }

    /// Returns a reference to the [`ReportData`] instance.
    #[inline]
    pub fn as_ref(&self) -> RawReportRef<'_> {
        RawReportRef {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }

    /// Consumes the mutable reference and returns an immutable one with the
    /// same lifetime.
    #[inline]
    pub fn into_ref(self) -> RawReportRef<'a> {
        RawReportRef {
            ptr: self.ptr,
            _marker: core::marker::PhantomData,
        }
    }

    /// Consumes this [`RawReportMut`] and returns a raw mutable pointer to the
    /// underlying [`ReportData`].
    ///
    /// This method is primarily used for internal operations that require
    /// direct pointer access.
    #[inline]
    pub(super) fn into_mut_ptr(self) -> *mut ReportData<Erased> {
        self.ptr.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec};
    use core::{error::Error, fmt};

    use super::*;
    use crate::handlers::ContextHandler;

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

    struct HandlerString;
    impl ContextHandler<String> for HandlerString {
        fn source(_value: &String) -> Option<&(dyn Error + 'static)> {
            None
        }

        fn display(value: &String, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(value, formatter)
        }

        fn debug(value: &String, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(value, formatter)
        }
    }

    #[test]
    fn test_raw_report_size() {
        assert_eq!(
            core::mem::size_of::<RawReport>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<RawReport>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<(), RawReport>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<String, RawReport>>(),
            core::mem::size_of::<String>()
        );
        assert_eq!(
            core::mem::size_of::<Option<Option<RawReport>>>(),
            core::mem::size_of::<Option<usize>>()
        );

        assert_eq!(
            core::mem::size_of::<RawReportRef<'_>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<RawReportRef<'_>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<(), RawReportRef<'_>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<String, RawReportRef<'_>>>(),
            core::mem::size_of::<String>()
        );
        assert_eq!(
            core::mem::size_of::<Option<Option<RawReportRef<'_>>>>(),
            core::mem::size_of::<Option<usize>>()
        );

        assert_eq!(
            core::mem::size_of::<RawReportMut<'_>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Option<RawReportMut<'_>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<(), RawReportMut<'_>>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<Result<String, RawReportMut<'_>>>(),
            core::mem::size_of::<String>()
        );
        assert_eq!(
            core::mem::size_of::<Option<Option<RawReportMut<'_>>>>(),
            core::mem::size_of::<Option<usize>>()
        );
    }

    #[test]
    fn test_raw_report_get_refs() {
        let report = RawReport::new::<i32, HandlerI32>(789, vec![], vec![]);
        let report_ref = report.as_ref();

        // Accessing the pointer multiple times should be safe and consistent
        let ptr1 = report_ref.as_ptr();
        let ptr2 = report_ref.as_ptr();
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_raw_report_clone_arc() {
        // Test that Arc cloning maintains safety
        let report = RawReport::new::<i32, HandlerI32>(123, vec![], vec![]);
        let report_ref = report.as_ref();

        assert_eq!(report_ref.strong_count(), 1);

        // Original should have valid data
        assert_eq!(report_ref.context_type_id(), TypeId::of::<i32>());

        // Clone should work and maintain same type
        // SAFETY: There are no assumptions on single ownership
        let cloned = unsafe { report_ref.clone_arc() };
        let cloned_ref = cloned.as_ref();

        assert_eq!(report_ref.strong_count(), 2);
        assert_eq!(cloned_ref.strong_count(), 2);

        // Both should have same type and vtable
        assert_eq!(report_ref.context_type_id(), cloned_ref.context_type_id());
        assert!(core::ptr::eq(report_ref.vtable(), cloned_ref.vtable()));

        core::mem::drop(cloned);

        // After dropping the strong count should go back down
        assert_eq!(report_ref.strong_count(), 1);
    }

    #[test]
    fn test_raw_attachment_downcast() {
        let int_report = RawReport::new::<i32, HandlerI32>(42, vec![], vec![]);
        let string_report =
            RawReport::new::<String, HandlerString>(String::from("test"), vec![], vec![]);

        let int_ref = int_report.as_ref();
        let string_ref = string_report.as_ref();

        // Are TypeIds what we expect?
        assert_eq!(int_ref.context_type_id(), TypeId::of::<i32>());
        assert_eq!(string_ref.context_type_id(), TypeId::of::<String>());

        // The vtables should be different
        assert!(!core::ptr::eq(int_ref.vtable(), string_ref.vtable()));

        // Cross-type downcasting should fail safely
        assert!(int_ref.context_downcast::<String>().is_none());
        assert!(string_ref.context_downcast::<i32>().is_none());

        // Correct downcasting should work
        assert_eq!(int_ref.context_downcast::<i32>().unwrap(), &42);
        assert_eq!(string_ref.context_downcast::<String>().unwrap(), "test");
    }

    #[test]
    fn test_raw_report_children() {
        let child = RawReport::new::<i32, HandlerI32>(1, vec![], vec![]);
        let parent = RawReport::new::<i32, HandlerI32>(0, vec![child], vec![]);

        let parent_ref = parent.as_ref();
        assert_eq!(parent_ref.context_downcast::<i32>().unwrap(), &0);

        // Parent should have one child
        let children = parent_ref.children();
        assert_eq!(children.len(), 1);

        // Child should be accessible safely
        let child_ref = children[0].as_ref();
        assert_eq!(child_ref.context_type_id(), TypeId::of::<i32>());
        assert_eq!(child_ref.children().len(), 0);
        assert_eq!(child_ref.context_downcast::<i32>().unwrap(), &1);

        // Both should have same vtable (same type)
        assert!(core::ptr::eq(parent_ref.vtable(), child_ref.vtable()));
    }

    #[test]
    fn test_raw_report_with_attachments() {
        use crate::{attachment::RawAttachment, handlers::AttachmentHandler};

        // Create a simple attachment handler for i32
        struct AttachmentHandlerI32;
        impl AttachmentHandler<i32> for AttachmentHandlerI32 {
            fn display(value: &i32, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(value, formatter)
            }

            fn debug(value: &i32, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(value, formatter)
            }
        }

        // Create some attachments
        let attachment1 = RawAttachment::new::<i32, AttachmentHandlerI32>(100);
        let attachment2 = RawAttachment::new::<i32, AttachmentHandlerI32>(200);

        // Create a child report with one attachment
        let child = RawReport::new::<i32, HandlerI32>(1, vec![], vec![attachment1]);

        // Create a parent report with the child and another attachment
        let parent = RawReport::new::<i32, HandlerI32>(0, vec![child], vec![attachment2]);

        let parent_ref = parent.as_ref();
        assert_eq!(parent_ref.context_downcast::<i32>().unwrap(), &0);

        // Parent should have one child and one attachment
        let children = parent_ref.children();
        let attachments = parent_ref.attachments();
        assert_eq!(children.len(), 1);
        assert_eq!(attachments.len(), 1);

        // Child should be accessible safely and have one attachment
        let child_ref = children[0].as_ref();
        assert_eq!(child_ref.context_type_id(), TypeId::of::<i32>());
        assert_eq!(child_ref.context_downcast::<i32>().unwrap(), &1);
        assert_eq!(child_ref.children().len(), 0);
        assert_eq!(child_ref.attachments().len(), 1);

        // Check attachment downcasting works
        let parent_attachment_ref = attachments[0].as_ref();
        let child_attachment_ref = child_ref.attachments()[0].as_ref();

        assert_eq!(
            parent_attachment_ref.attachment_type_id(),
            TypeId::of::<i32>()
        );
        assert_eq!(
            child_attachment_ref.attachment_type_id(),
            TypeId::of::<i32>()
        );

        // Downcast attachments and verify values
        assert_eq!(
            *parent_attachment_ref.attachment_downcast::<i32>().unwrap(),
            200
        );
        assert_eq!(
            *child_attachment_ref.attachment_downcast::<i32>().unwrap(),
            100
        );

        // Both reports should have same vtable (same context type)
        assert!(core::ptr::eq(parent_ref.vtable(), child_ref.vtable()));
    }

    #[test]
    fn test_raw_report_mut_basic() {
        let mut report = RawReport::new::<i32, HandlerI32>(789, vec![], vec![]);

        // SAFETY: We have unique ownership of the report
        let mut report_mut = unsafe { report.as_mut() };

        // Test that we can get a reference from the mutable reference
        let report_ref = report_mut.as_ref();
        assert_eq!(report_ref.context_type_id(), TypeId::of::<i32>());
        assert_eq!(report_ref.context_downcast::<i32>().unwrap(), &789);

        // Test reborrow functionality
        let reborrowed = report_mut.reborrow();
        let ref_from_reborrow = reborrowed.as_ref();
        assert_eq!(ref_from_reborrow.context_type_id(), TypeId::of::<i32>());
        assert_eq!(ref_from_reborrow.context_downcast::<i32>().unwrap(), &789);

        // Test into_mut_ptr
        let ptr = report_mut.into_mut_ptr();
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_raw_report_mut_reborrow_lifetime() {
        let mut report =
            RawReport::new::<String, HandlerString>(String::from("test"), vec![], vec![]);

        // SAFETY: We have unique ownership of the report
        let mut report_mut = unsafe { report.as_mut() };

        // Test that reborrow works with different lifetimes
        {
            let short_reborrow = report_mut.reborrow();
            let ref_from_short = short_reborrow.as_ref();
            assert_eq!(ref_from_short.context_downcast::<String>().unwrap(), "test");
        }

        // Original mutable reference should still be usable
        let final_ref = report_mut.as_ref();
        assert_eq!(final_ref.context_downcast::<String>().unwrap(), "test");
    }

    #[test]
    fn test_raw_report_mut_with_children() {
        let child = RawReport::new::<i32, HandlerI32>(1, vec![], vec![]);
        let mut parent = RawReport::new::<i32, HandlerI32>(0, vec![child], vec![]);

        // SAFETY: We have unique ownership of the parent report
        let mut parent_mut = unsafe { parent.as_mut() };

        let parent_ref = parent_mut.as_ref();
        assert_eq!(parent_ref.context_downcast::<i32>().unwrap(), &0);

        // Check that children are still accessible through the reference
        let children = parent_ref.children();
        assert_eq!(children.len(), 1);

        let child_ref = children[0].as_ref();
        assert_eq!(child_ref.context_downcast::<i32>().unwrap(), &1);

        // Test reborrow with children
        let reborrowed = parent_mut.reborrow();
        let reborrow_ref = reborrowed.as_ref();
        let reborrow_children = reborrow_ref.children();
        assert_eq!(reborrow_children.len(), 1);
        assert_eq!(
            reborrow_children[0]
                .as_ref()
                .context_downcast::<i32>()
                .unwrap(),
            &1
        );
    }

    #[test]
    fn test_raw_report_mut_ptr_consistency() {
        let mut report = RawReport::new::<i32, HandlerI32>(42, vec![], vec![]);

        // Get immutable reference pointer first
        let immut_ref = report.as_ref();
        let immut_ptr = immut_ref.as_ptr() as usize;
        // SAFETY: We have unique ownership of the report
        let report_mut = unsafe { report.as_mut() };

        // Get mutable pointer
        let mut_ptr = report_mut.into_mut_ptr();

        // Both pointers should point to the same location
        assert_eq!(immut_ptr, mut_ptr as *const _ as usize);
    }
    #[test]
    fn test_send_sync() {
        static_assertions::assert_not_impl_any!(RawReport: Send, Sync);
        static_assertions::assert_not_impl_any!(RawReportRef<'_>: Send, Sync);
        static_assertions::assert_not_impl_any!(RawReportMut<'_>: Send, Sync);
    }
}
