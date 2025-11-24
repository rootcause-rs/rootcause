//! Convert rootcause [`Report`]s into boxed error trait objects.
//!
//! # Overview
//!
//! This module provides conversion utilities for transforming rootcause
//! [`Report`]s into standard Rust error trait objects (`Box<dyn Error>`). This
//! is useful when:
//! - Integrating with APIs that expect `Box<dyn Error>`
//! - Converting to standardized error types for external consumption
//! - Working with error handling patterns that use trait objects
//! - Providing a uniform error interface across different error types
//!
//! The conversion preserves the report's formatting and error information while
//! wrapping it in a standard error trait object that can be used with any code
//! expecting `Box<dyn Error>`.
//!
//! # Converting from Rootcause to Boxed Errors
//!
//! Use the [`IntoBoxedError`] trait to convert reports into boxed error trait
//! objects:
//!
//! ```
//! use rootcause::prelude::*;
//! use std::error::Error;
//!
//! fn rootcause_function() -> Result<String, Report> {
//!     Err(report!("database connection failed"))
//! }
//!
//! fn boxed_error_function() -> Result<String, Box<dyn Error + Send + Sync>> {
//!     // Convert Result<T, Report> to Result<T, Box<dyn Error + Send + Sync>>
//!     let value = rootcause_function().into_boxed_error()?;
//!     Ok(value)
//! }
//! ```
//!
//! You can also convert individual [`Report`] values:
//!
//! ```
//! use rootcause::prelude::*;
//! use std::error::Error;
//!
//! let report: Report = report!("operation failed").attach("debug info");
//! let boxed_err: Box<dyn Error + Send + Sync> = report.into_boxed_error();
//!
//! // The boxed error preserves the report's formatting
//! println!("{}", boxed_err);
//! ```
//!
//! # Thread Safety
//!
//! The type of boxed error depends on the thread safety marker:
//! - [`SendSync`] reports convert to `Box<dyn Error + Send + Sync>`
//! - [`Local`] reports convert to `Box<dyn Error>`
//!
//! This ensures that the resulting boxed error respects the thread safety
//! constraints of the original report.
//!
//! ```
//! use rootcause::{prelude::*, markers::Local};
//! use std::{error::Error, rc::Rc};
//!
//! // SendSync report becomes Box<dyn Error + Send + Sync>
//! let send_sync_report: Report = report!("network error");
//! let send_sync_boxed: Box<dyn Error + Send + Sync> = send_sync_report.into_boxed_error();
//!
//! // Local report becomes Box<dyn Error>
//! let local_report: Report<_, _, Local> = report!("local error").into_local().attach(Rc::new("data"));
//! let local_boxed: Box<dyn Error> = local_report.into_boxed_error();
//! ```
//!
//! # Converting from Boxed Errors to Rootcause
//!
//! Use the [`IntoRootcause`] trait to convert boxed errors into reports:
//!
//! ```
//! use rootcause::prelude::*;
//! use std::error::Error;
//!
//! fn uses_boxed_error() -> Result<String, Box<dyn Error + Send + Sync>> {
//!     Err("something failed".into())
//! }
//!
//! fn uses_rootcause() -> Result<String, Report> {
//!     // Convert to rootcause report
//!     let value = uses_boxed_error().into_rootcause()?;
//!     Ok(value)
//! }
//! ```
//!
//! # Using `From` Trait
//!
//! The `From` trait is also implemented for direct conversions:
//!
//! ```
//! use rootcause::prelude::*;
//! use std::error::Error;
//!
//! let report: Report = report!("failed");
//! let boxed_err: Box<dyn Error + Send + Sync> = report.into();
//!
//! // The ? operator works automatically
//! fn convert_automatically() -> Result<(), Box<dyn Error + Send + Sync>> {
//!     let _report: Report = report!("error");
//!     // Automatic conversion via ?
//!     Err(_report)?
//! }
//! ```

use super::{IntoRootcause, ReportAsError};
use crate::{
    Report,
    markers::{self, Local, SendSync},
};
use alloc::boxed::Box;
use core::{any::Any, error::Error};
use rootcause_internals::handlers::{ContextFormattingStyle, ContextHandler, FormattingFunction};

/// A custom handler for boxed error trait objects that delegates to the
/// underlying error's formatting.
///
/// This handler ensures that boxed errors (`Box<dyn Error>` and `Box<dyn Error
/// + Send + Sync>`) display identically whether they're used directly or
/// wrapped in a rootcause [`Report`]. You typically don't need to use this
/// handler directly - it's used automatically by the [`IntoRootcause`] trait.
///
/// # Implementation Details
///
/// - **Display**: Uses the boxed error's `Display` implementation
/// - **Debug**: Uses the boxed error's `Debug` implementation
/// - **Source**: Uses the boxed error's `source` method to traverse the error
///   chain
/// - **Formatting style**: Matches the report's formatting function (Display or
///   Debug)
#[derive(Copy, Clone, Debug)]
pub struct BoxedErrorHandler;

impl ContextHandler<Box<dyn Error + Send + Sync>> for BoxedErrorHandler {
    fn source(boxed_error: &Box<dyn Error + Send + Sync>) -> Option<&(dyn Error + 'static)> {
        boxed_error.source()
    }

    fn display(
        boxed_error: &Box<dyn Error + Send + Sync>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Display::fmt(boxed_error, formatter)
    }

    fn debug(
        boxed_error: &Box<dyn Error + Send + Sync>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(boxed_error, formatter)
    }

    fn preferred_formatting_style(
        _value: &Box<dyn Error + Send + Sync>,
        formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: formatting_function,
        }
    }
}

impl ContextHandler<Box<dyn Error>> for BoxedErrorHandler {
    fn source(boxed_error: &Box<dyn Error>) -> Option<&(dyn Error + 'static)> {
        boxed_error.source()
    }

    fn display(
        boxed_error: &Box<dyn Error>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Display::fmt(boxed_error, formatter)
    }

    fn debug(
        boxed_error: &Box<dyn Error>,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(boxed_error, formatter)
    }

    fn preferred_formatting_style(
        _value: &Box<dyn Error>,
        formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: formatting_function,
        }
    }
}

/// A trait for converting rootcause [`Report`]s into boxed error trait objects.
///
/// This trait provides the `.into_boxed_error()` method for converting
/// rootcause reports into standard Rust error trait objects. It's implemented
/// for both [`Report`] and [`Result<T, Report>`], making it easy to integrate
/// with APIs that expect `Box<dyn Error>`.
///
/// The specific type of boxed error depends on the thread safety marker:
/// - [`SendSync`] reports convert to `Box<dyn Error + Send + Sync>`
/// - [`Local`] reports convert to `Box<dyn Error>`
///
/// # Examples
///
/// ## Converting a Result with SendSync Report
///
/// ```
/// use rootcause::prelude::*;
/// use std::error::Error;
///
/// fn uses_rootcause() -> Result<i32, Report> {
///     Err(report!("failed"))
/// }
///
/// fn uses_boxed_error() -> Result<i32, Box<dyn Error + Send + Sync>> {
///     let value = uses_rootcause().into_boxed_error()?;
///     Ok(value)
/// }
/// ```
///
/// ## Converting a Local Report
///
/// ```
/// use rootcause::{prelude::*, markers::Local};
/// use std::{error::Error, rc::Rc};
///
/// let local_report: Report<_, _, Local> = report!("error").into_local().attach(Rc::new("data"));
/// let boxed_err: Box<dyn Error> = local_report.into_boxed_error();
///
/// // The boxed error displays the full report structure
/// println!("{}", boxed_err);
/// ```
///
/// ## Using `From` Instead
///
/// You can also use the `From` trait for explicit conversions:
///
/// ```
/// use rootcause::prelude::*;
/// use std::error::Error;
///
/// let report: Report = report!("error");
/// let boxed_err: Box<dyn Error + Send + Sync> = report.into();
/// ```
pub trait IntoBoxedError {
    /// The type produced by the conversion.
    ///
    /// - For [`Report<_, _, SendSync>`]: produces `Box<dyn Error + Send + Sync>`
    /// - For [`Report<_, _, Local>`]: produces `Box<dyn Error>`
    /// - For [`Result<T, Report<_, _, SendSync>>`]: produces `Result<T, Box<dyn Error + Send + Sync>>`
    /// - For [`Result<T, Report<_, _, Local>>`]: produces `Result<T, Box<dyn Error>>`
    type Output;

    /// Converts this value into a boxed error type.
    ///
    /// For [`Report`], this wraps the report in a boxed error trait object. For
    /// [`Result<T, Report>`], this converts the error variant while preserving
    /// the success value.
    ///
    /// The thread safety of the resulting boxed error matches the thread safety
    /// of the original report.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::prelude::*;
    /// use std::error::Error;
    ///
    /// // Convert a result
    /// let result: Result<i32, Report> = Ok(42);
    /// let converted: Result<i32, Box<dyn Error + Send + Sync>> = result.into_boxed_error();
    /// assert_eq!(converted.unwrap(), 42);
    ///
    /// // Convert a report
    /// let report: Report = report!("failed");
    /// let boxed_err: Box<dyn Error + Send + Sync> = report.into_boxed_error();
    /// ```
    fn into_boxed_error(self) -> Self::Output;
}

impl<C: ?Sized, O> IntoBoxedError for Report<C, O, SendSync> {
    type Output = Box<dyn Error + Send + Sync>;

    fn into_boxed_error(self) -> Self::Output {
        Box::new(ReportAsError(self.into_dyn_any().into_cloneable()))
    }
}

impl<C: ?Sized, O> IntoBoxedError for Report<C, O, Local> {
    type Output = Box<dyn Error>;

    fn into_boxed_error(self) -> Self::Output {
        Box::new(ReportAsError(self.into_dyn_any().into_cloneable()))
    }
}

impl<T, C: ?Sized, O> IntoBoxedError for Result<T, Report<C, O, SendSync>> {
    type Output = Result<T, Box<dyn Error + Send + Sync>>;

    fn into_boxed_error(self) -> Self::Output {
        self.map_err(|r| r.into_boxed_error())
    }
}

impl<T, C: ?Sized, O> IntoBoxedError for Result<T, Report<C, O, Local>> {
    type Output = Result<T, Box<dyn Error>>;

    fn into_boxed_error(self) -> Self::Output {
        self.map_err(|r| r.into_boxed_error())
    }
}

impl<C: ?Sized, O> From<Report<C, O, SendSync>> for Box<dyn Error + Send + Sync> {
    fn from(report: Report<C, O, SendSync>) -> Self {
        report.into_boxed_error()
    }
}

impl<C: ?Sized, O> From<Report<C, O, Local>> for Box<dyn Error> {
    fn from(report: Report<C, O, Local>) -> Self {
        report.into_boxed_error()
    }
}

impl IntoRootcause for Box<dyn Error + Send + Sync> {
    type Output = Report;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        Report::new_custom::<BoxedErrorHandler>(self).into_dyn_any()
    }
}

impl IntoRootcause for Box<dyn Error> {
    type Output = Report<dyn Any, markers::Mutable, Local>;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        Report::new_custom::<BoxedErrorHandler>(self).into_dyn_any()
    }
}

impl<T> IntoRootcause for Result<T, Box<dyn Error + Send + Sync>> {
    type Output = Result<T, Report>;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        self.map_err(|e| e.into_rootcause())
    }
}

impl<T> IntoRootcause for Result<T, Box<dyn Error>> {
    type Output = Result<T, Report<dyn Any, markers::Mutable, Local>>;

    #[inline(always)]
    fn into_rootcause(self) -> Self::Output {
        self.map_err(|e| e.into_rootcause())
    }
}
