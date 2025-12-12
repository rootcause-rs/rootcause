use crate::{
    markers::{Mutable, ObjectMarkerFor},
    prelude::Report,
};

/// Converts between [`Report`] instances with different context types.
///
/// This trait enables type-safe conversion from one report context to another,
/// preserving the thread-safety marker while transforming the context. It's
/// primarily used as a trait bound for the
/// [`context_to`](crate::Report::context_to) and
/// [`ResultExt::context_to`](crate::result_ext::ResultExt::context_to) methods,
/// allowing ergonomic conversions between related error types.
///
/// Implement `ReportConversion` for reusable error conversions. For one-off
/// transformations, use [`context()`](crate::Report::context) directly.
///
/// # Choosing a Transformation Strategy
///
/// Inside your `convert_report` implementation, you have full access to
/// inspect the report before choosing how to convert it. However most
/// implementations are straightforward and can use one of the following
/// methods:
///
/// | Method                         | New Report? | Runs Hooks? | Preformats? | Result Structure                                       |
/// |--------------------------------|-------------|-------------|-------------|--------------------------------------------------------|
/// | [`context()`]                  | ✅ Yes      | ✅ Yes      | ❌ No       | New context in new node                                |
/// | [`context_transform()`]        | ❌ No       | ❌ No       | ❌ No       | Same structure, new context                            |
/// | [`context_transform_nested()`] | ✅ Yes      | ✅ Yes      | ✅ Yes      | Original context preformatted, new context in new node |
///
/// [`context_transform()`]: crate::Report::context_transform
/// [`context_transform_nested()`]: crate::Report::context_transform_nested
/// [`context()`]: crate::Report::context
///
/// See [`examples/context_methods.rs`] for a complete comparison of these
/// strategies, and [`examples/thiserror_interop.rs`] for integration patterns.
///
/// [`examples/context_methods.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/context_methods.rs
/// [`examples/thiserror_interop.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/thiserror_interop.rs
///
/// # Examples
///
/// Using [`context()`](crate::Report::context):
///
/// ```rust
/// use rootcause::{ReportConversion, markers, prelude::*};
///
/// # #[derive(Debug)]
/// enum AppError {
///   Io
/// }
/// # impl std::fmt::Display for AppError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         match self { AppError::Io => write!(f, "Io error") }
/// #     }
/// # }
///
/// impl<O, T> ReportConversion<std::io::Error, O, T> for AppError
/// where
///     AppError: markers::ObjectMarkerFor<T>,
/// {
///     fn convert_report(
///         report: Report<std::io::Error, O, T>,
///     ) -> Report<Self, markers::Mutable, T> {
///         report.context(AppError::Io)
///     }
/// }
/// ```

///
/// Using [`context_transform()`](crate::Report::context_transform):
///
/// ```rust
/// use rootcause::{ReportConversion, markers, prelude::*};
/// # use std::io;
///
/// # #[derive(Debug)]
/// enum AppError {
///   Io(io::Error)
/// }
/// # impl std::fmt::Display for AppError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         match self { AppError::Io(e) => write!(f, "IO error: {}", e) }
/// #     }
/// # }
///
/// impl<T> ReportConversion<io::Error, markers::Mutable, T> for AppError
/// where
///     AppError: markers::ObjectMarkerFor<T>,
/// {
///     fn convert_report(
///         report: Report<io::Error, markers::Mutable, T>,
///     ) -> Report<Self, markers::Mutable, T> {
///         report.context_transform(AppError::Io)
///     }
/// }
/// ```
pub trait ReportConversion<C: ?Sized, O, T>: ObjectMarkerFor<T> {
    /// Converts a report with context `C` to a report with context `Self`.
    ///
    /// This method is called automatically by [`context_to`](crate::Report::context_to)
    /// and [`ResultExt::context_to`](crate::result_ext::ResultExt::context_to).
    /// The thread-safety marker `T` is preserved, while the ownership marker
    /// becomes [`Mutable`] for the new report.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{ReportConversion, markers, prelude::*};
    /// # #[derive(Debug)]
    /// # enum AppError { ParseError(String) }
    /// # impl std::fmt::Display for AppError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #         match self { AppError::ParseError(msg) => write!(f, "Parse error: {}", msg) }
    /// #     }
    /// # }
    ///
    /// impl<O, T> ReportConversion<std::num::ParseIntError, O, T> for AppError
    /// where
    ///     AppError: markers::ObjectMarkerFor<T>,
    /// {
    ///     fn convert_report(
    ///         report: Report<std::num::ParseIntError, O, T>,
    ///     ) -> Report<Self, markers::Mutable, T> {
    ///         report.context(AppError::ParseError("Invalid number format".to_string()))
    ///     }
    /// }
    /// ```
    fn convert_report(report: Report<C, O, T>) -> Report<Self, Mutable, T>;
}
