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
/// # When to Implement
///
/// Implement `ReportConversion` when you have a standard, reusable way to
/// convert from one error context to another. Common scenarios include:
///
/// - **Wrapping library errors**: Converting `serde_json::Error` to your
///   application's `MyError::Json` variant
/// - **Error hierarchies**: Converting specific errors to more general types
/// - **Domain boundaries**: Translating errors between layers of your
///   application
///
/// For ad-hoc, one-off transformations, consider using the
/// [`context()`](crate::Report::context) method to wrap errors instead of
/// implementing this trait.
///
/// # Examples
///
/// ## Basic Implementation
///
/// ```rust
/// use rootcause::{ReportConversion, markers, prelude::*};
///
/// #[derive(Debug)]
/// enum MyError {
///     ParseError(String),
/// }
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             MyError::ParseError(msg) => write!(f, "Parse error: {}", msg),
///         }
///     }
/// }
///
/// impl std::error::Error for MyError {}
///
/// // Implement conversion from std::num::ParseIntError
/// impl<O, T> ReportConversion<std::num::ParseIntError, O, T> for MyError
/// where
///     MyError: markers::ObjectMarkerFor<T>,
/// {
///     fn convert_report(
///         report: Report<std::num::ParseIntError, O, T>,
///     ) -> Report<Self, markers::Mutable, T> {
///         report.context(MyError::ParseError("Invalid number".to_string()))
///     }
/// }
/// ```
///
/// ## Using the Conversion
///
/// ```rust
/// # use rootcause::{ReportConversion, markers, prelude::*};
/// #
/// # #[derive(Debug)]
/// # enum MyError {
/// #     ParseError(String),
/// # }
/// #
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         match self {
/// #             MyError::ParseError(msg) => write!(f, "Parse error: {}", msg),
/// #         }
/// #     }
/// # }
/// #
/// # impl std::error::Error for MyError {}
/// #
/// # impl<O, T> ReportConversion<std::num::ParseIntError, O, T> for MyError
/// #   where MyError: rootcause::markers::ObjectMarkerFor<T>
/// # {
/// #     fn convert_report(
/// #         report: Report<std::num::ParseIntError, O, T>) -> Report<Self, markers::Mutable, T> {
/// #         report.context(MyError::ParseError("Invalid number".to_string()))
/// #     }
/// # }
/// #
/// fn parse_config(s: &str) -> Result<i32, Report<MyError>> {
///     s.parse::<i32>().context_to() // Uses ReportConversion
/// }
/// ```
pub trait ReportConversion<C: ?Sized, O, T>: ObjectMarkerFor<T> {
    /// Converts a report with context `C` to a report with context `Self`.
    ///
    /// This method receives the source report and should return a new report
    /// with the transformed context. The thread-safety marker `T` is preserved,
    /// while the ownership marker becomes [`Mutable`] for the new report.
    ///
    /// # Implementation Notes
    ///
    /// Implementations typically use one of these report transformation
    /// methods:
    ///
    /// - [`context()`](crate::Report::context) - Wraps the report as a child
    ///   under new context (runs creation hooks)
    /// - [`context_transform()`](crate::Report::context_transform) - Transforms
    ///   context in-place without running creation hooks again
    /// - [`context_transform_nested()`](crate::Report::context_transform_nested) -
    ///   Transforms while preserving the original report as a nested child with
    ///   fresh creation hooks
    ///
    /// # Examples
    ///
    /// Using `context()` to wrap the error:
    ///
    /// ```
    /// use rootcause::{ReportConversion, markers, prelude::*};
    ///
    /// #[derive(Debug)]
    /// enum AppError {
    ///     ParseError(String),
    /// }
    ///
    /// impl std::fmt::Display for AppError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         match self {
    ///             AppError::ParseError(msg) => write!(f, "Parse error: {}", msg),
    ///         }
    ///     }
    /// }
    /// # impl std::error::Error for AppError {}
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
    ///
    /// Using `context_transform()` for in-place transformation:
    ///
    /// ```
    /// use rootcause::{ReportConversion, markers, prelude::*};
    ///
    /// #[derive(Debug)]
    /// enum AppError {
    ///     ParseError(std::num::ParseIntError),
    /// }
    ///
    /// impl std::fmt::Display for AppError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         match self {
    ///             AppError::ParseError(e) => write!(f, "Parse error: {}", e),
    ///         }
    ///     }
    /// }
    /// # impl std::error::Error for AppError {}
    ///
    /// impl<T> ReportConversion<std::num::ParseIntError, markers::Mutable, T> for AppError
    /// where
    ///     AppError: markers::ObjectMarkerFor<T>,
    /// {
    ///     fn convert_report(
    ///         report: Report<std::num::ParseIntError, markers::Mutable, T>,
    ///     ) -> Report<Self, markers::Mutable, T> {
    ///         report.context_transform(AppError::ParseError)
    ///     }
    /// }
    /// ```
    ///
    /// This method is called automatically by
    /// [`context_to`](crate::Report::context_to) and
    /// [`ResultExt::context_to`](crate::result_ext::ResultExt::context_to), so
    /// users rarely invoke it directly.
    fn convert_report(report: Report<C, O, T>) -> Report<Self, Mutable, T>;
}
