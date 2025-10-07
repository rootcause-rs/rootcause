//! Utility module

/// Helper trait to force turbofish on calls to e.g. `ptr.cast::<U>()`.
pub(crate) trait CastTo: 'static {
    /// Target type of the cast. This is always equal to `Self`, but
    /// can deliberately not be inferred by the compiler.
    type Target: 'static;
}

impl<T: 'static> CastTo for T {
    type Target = T;
}

/// Marker struct used when type-erasing reports or attachments
pub(crate) struct Erased;
