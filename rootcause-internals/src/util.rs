//! Internal utility types and traits.

/// Helper trait to force explicit turbofish syntax on pointer casts.
///
/// This trait prevents accidental type inference on calls like
/// `ptr.cast::<U>()`, ensuring callers explicitly specify the target type. This
/// helps catch bugs where the wrong type might be silently inferred.
///
/// The trait's associated type `Target` is always equal to `Self`, but the
/// compiler cannot infer it automatically, requiring explicit specification.
pub(crate) trait CastTo: 'static {
    /// Target type of the cast. Always equal to `Self`, but cannot be inferred
    /// by the compiler, forcing explicit turbofish syntax.
    type Target: 'static;
}

impl<T: 'static> CastTo for T {
    type Target = T;
}

/// Marker type used when type-erasing reports or attachments.
///
/// This zero-sized type serves as a placeholder in generic type parameters
/// when the actual concrete type has been erased. For example,
/// `AttachmentData<Erased>` represents an attachment whose concrete type
/// is unknown at the current scope.
///
/// Using a distinct marker type (rather than `()`) makes the intent clearer
/// in type signatures and error messages.
pub(crate) struct Erased;
