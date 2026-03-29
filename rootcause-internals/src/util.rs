//! Internal utility types and traits.

/// Marker type used when type-erasing reports or attachments.
///
/// This zero-sized type serves as a placeholder in generic type parameters
/// when the actual concrete type has been erased. [`ReportData<Erased>`]
/// represents a report whose concrete type
/// is unknown at the current scope, and [`AttachmentData<Erased>`] same for
/// attachments.
///
/// [`ReportData<Erased>`]: crate::report::data::ReportData
/// [`AttachmentData<Erased>`]: crate::attachment::data::AttachmentData
///
/// Using a distinct marker type (rather than `()`) makes the intent clearer
/// in type signatures and error messages.
pub(crate) struct Erased;
