//! Bidirectional integration with the [`error-stack`] 0.6.x error handling
//! library.
//!
//! This module specifically supports `error-stack` version 0.6.x. To enable
//! this integration, add the `compat-error-stack06` feature flag to your
//! `Cargo.toml`.
//!
//! # Overview
//!
//! This module provides seamless interoperability between rootcause [`Report`]s
//! and [`error_stack::Report`], supporting conversions in both directions.
//! This is useful when:
//! - Migrating from error-stack to rootcause incrementally
//! - Working with libraries that use error-stack for error handling
//! - Integrating rootcause into an existing error-stack-based codebase
//! - Calling error-stack-based APIs from rootcause code (and vice versa)
//!
//! Conversions preserve error information and formatting, allowing you to mix
//! both error handling approaches seamlessly.
//!
//! # Converting from error-stack to Rootcause
//!
//! Use the [`IntoRootcause`] trait to convert error-stack
//! reports into rootcause reports:
//!
//! ```
//! use std::io;
//!
//! use rootcause::prelude::*;
//!
//! fn error_stack_function() -> Result<String, error_stack::Report<io::Error>> {
//!     Err(error_stack::report!(io::Error::from(
//!         io::ErrorKind::NotFound
//!     )))
//! }
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     // Convert error_stack result to rootcause result
//!     let value = error_stack_function().into_rootcause()?;
//!     Ok(value)
//! }
//! ```
//!
//! You can also convert individual [`error_stack::Report`] values:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! let es_report = error_stack::report!(std::io::Error::from(std::io::ErrorKind::NotFound));
//! let report: Report<_> = es_report.into_rootcause();
//!
//! // The report preserves error-stack's formatting
//! println!("{}", report);
//! ```
//!
//! # Converting from Rootcause to error-stack
//!
//! Use the [`IntoErrorStack`] trait to convert rootcause reports into
//! error-stack reports:
//!
//! ```
//! use rootcause::{compat::error_stack06::IntoErrorStack, prelude::*};
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(report!("database connection failed"))
//! }
//!
//! // The ? operator automatically converts Result<T, Report> to Result<T, error_stack::Report>
//! fn error_stack_function()
//! -> Result<String, error_stack::Report<rootcause::compat::ReportAsError>> {
//!     rootcause_function().into_error_stack()?;
//!     Ok("success".to_string())
//! }
//! ```
//!
//! You can also convert individual [`Report`] values:
//!
//! ```
//! use rootcause::{compat::error_stack06::IntoErrorStack, prelude::*};
//!
//! let report = report!("operation failed").attach("debug info");
//! let es_report: error_stack::Report<_> = report.into_error_stack();
//!
//! // The error-stack report preserves the report's formatting
//! println!("{}", es_report);
//! ```
//!
//! Or using the `From` trait directly:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! let report: Report = report!("operation failed");
//! let es_report: error_stack::Report<_> = report.into();
//!
//! // The error-stack report preserves the report's formatting
//! println!("{}", es_report);
//! ```
//!
//! The `From` trait also works with the `?` operator for automatic conversion:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(report!("something failed"))
//! }
//!
//! fn error_stack_function()
//! -> Result<String, error_stack::Report<rootcause::compat::ReportAsError>> {
//!     // The ? operator automatically converts Report to error_stack::Report
//!     rootcause_function()?;
//!     Ok("success".to_string())
//! }
//! ```
//!
//! # Handler Behavior
//!
//! The [`ErrorStackHandler`] delegates to error-stack's own formatting
//! implementation, ensuring that converted reports display exactly as they
//! would in pure error-stack code. This includes:
//! - Display formatting via [`error_stack::Report`]'s `Display` implementation
//! - Debug formatting via [`error_stack::Report`]'s `Debug` implementation
//! - Source chain navigation via the context's `source` method
//!
//! When converting from rootcause to error-stack, the entire [`Report`]
//! structure (including all contexts and attachments) is preserved and
//! formatted according to rootcause's formatting rules.
//!
//! [`error-stack`]: error_stack

use core::marker::PhantomData;

use rootcause_internals::handlers::ContextHandler;

use crate::{
    Report,
    compat::{IntoRootcause, ReportAsError},
    markers::{self, SendSync},
};

/// A custom handler for [`error_stack::Report`] that delegates to
/// error-stack's own formatting.
///
/// This handler ensures that [`error_stack::Report`] objects display
/// identically whether they're used directly or wrapped in a rootcause
/// [`Report`]. You typically don't need to use this handler directly - it's
/// used automatically by the [`IntoRootcause`] trait.
///
/// # Implementation Details
///
/// - **Display**: Uses [`error_stack::Report`]'s `Display` implementation
/// - **Debug**: Uses [`error_stack::Report`]'s `Debug` implementation
/// - **Source**: Uses the current context's `source` method to traverse the
///   error chain
/// - **Formatting style**: Matches the report's formatting function (Display or
///   Debug)
///
/// # Examples
///
/// ```
/// use rootcause::{Report, compat::error_stack06::ErrorStackHandler};
///
/// let es_report = error_stack::Report::new(std::io::Error::from(std::io::ErrorKind::NotFound));
/// let report: Report<_> = Report::new_custom::<ErrorStackHandler<_>>(es_report);
/// ```
///
/// # Type Parameters
///
/// - `C`: The context type of the error-stack report, which must implement
///   `Error + Send + Sync + 'static`
pub struct ErrorStackHandler<C>(PhantomData<C>);

impl<C> ContextHandler<error_stack::Report<C>> for ErrorStackHandler<C>
where
    C: core::error::Error + Send + Sync + 'static,
{
    fn source(value: &error_stack::Report<C>) -> Option<&(dyn core::error::Error + 'static)> {
        value.current_context().source()
    }

    fn display(
        value: &error_stack::Report<C>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Display::fmt(value, formatter)
    }

    fn debug(
        value: &error_stack::Report<C>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(value, formatter)
    }
}

impl<C> IntoRootcause for error_stack::Report<C>
where
    C: core::error::Error + Send + Sync + 'static,
{
    type Output = crate::Report<Self>;

    fn into_rootcause(self) -> Self::Output {
        Report::new_custom::<ErrorStackHandler<C>>(self)
    }
}

impl<T, C> IntoRootcause for Result<T, error_stack::Report<C>>
where
    C: core::error::Error + Send + Sync + 'static,
{
    type Output = Result<T, Report<error_stack::Report<C>>>;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        self.map_err(|e| e.into_rootcause())
    }
}

/// A trait for converting rootcause [`Report`]s into [`error_stack::Report`].
///
/// This trait provides the `.into_error_stack()` method for converting
/// rootcause reports into error-stack reports. It's implemented for both
/// [`Report`] and [`Result<T, Report>`], making it easy to call
/// error-stack-based APIs from rootcause code.
///
/// The conversion wraps the entire report structure inside an
/// [`error_stack::Report`], preserving all contexts, attachments, and
/// formatting behavior.
///
/// # Examples
///
/// ## Converting a Result
///
/// ```
/// use rootcause::{compat::error_stack06::IntoErrorStack, prelude::*};
///
/// fn uses_rootcause() -> Result<i32, Report> {
///     Err(report!("failed"))
/// }
///
/// fn uses_error_stack() -> Result<i32, error_stack::Report<rootcause::compat::ReportAsError>> {
///     let value = uses_rootcause().into_error_stack()?;
///     Ok(value)
/// }
/// ```
///
/// ## Converting a Report
///
/// ```
/// use rootcause::{compat::error_stack06::IntoErrorStack, prelude::*};
///
/// let report = report!("operation failed").attach("debug info");
/// let es_report: error_stack::Report<_> = report.into_error_stack();
///
/// // The error-stack report displays the full rootcause structure
/// println!("{}", es_report);
/// ```
///
/// ## Using `From` Instead
///
/// You can also use the `From` trait for explicit conversions:
///
/// ```
/// use rootcause::prelude::*;
///
/// let report: Report = report!("error");
/// let es_report: error_stack::Report<_> = report.into();
/// ```
///
/// # Type Parameters
///
/// - `C`: The context type parameter of the rootcause report. When converting,
///   the report will be wrapped as an
///   [`error_stack::Report<ReportAsError<C>>`].
pub trait IntoErrorStack<C: ?Sized> {
    /// The type produced by the conversion.
    ///
    /// - For [`Report`]: produces
    ///   [`error_stack::Report<ReportAsError<C>>`](error_stack::Report)
    /// - For [`Result<T, Report>`]: produces `Result<T,
    ///   error_stack::Report<ReportAsError<C>>>`
    type Output;

    /// Converts this value into an error-stack type.
    ///
    /// For [`Report`], this wraps the report in an error-stack report using
    /// [`ReportAsError`] as the context type. For [`Result<T, Report>`], this
    /// converts the error variant while preserving the success value.
    ///
    /// The report is wrapped in a [`ReportAsError`] adapter that implements
    /// [`core::error::Error`], allowing it to be used as the context type in
    /// an error-stack report.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{compat::error_stack06::IntoErrorStack, prelude::*};
    ///
    /// // Convert a result
    /// let result: Result<i32, Report> = Ok(42);
    /// let converted: Result<i32, error_stack::Report<_>> = result.into_error_stack();
    /// assert_eq!(converted.unwrap(), 42);
    ///
    /// // Convert a report
    /// let report: Report = report!("failed");
    /// let es_report: error_stack::Report<_> = report.into_error_stack();
    /// ```
    fn into_error_stack(self) -> Self::Output;
}

impl<C: ?Sized, O> IntoErrorStack<C> for Report<C, O, SendSync> {
    type Output = error_stack::Report<ReportAsError<C>>;

    fn into_error_stack(self) -> Self::Output {
        error_stack::Report::from(self)
    }
}

impl<T, C: ?Sized, O> IntoErrorStack<C> for Result<T, Report<C, O, SendSync>> {
    type Output = Result<T, error_stack::Report<ReportAsError<C>>>;

    fn into_error_stack(self) -> Self::Output {
        self.map_err(|e| e.into_error_stack())
    }
}

impl<C: ?Sized, O> From<Report<C, O, markers::SendSync>> for error_stack::Report<ReportAsError<C>> {
    fn from(report: Report<C, O, markers::SendSync>) -> Self {
        error_stack::Report::from(ReportAsError(report.into_cloneable()))
    }
}
