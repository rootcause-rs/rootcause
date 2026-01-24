use crate::{
    markers::{Mutable, ObjectMarkerFor},
    prelude::Report,
};

/// Converts between [`Report`] instances with different context types.
///
/// Enables type-safe conversion from one report context to another, preserving
/// the thread-safety marker. Used as a trait bound for
/// [`context_to`](crate::Report::context_to) and
/// [`ResultExt::context_to`](crate::result_ext::ResultExt::context_to),
/// allowing ergonomic conversions between related error types. For one-off
/// transformations, use [`context`](crate::Report::context) directly.
///
/// Implementations typically use
/// [`context_transform`](crate::Report::context_transform) or
/// [`context`](crate::Report::context). See [`examples/context_methods.rs`] for
/// comparison, and [`examples/thiserror_interop.rs`] for integration patterns.
///
/// [`examples/context_methods.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/context_methods.rs
/// [`examples/thiserror_interop.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/thiserror_interop.rs
///
/// # Examples
///
/// ```
/// use rootcause::{ReportConversion, markers, prelude::*};
/// # use std::io;
///
/// # #[derive(Debug)]
/// enum AppError {
///     Io(io::Error),
/// }
/// # impl std::fmt::Display for AppError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         match self { AppError::Io(e) => write!(f, "IO error: {}", e) }
/// #     }
/// # }
///
/// // Using context_transform to preserve report structure
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
    /// Called automatically by [`context_to`](crate::Report::context_to) and
    /// [`ResultExt::context_to`](crate::result_ext::ResultExt::context_to).
    /// Preserves the thread-safety marker `T`, returns [`Mutable`] ownership.
    fn convert_report(report: Report<C, O, T>) -> Report<Self, Mutable, T>;
}
