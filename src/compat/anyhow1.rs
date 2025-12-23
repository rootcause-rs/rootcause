//! Bidirectional integration with the [`anyhow`] 1.x error handling library.
//!
//! This module specifically supports `anyhow` version 1.x. To enable this
//! integration, add the `compat-anyhow1` feature flag to your `Cargo.toml`.
//!
//! # Overview
//!
//! This module provides seamless interoperability between rootcause [`Report`]s
//! and [`anyhow::Error`], supporting conversions in both directions. This is
//! useful when:
//! - Migrating from anyhow to rootcause incrementally
//! - Working with libraries that use anyhow for error handling
//! - Integrating rootcause into an existing anyhow-based codebase
//! - Calling anyhow-based APIs from rootcause code (and vice versa)
//!
//! Conversions preserve error information and formatting, allowing you to mix
//! both error handling approaches seamlessly.
//!
//! # Converting from Anyhow to Rootcause
//!
//! Use the [`IntoRootcause`] trait to convert anyhow
//! errors into reports:
//!
//! ```
//! use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};
//!
//! fn anyhow_function() -> anyhow::Result<String> {
//!     anyhow::bail!("something went wrong");
//! }
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     // Convert anyhow::Result to Result<T, Report>
//!     let value = anyhow_function().into_rootcause()?;
//!     Ok(value)
//! }
//! ```
//!
//! You can also convert individual [`anyhow::Error`] values:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! let anyhow_error: anyhow::Error = anyhow::anyhow!("failed to connect");
//! let report: Report = anyhow_error.into_rootcause();
//!
//! // The report preserves anyhow's formatting
//! println!("{}", report);
//! ```
//!
//! # Converting from Rootcause to Anyhow
//!
//! Use the [`IntoAnyhow`] trait to convert reports into anyhow errors:
//!
//! ```
//! use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(report!("database connection failed"))
//! }
//!
//! fn anyhow_function() -> anyhow::Result<String> {
//!     // Convert Result<T, Report> to anyhow::Result<T>
//!     let value = rootcause_function().into_anyhow()?;
//!     Ok(value)
//! }
//! ```
//!
//! Or using the `From` trait directly:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! let report: Report = report!("operation failed");
//! let anyhow_error: anyhow::Error = report.into();
//!
//! // The anyhow error preserves the report's formatting
//! println!("{}", anyhow_error);
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
//! fn anyhow_function() -> anyhow::Result<String> {
//!     // The ? operator automatically converts Report to anyhow::Error
//!     rootcause_function()?;
//!     Ok("success".to_string())
//! }
//! ```
//!
//! # Using Anyhow's Context Trait
//!
//! **Note:** You cannot use [`anyhow::Context`] directly on `Result<T, Report>`
//! because [`Report`] does not implement [`core::error::Error`]. To use
//! anyhow's `.context()` method with rootcause reports, you need to convert to
//! anyhow first:
//!
//! ```
//! use anyhow::Context;
//! use rootcause::{Report, compat::anyhow1::IntoAnyhow};
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(rootcause::report!("connection failed"))
//! }
//!
//! fn anyhow_function() -> anyhow::Result<String> {
//!     // First convert to anyhow, then use anyhow's .context()
//!     let value = rootcause_function()
//!         .into_anyhow()
//!         .context("Failed to fetch data")?;
//!     Ok(value)
//! }
//! ```
//!
//! # Handler Behavior
//!
//! The [`AnyhowHandler`] delegates to anyhow's own formatting implementation,
//! ensuring that converted errors display exactly as they would in pure anyhow
//! code. This includes:
//! - Display formatting via [`anyhow::Error`]'s `Display` implementation
//! - Debug formatting via [`anyhow::Error`]'s `Debug` implementation
//! - Source chain navigation via [`anyhow::Error`]'s `source` method
//!
//! When converting from rootcause to anyhow, the entire [`Report`] structure
//! (including all contexts and attachments) is preserved and formatted
//! according to rootcause's formatting rules.

use rootcause_internals::handlers::{ContextFormattingStyle, ContextHandler, FormattingFunction};

use super::IntoRootcause;
use crate::{Report, compat::ReportAsError, markers};

/// A custom handler for [`anyhow::Error`] that delegates to anyhow's own
/// formatting.
///
/// This handler ensures that [`anyhow::Error`] objects display identically
/// whether they're used directly or wrapped in a rootcause [`Report`]. You
/// typically don't need to use this handler directly - it's used automatically
/// by the [`IntoRootcause`] trait.
///
/// # Implementation Details
///
/// - **Display**: Uses [`anyhow::Error`]'s `Display` implementation
/// - **Debug**: Uses [`anyhow::Error`]'s `Debug` implementation
/// - **Source**: Uses [`anyhow::Error`]'s `source` method to traverse the error
///   chain
/// - **Formatting style**: Matches the report's formatting function (Display or
///   Debug)
///
/// # Examples
///
/// ```
/// use rootcause::{Report, compat::anyhow1::AnyhowHandler};
///
/// let anyhow_err = anyhow::anyhow!("operation failed");
/// let report = Report::new_sendsync_custom::<AnyhowHandler>(anyhow_err);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct AnyhowHandler;

impl ContextHandler<anyhow::Error> for AnyhowHandler {
    fn source(anyhow_error: &anyhow::Error) -> Option<&(dyn core::error::Error + 'static)> {
        anyhow_error.source()
    }

    fn display(
        anyhow_error: &anyhow::Error,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Display::fmt(anyhow_error, formatter)
    }

    fn debug(
        anyhow_error: &anyhow::Error,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(anyhow_error, formatter)
    }

    fn preferred_formatting_style(
        _value: &anyhow::Error,
        formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: formatting_function,
        }
    }
}

impl IntoRootcause for anyhow::Error {
    type Output = Report;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        Report::new_custom::<AnyhowHandler>(self).into_dynamic()
    }
}

impl<T> IntoRootcause for anyhow::Result<T> {
    type Output = Result<T, Report>;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        self.map_err(|e| e.into_rootcause())
    }
}

/// A trait for converting rootcause [`Report`]s into [`anyhow::Error`].
///
/// This trait provides the `.into_anyhow()` method for converting rootcause
/// reports into anyhow errors. It's implemented for both [`Report`] and
/// [`Result<T, Report>`], making it easy to call anyhow-based APIs from
/// rootcause code.
///
/// The conversion wraps the entire report structure inside an
/// [`anyhow::Error`], preserving all contexts, attachments, and formatting
/// behavior.
///
/// # Examples
///
/// ## Converting a Result
///
/// ```
/// use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};
///
/// fn uses_rootcause() -> Result<i32, Report> {
///     Err(report!("failed"))
/// }
///
/// fn uses_anyhow() -> anyhow::Result<i32> {
///     let value = uses_rootcause().into_anyhow()?;
///     Ok(value)
/// }
/// ```
///
/// ## Converting a Report
///
/// ```
/// use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};
///
/// let report = report!("operation failed").attach("debug info");
/// let anyhow_err: anyhow::Error = report.into_anyhow();
///
/// // The anyhow error displays the full report structure
/// println!("{}", anyhow_err);
/// ```
///
/// ## Using `From` Instead
///
/// You can also use the `From` trait for explicit conversions:
///
/// ```
/// use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};
///
/// let report: Report = report!("error");
/// let anyhow_err: anyhow::Error = report.into();
/// ```
pub trait IntoAnyhow {
    /// The type produced by the conversion.
    ///
    /// - For [`Report`]: produces [`anyhow::Error`]
    /// - For [`Result<T, Report>`]: produces [`anyhow::Result<T>`]
    type Output;

    /// Converts this value into an anyhow type.
    ///
    /// For [`Report`], this wraps the report in an [`anyhow::Error`]. For
    /// [`Result<T, Report>`], this converts the error variant while preserving
    /// the success value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};
    ///
    /// // Convert a result
    /// let result: Result<i32, Report> = Ok(42);
    /// let converted: anyhow::Result<i32> = result.into_anyhow();
    /// assert_eq!(converted.unwrap(), 42);
    ///
    /// // Convert a report
    /// let report: Report = report!("failed");
    /// let anyhow_err: anyhow::Error = report.into_anyhow();
    /// ```
    fn into_anyhow(self) -> Self::Output;
}

impl<C: ?Sized, O> IntoAnyhow for Report<C, O> {
    type Output = anyhow::Error;

    fn into_anyhow(self) -> Self::Output {
        anyhow::Error::from(self)
    }
}

impl<T, C: ?Sized, O> IntoAnyhow for Result<T, Report<C, O>> {
    type Output = Result<T, anyhow::Error>;

    fn into_anyhow(self) -> Self::Output {
        self.map_err(|r| r.into_anyhow())
    }
}

impl<C: ?Sized, O> From<Report<C, O, markers::SendSync>> for anyhow::Error {
    fn from(report: Report<C, O, markers::SendSync>) -> Self {
        anyhow::Error::new(ReportAsError(report.into_dynamic().into_cloneable()))
    }
}
