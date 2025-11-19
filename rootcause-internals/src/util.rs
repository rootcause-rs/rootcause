//! Internal utility types and traits.

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
