//! This module encapsulates the fields of the [`AttachmentData`]. Since this is
//! the only place they are visible, this means that the type of the
//! [`AttachmentVtable`] is guaranteed to always be in sync with the type of the
//! actual attachment. This follows from the fact that they are in sync
//! when created and that the API offers no way to change the
//! [`AttachmentVtable`] or attachment type after creation.

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
pub(super) struct AttachmentData<A: 'static> {
    /// The Vtable of this attachment
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
        let vtable_ptr: *const &'static AttachmentVtable = unsafe { &raw const (*ptr).vtable };

        // SAFETY: Deferencing the pointer and getting out the `&'static
        // AttachmentVtable` is valid for the same reasons
        unsafe { *vtable_ptr }
    }

    /// Accesses the inner attachment of the [`AttachmentData`] instance as a
    /// reference to the specified type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type `A` matches the actual attachment
    /// type stored in the [`AttachmentData`].
    #[inline]
    pub unsafe fn attachment_downcast_unchecked<A: 'static>(self) -> &'a A {
        // SAFETY: The inner function requires that `A` matches the type stored, but
        // that is guaranteed by our caller.
        let this = unsafe { self.cast_inner::<A>() };
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
