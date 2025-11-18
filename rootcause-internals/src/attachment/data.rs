//! `AttachmentData<A>` wrapper and field access.
//!
//! This module encapsulates the fields of [`AttachmentData`], ensuring they are
//! only visible within this module. This visibility restriction guarantees the
//! safety invariant: **the vtable type must always match the actual attachment
//! type**.
//!
//! # Safety Invariant
//!
//! Since `AttachmentData` can only be constructed via [`AttachmentData::new`]
//! (which creates matching vtable and attachment), and fields cannot be
//! modified after construction (no `pub` or `pub(crate)` fields), the types
//! remain in sync throughout the value's lifetime.
//!
//! # `#[repr(C)]` Layout
//!
//! The `#[repr(C)]` attribute enables safe field projection even when the type
//! parameter `A` is erased. This allows accessing the vtable field from a
//! pointer to `AttachmentData<Erased>` without constructing an invalid
//! reference to the full struct.

use crate::{
    attachment::{raw::RawAttachmentRef, vtable::AttachmentVtable},
    handlers::AttachmentHandler,
};

/// Type-erased attachment data structure with vtable-based dispatch.
///
/// This struct uses `#[repr(C)]` to enable safe field access in type-erased
/// contexts, allowing access to the vtable field even when the concrete
/// attachment type `A` is unknown.
#[repr(C)]
pub(crate) struct AttachmentData<A: 'static> {
    /// The Vtable of this attachment
    ///
    /// # Safety
    ///
    /// The following safety invariants must be upheld as long as this
    /// struct exists:
    ///
    /// 1. The `AttachmentVtable` always pointers to a vtable created for
    ///    the actual vtable type `A` stored below. This is true even
    ///    when accessed via type-erased pointers.
    vtable: &'static AttachmentVtable,
    /// The actual attachment data
    attachment: A,
}

impl<A: 'static> AttachmentData<A> {
    /// Creates a new [`AttachmentData`] with the specified handler and
    /// attachment.
    ///
    /// This method creates the vtable for type-erased dispatch and pairs it
    /// with the attachment data.
    #[inline]
    pub(super) fn new<H: AttachmentHandler<A>>(attachment: A) -> Self {
        Self {
            vtable: AttachmentVtable::new::<A, H>(),
            attachment,
        }
    }
}

impl<'a> RawAttachmentRef<'a> {
    /// Returns a reference to the [`AttachmentVtable`] of the
    /// [`AttachmentData`] instance.
    ///
    /// This is guaranteed to be valid for the actual attachment type
    /// stored in the `AttachmentData`, even when accessed via
    /// type-erased pointers.
    #[inline]
    pub(super) fn vtable(self) -> &'static AttachmentVtable {
        let ptr = self.as_ptr();
        // SAFETY: We don't know the actual inner attachment type, but we do know
        // that it points to an instance of `AttachmentData<A>` for some specific `A`.
        // Since `AttachmentData<A>` is `#[repr(C)]`, that means that it's
        // safe to create pointers to the fields before the actual attachment.
        //
        // We need to take care to avoid creating an actual reference to
        // the `AttachmentData` itself though, as that would still be undefined behavior
        // since we don't have the right type.
        let vtable_ptr: *const &'static AttachmentVtable = unsafe {
            // @add-unsafe-context: AttachmentData
            &raw const (*ptr).vtable
        };

        // SAFETY: The vtable_ptr is derived from a valid Box pointer and points
        // to an initialized `&'static AttachmentVtable` field. Dereferencing is safe
        // because:
        // - The pointer is valid and properly aligned
        // - The vtable field is initialized in AttachmentData::new and never modified
        unsafe { *vtable_ptr }
    }

    /// Accesses the inner attachment of the [`AttachmentData`] instance as a
    /// reference to the specified type.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `A` matches the actual attachment type stored in the
    ///    [`AttachmentData`].
    #[inline]
    pub unsafe fn attachment_downcast_unchecked<A: 'static>(self) -> &'a A {
        // SAFETY:
        // 1. Guaranteed by the caller
        let this = unsafe {
            // @add-unsafe-context: AttachmentData
            self.cast_inner::<A>()
        };
        &this.attachment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_field_offsets() {
        use core::mem::{offset_of, size_of};

        #[repr(align(32))]
        struct LargeAlignment {
            _value: u8,
        }

        assert_eq!(offset_of!(AttachmentData<u8>, vtable), 0);
        assert_eq!(offset_of!(AttachmentData<u32>, vtable), 0);
        assert_eq!(offset_of!(AttachmentData<[u64; 4]>, vtable), 0);
        assert_eq!(offset_of!(AttachmentData<LargeAlignment>, vtable), 0);

        assert!(
            offset_of!(AttachmentData<u8>, attachment) >= size_of::<&'static AttachmentVtable>()
        );
        assert!(
            offset_of!(AttachmentData<u32>, attachment) >= size_of::<&'static AttachmentVtable>()
        );
        assert!(
            offset_of!(AttachmentData<[u64; 4]>, attachment)
                >= size_of::<&'static AttachmentVtable>()
        );
        assert!(
            offset_of!(AttachmentData<LargeAlignment>, attachment)
                >= size_of::<&'static AttachmentVtable>()
        );
    }
}
