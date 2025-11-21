//! Compatibility and interoperability with other error handling libraries.
//!
//! # Overview
//!
//! This module provides integration with popular error handling libraries in
//! the Rust ecosystem, enabling seamless interoperability and gradual migration
//! paths. Each submodule offers bidirectional conversion traits and utilities
//! for working with rootcause alongside other error handling approaches.
//!
//! # Available Integrations
//!
//! - [`anyhow`] - Integration with the `anyhow` error handling library
//!   (requires the `anyhow` feature flag)
//! - [`error_stack`] - Integration with the `error-stack` error handling library
//!   (requires the `error-stack` feature flag)
//! - [`eyre`] - Integration with the `eyre` error handling library
//!   (requires the `eyre` feature flag)
//!
//! # When to Use Compatibility Modules
//!
//! These compatibility modules are useful when:
//! - **Migrating incrementally**: Gradually adopt rootcause in an existing
//!   codebase without rewriting everything at once
//! - **Interoperating with dependencies**: Call libraries that use different
//!   error handling approaches from your rootcause-based code
//! - **Mixed codebases**: Work in projects where different parts use different
//!   error handling strategies
//! - **Evaluating rootcause**: Try out rootcause features while maintaining
//!   compatibility with your existing error handling
//!
//! # Design Philosophy
//!
//! Each compatibility module aims to provide:
//! - **Bidirectional conversions**: Convert errors in both directions to support
//!   flexible integration patterns
//! - **Information preservation**: Maintain error context and formatting across
//!   conversions where possible
//! - **Ergonomic APIs**: Use familiar Rust patterns like `From`/`Into` traits
//!   and extension traits with descriptive method names
//!
//! # Example
//!
//! Here's how to use the [`IntoRootcause`] trait to convert external errors:
//!
//! ```
//! use rootcause::prelude::*;
//!
//! # #[cfg(feature = "anyhow")] {
//! // Call an anyhow-based function from rootcause code
//! fn legacy_function() -> anyhow::Result<String> {
//!     anyhow::bail!("something went wrong");
//! }
//!
//! fn new_function() -> Result<String, Report> {
//!     // Convert seamlessly using .into_rootcause()
//!     let value = legacy_function().into_rootcause()?;
//!     Ok(value)
//! }
//! # }
//! ```
//!
//! The [`IntoRootcause`] trait is available for all supported external error
//! types, making it easy to integrate them with rootcause.
//!
//! See the individual module documentation for detailed integration guides and
//! migration strategies.

use crate::{Report, markers};

/// A trait for converting external error types into rootcause [`Report`]s.
///
/// This trait provides a standardized way to convert errors from other error
/// handling libraries into rootcause reports. It's designed to be implemented
/// by compatibility modules for different error handling libraries.
///
/// The trait provides the `.into_rootcause()` method, which can convert both
/// individual error values and `Result` types. This makes it easy to integrate
/// external error types with rootcause's error handling.
///
/// # When to Use
///
/// Use this trait when you need to:
/// - Call functions that return errors from other libraries while using
///   rootcause for your own error handling
/// - Migrate incrementally from another error handling approach to rootcause
/// - Integrate with dependencies that use different error handling libraries
///
/// # Implementations
///
/// This trait is implemented by compatibility modules for specific error
/// handling libraries:
/// - [`anyhow`] module provides implementations for [`anyhow::Error`] and
///   [`anyhow::Result<T>`]
/// - [`error_stack`] module provides implementations for
///   [`error_stack::Report<C>`] and `Result<T, error_stack::Report<C>>`
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
///
/// # #[cfg(feature = "anyhow")] {
/// // Converting an anyhow Result to a rootcause Result
/// fn uses_anyhow() -> anyhow::Result<i32> {
///     Ok(42)
/// }
///
/// fn uses_rootcause() -> Result<i32, Report> {
///     let value = uses_anyhow().into_rootcause()?;
///     Ok(value)
/// }
/// # }
/// ```
///
/// ```
/// use rootcause::prelude::*;
///
/// # #[cfg(feature = "anyhow")] {
/// // Converting an individual error
/// let external_error = anyhow::anyhow!("database connection failed");
/// let report: Report = external_error.into_rootcause();
/// # }
/// ```
pub trait IntoRootcause {
    /// The type produced by the conversion.
    ///
    /// For error types, this is typically [`Report`]. For `Result` types, this
    /// is typically `Result<T, Report>`.
    type Output;

    /// Converts this value into a rootcause type.
    ///
    /// The specific behavior depends on the type being converted:
    /// - For error types: wraps the error in a [`Report`]
    /// - For `Result` types: converts the error variant while preserving the
    ///   success value
    fn into_rootcause(self) -> Self::Output;
}

#[cfg(feature = "anyhow")]
pub mod anyhow;

#[cfg(feature = "error-stack")]
pub mod error_stack;

#[cfg(feature = "eyre")]
pub mod eyre;

/// A wrapper that adapts a rootcause [`Report`] to implement
/// [`core::error::Error`].
///
/// This type is used internally by compatibility modules to convert rootcause
/// reports into error types that can be used with external error handling
/// libraries. It wraps a cloneable, sendable report and delegates formatting
/// and error trait implementations to the underlying report.
///
/// You typically don't need to use this type directly - it's used automatically
/// by conversion traits like [`anyhow::IntoAnyhow`] and
/// [`error_stack::IntoErrorStack`].
///
/// # Type Parameters
///
/// - `C`: The context type of the wrapped report
pub struct ReportAsError<C>(Report<C, markers::Cloneable, markers::SendSync>)
where
    C: markers::ObjectMarker + ?Sized;

impl<C> Clone for ReportAsError<C>
where
    C: markers::ObjectMarker + ?Sized,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<C> core::fmt::Debug for ReportAsError<C>
where
    C: markers::ObjectMarker + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

impl<C> core::fmt::Display for ReportAsError<C>
where
    C: markers::ObjectMarker + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}

impl<C> core::error::Error for ReportAsError<C> where C: markers::ObjectMarker + ?Sized {}
