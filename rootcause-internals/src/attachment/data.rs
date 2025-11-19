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
    /// The following safety invariants are guaranteed to be upheld as long as this
    /// struct exists:
    ///
    /// 1. The vtable must always point to an `AttachmentVtable` created for the
    ///    actual attachment type `A` stored below. This is true even when
    ///    accessed via type-erased pointers.
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
    /// The returned vtable is guaranteed to be a vtable for the
    /// attachment type stored in the [`AttachmentData`].
    #[inline]
    pub(super) fn vtable(self) -> &'static AttachmentVtable {
        let ptr = self.as_ptr();
        // SAFETY: The safety requirements for `&raw const (*ptr).vtable` are upheld:
        // 1. `ptr` is a valid pointer to a live `AttachmentData<A>` (for some `A`) as
        //    guaranteed by `RawAttachmentRef`'s invariants
        // 2. `AttachmentData<A>` is `#[repr(C)]`, so the `vtable` field is at a
        //    consistent offset regardless of the type parameter `A`
        // 3. We avoid creating a reference to the full `AttachmentData` struct, which
        //    would be UB since we don't know the correct type parameter
        let vtable_ptr: *const &'static AttachmentVtable = unsafe {
            // @add-unsafe-context: AttachmentData
            &raw const (*ptr).vtable
        };

        // SAFETY: The safety requirements for dereferencing `vtable_ptr` are upheld:
        // 1. The pointer is valid and properly aligned because it points to the first
        //    field of a valid `AttachmentData<A>` instance
        // 2. The `vtable` field is initialized in `AttachmentData::new` and never
        //    modified, so it contains a valid `&'static AttachmentVtable` value
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
