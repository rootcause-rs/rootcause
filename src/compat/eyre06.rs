//! Bidirectional integration with the [`eyre`] 0.6.x error handling library.
//!
//! This module specifically supports `eyre` version 0.6.x. To enable this
//! integration, add the `compat-eyre06` feature flag to your `Cargo.toml`.
//!
//! # Overview
//!
//! This module provides seamless interoperability between rootcause [`Report`]s
//! and [`eyre::Report`], supporting conversions in both directions. This is
//! useful when:
//! - Migrating from eyre to rootcause incrementally
//! - Working with libraries that use eyre for error handling
//! - Integrating rootcause into an existing eyre-based codebase
//! - Calling eyre-based APIs from rootcause code (and vice versa)
//!
//! Conversions preserve error information and formatting, allowing you to mix
//! both error handling approaches seamlessly.
//!
//! # Converting from Eyre to Rootcause
//!
//! Use the [`IntoRootcause`] trait to convert eyre
//! reports into rootcause reports:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! fn eyre_function() -> eyre::Result<String> {
//!     eyre::bail!("something went wrong");
//! }
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     // Convert eyre::Result to Result<T, Report>
//!     let value = eyre_function().into_rootcause()?;
//!     Ok(value)
//! }
//! ```
//!
//! You can also convert individual [`eyre::Report`] values:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! let eyre_error: eyre::Report = eyre::eyre!("failed to connect");
//! let report: Report = eyre_error.into_rootcause();
//!
//! // The report preserves eyre's formatting
//! println!("{}", report);
//! ```
//!
//! # Converting from Rootcause to Eyre
//!
//! Use the [`IntoEyre`] trait to convert reports into eyre reports:
//!
//! ```
//! use rootcause::{compat::eyre06::IntoEyre, prelude::*};
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(report!("database connection failed"))
//! }
//!
//! fn eyre_function() -> eyre::Result<String> {
//!     // Convert Result<T, Report> to eyre::Result<T>
//!     let value = rootcause_function().into_eyre()?;
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
//! let eyre_error: eyre::Report = report.into();
//!
//! // The eyre report preserves the report's formatting
//! println!("{}", eyre_error);
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
//! fn eyre_function() -> eyre::Result<String> {
//!     // The ? operator automatically converts Report to eyre::Report
//!     rootcause_function()?;
//!     Ok("success".to_string())
//! }
//! ```
//!
//! # Using Eyre's Context Trait
//!
//! **Note:** You cannot use [`eyre::WrapErr`] directly on `Result<T, Report>`
//! because [`Report`] does not implement [`core::error::Error`]. To use
//! eyre's `.wrap_err()` method with rootcause reports, you need to convert to
//! eyre first:
//!
//! ```
//! use eyre::WrapErr;
//! use rootcause::{Report, compat::eyre06::IntoEyre};
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(rootcause::report!("connection failed"))
//! }
//!
//! fn eyre_function() -> eyre::Result<String> {
//!     // First convert to eyre, then use eyre's .wrap_err()
//!     let value = rootcause_function()
//!         .into_eyre()
//!         .wrap_err("Failed to fetch data")?;
//!     Ok(value)
//! }
//! ```
//!
//! # Handler Behavior
//!
//! The [`EyreHandler`] delegates to eyre's own formatting implementation,
//! ensuring that converted reports display exactly as they would in pure eyre
//! code. This includes:
//! - Display formatting via [`eyre::Report`]'s `Display` implementation
//! - Debug formatting via [`eyre::Report`]'s `Debug` implementation
//! - Source chain navigation via [`eyre::Report`]'s `source` method
//!
//! When converting from rootcause to eyre, the entire [`Report`] structure
//! (including all contexts and attachments) is preserved and formatted
//! according to rootcause's formatting rules.

use rootcause_internals::handlers::{ContextFormattingStyle, ContextHandler, FormattingFunction};

use super::IntoRootcause;
use crate::{Report, compat::ReportAsError, markers};

/// A custom handler for [`eyre::Report`] that delegates to eyre's own
/// formatting.
///
/// This handler ensures that [`eyre::Report`] objects display identically
/// whether they're used directly or wrapped in a rootcause [`Report`]. You
/// typically don't need to use this handler directly - it's used automatically
/// by the [`IntoRootcause`] trait.
///
/// # Implementation Details
///
/// - **Display**: Uses [`eyre::Report`]'s `Display` implementation
/// - **Debug**: Uses [`eyre::Report`]'s `Debug` implementation
/// - **Source**: Uses [`eyre::Report`]'s `source` method to traverse the error
///   chain
/// - **Formatting style**: Matches the report's formatting function (Display or
///   Debug)
#[derive(Copy, Clone, Debug)]
pub struct EyreHandler;

impl ContextHandler<eyre::Report> for EyreHandler {
    fn source(eyre_error: &eyre::Report) -> Option<&(dyn core::error::Error + 'static)> {
        eyre_error.source()
    }

    fn display(
        eyre_error: &eyre::Report,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Display::fmt(eyre_error, formatter)
    }

    fn debug(
        eyre_error: &eyre::Report,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(eyre_error, formatter)
    }

    fn preferred_formatting_style(
        _value: &eyre::Report,
        formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: formatting_function,
        }
    }
}

impl IntoRootcause for eyre::Report {
    type Output = Report;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        Report::new_custom::<EyreHandler>(self).into_dyn_any()
    }
}

impl<T> IntoRootcause for eyre::Result<T> {
    type Output = Result<T, Report>;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        self.map_err(|e| e.into_rootcause())
    }
}

/// A trait for converting rootcause [`Report`]s into [`eyre::Report`].
///
/// This trait provides the `.into_eyre()` method for converting rootcause
/// reports into eyre reports. It's implemented for both [`Report`] and
/// [`Result<T, Report>`], making it easy to call eyre-based APIs from
/// rootcause code.
///
/// The conversion wraps the entire report structure inside an
/// [`eyre::Report`], preserving all contexts, attachments, and formatting
/// behavior.
///
/// # Examples
///
/// ## Converting a Result
///
/// ```
/// use rootcause::{compat::eyre06::IntoEyre, prelude::*};
///
/// fn uses_rootcause() -> Result<i32, Report> {
///     Err(report!("failed"))
/// }
///
/// fn uses_eyre() -> eyre::Result<i32> {
///     let value = uses_rootcause().into_eyre()?;
///     Ok(value)
/// }
/// ```
///
/// ## Converting a Report
///
/// ```
/// use rootcause::{compat::eyre06::IntoEyre, prelude::*};
///
/// let report = report!("operation failed").attach("debug info");
/// let eyre_err: eyre::Report = report.into_eyre();
///
/// // The eyre report displays the full report structure
/// println!("{}", eyre_err);
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
/// let eyre_err: eyre::Report = report.into();
/// ```
pub trait IntoEyre {
    /// The type produced by the conversion.
    ///
    /// - For [`Report`]: produces [`eyre::Report`]
    /// - For [`Result<T, Report>`]: produces [`eyre::Result<T>`]
    type Output;

    /// Converts this value into an eyre type.
    ///
    /// For [`Report`], this wraps the report in an [`eyre::Report`]. For
    /// [`Result<T, Report>`], this converts the error variant while preserving
    /// the success value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{compat::eyre06::IntoEyre, prelude::*};
    ///
    /// // Convert a result
    /// let result: Result<i32, Report> = Ok(42);
    /// let converted: eyre::Result<i32> = result.into_eyre();
    /// assert_eq!(converted.unwrap(), 42);
    ///
    /// // Convert a report
    /// let report: Report = report!("failed");
    /// let eyre_err: eyre::Report = report.into_eyre();
    /// ```
    fn into_eyre(self) -> Self::Output;
}

impl<C: ?Sized, O> IntoEyre for Report<C, O> {
    type Output = eyre::Report;

    fn into_eyre(self) -> Self::Output {
        eyre::Report::from(self)
    }
}

impl<T, C: ?Sized, O> IntoEyre for Result<T, Report<C, O>> {
    type Output = Result<T, eyre::Report>;

    fn into_eyre(self) -> Self::Output {
        self.map_err(|r| r.into_eyre())
    }
}

impl<C: ?Sized, O> From<Report<C, O, markers::SendSync>> for eyre::Report {
    fn from(report: Report<C, O, markers::SendSync>) -> Self {
        eyre::Report::new(ReportAsError(report.into_dyn_any().into_cloneable()))
    }
}
