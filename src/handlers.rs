//! Handlers that control how errors and attachments are formatted and displayed.
//!
//! Handlers determine how context objects and attachments are formatted when displaying
//! or debugging error reports. The rootcause library provides several built-in handlers
//! that cover common use cases.
//!
//! # What Are Handlers?
//!
//! Handlers are types that implement the [`ContextHandler`] and/or [`AttachmentHandler`]
//! traits. They define how to format your error contexts and attachments, including:
//! - How to display an error context (via [`Display`](core::fmt::Display))
//! - How to debug-format an error context (via [`Debug`](core::fmt::Debug))
//! - How to navigate to the error's source (via [`Error::source`](core::error::Error::source))
//! - How to format attachments when they appear in reports
//! - Formatting preferences (inline vs appendix, display vs debug)
//!
//! # Formatting Behavior
//!
//! Handlers can control two aspects of formatting:
//!
//! ## 1. Formatting Function Selection
//!
//! The [`ContextHandler::preferred_formatting_style`] and
//! [`AttachmentHandler::preferred_formatting_style`] methods allow handlers to specify
//! whether they prefer [`Display`](core::fmt::Display) or [`Debug`](core::fmt::Debug)
//! formatting when shown in a report. The default behavior is to always use `Display`
//! formatting, regardless of how the report itself is being formatted.
//!
//! All built-in handlers ([`Error`], [`Display`], [`struct@Debug`], [`Any`]) use this
//! default behavior, which means they use their `display` method even when the report
//! is being debug-formatted with `{:?}`.
//!
//! ## 2. Attachment Placement
//!
//! For attachments, handlers can also specify placement preferences via
//! [`AttachmentFormattingStyle`]:
//! - **Inline**: Rendered directly in the error chain
//! - **InlineWithHeader**: Rendered inline but with a header (for multi-line content)
//! - **Appendix**: Rendered in a separate appendix section
//! - **Opaque**: Not shown, but counted in a summary
//! - **Hidden**: Not shown at all
//!
//! # Built-in Handlers
//!
//! ## [`Error`]
//!
//! For types implementing [`std::error::Error`](core::error::Error). Delegates to the
//! type's `Display`, `Debug`, and `source` implementations. This is the default handler
//! for error types.
//!
//! ## [`Display`]
//!
//! For types implementing [`Display`](core::fmt::Display) and [`Debug`](core::fmt::Debug).
//! Useful for custom context types that aren't errors. Always returns `None` for `source`.
//!
//! ## [`struct@Debug`]
//!
//! For types implementing [`Debug`](core::fmt::Debug). Uses debug formatting for the
//! `debug` method and shows "Context of type `TypeName`" for the `display` method.
//! Useful for types that don't implement `Display`.
//!
//! ## [`Any`]
//!
//! For any type. Shows "An object of type TypeName" for both `display` and `debug`.
//! Used when no other formatting is available.
//!
//! # When Handlers Are Selected
//!
//! Handlers are typically selected automatically by the [`report!`](crate::report!) macro
//! based on the traits implemented by your context type. You can also specify a handler
//! explicitly using [`Report::new_custom`](crate::Report::new_custom).
//!
//! # Examples
//!
//! ```rust
//! use rootcause::prelude::*;
//! use std::io;
//!
//! // Error handler (automatic for std::error::Error types)
//! let io_err: io::Error = io::Error::new(io::ErrorKind::NotFound, "file.txt");
//! let report: Report<io::Error> = report!(io_err);
//!
//! // Display handler (automatic for Display + Debug types)
//! let msg: String = "Configuration invalid".to_string();
//! let report2: Report<String> = report!(msg);
//! ```

pub use rootcause_internals::handlers::{
    AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
    ContextFormattingStyle, ContextHandler, FormattingFunction,
};

/// Handler for types implementing [`std::error::Error`](core::error::Error).
///
/// This handler delegates to the error type's existing implementations of
/// [`Error::source`](core::error::Error::source), [`Display`](core::fmt::Display),
/// and [`Debug`](core::fmt::Debug). This is the default handler for any type that
/// implements the `Error` trait.
///
/// # When to Use
///
/// This handler is automatically selected by the [`report!`](crate::report!) macro
/// when you create a report from a type implementing `std::error::Error`. You rarely
/// need to specify it explicitly.
///
/// # Example
///
/// ```rust
/// use rootcause::prelude::*;
/// use std::io;
///
/// let error: io::Error = io::Error::new(io::ErrorKind::NotFound, "config.toml");
/// let report: Report<io::Error> = report!(error);
///
/// // The Error handler is used automatically, delegating to io::Error's Display
/// assert!(format!("{}", report).contains("config.toml"));
/// ```
#[derive(Copy, Clone)]
pub struct Error;

impl<C> ContextHandler<C> for Error
where
    C: core::error::Error,
{
    fn source(context: &C) -> Option<&(dyn core::error::Error + 'static)> {
        context.source()
    }

    fn display(context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(context, f)
    }

    fn debug(context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(context, f)
    }
}

/// Handler for types implementing [`Display`](core::fmt::Display) and [`Debug`](core::fmt::Debug).
///
/// This handler delegates to the type's `Display` and `Debug` implementations for formatting.
/// This is suitable for custom context types that aren't errors but can be meaningfully
/// displayed. The [`source`](ContextHandler::source) method always returns `None` since
/// these types don't have error sources.
///
/// # When to Use
///
/// This handler is automatically selected for types that implement `Display` and `Debug` but
/// not `std::error::Error`. This is ideal for custom context types like configuration objects,
/// request parameters, or descriptive messages.
///
/// # Examples
///
/// ```rust
/// use rootcause::prelude::*;
///
/// // String types use the Display handler
/// let report: Report = report!("Operation failed");
/// let output = format!("{}", report);
/// assert!(output.contains("Operation failed"));
///
/// // Custom types with Display also use this handler
/// #[derive(Debug)]
/// struct Config {
///     path: String,
///     timeout: u32,
/// }
///
/// impl std::fmt::Display for Config {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "Config[path={}, timeout={}]", self.path, self.timeout)
///     }
/// }
///
/// let config = Config {
///     path: "settings.toml".to_string(),
///     timeout: 30,
/// };
/// let report: Report<Config> = report!(config);
/// assert!(format!("{}", report).contains("settings.toml"));
/// ```
#[derive(Copy, Clone)]
pub struct Display;

impl<C> ContextHandler<C> for Display
where
    C: core::fmt::Display + core::fmt::Debug,
{
    fn source(_context: &C) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(context, f)
    }

    fn debug(context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(context, f)
    }
}

impl<A> AttachmentHandler<A> for Display
where
    A: core::fmt::Display + core::fmt::Debug,
{
    fn display(context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(context, f)
    }

    fn debug(context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(context, f)
    }
}

/// Handler for types implementing [`Debug`](core::fmt::Debug).
///
/// This handler uses the type's `Debug` implementation for the `debug` method, but shows
/// a generic message like "Context of type `TypeName`" for the `display` method. This is
/// useful for types that have debug information but don't implement `Display`.
///
/// # When to Use
///
/// This handler is automatically selected for types that implement `Debug` but not `Display`.
/// This is useful for internal data structures or types where displaying the full debug output
/// as the primary message would be too verbose.
///
/// # Formatting Behavior
///
/// - **`display` method**: Shows "Context of type `TypeName`"
/// - **`debug` method**: Uses the type's `Debug` implementation
/// - **`source` method**: Always returns `None`
/// - **Preferred formatting**: Uses `Display` by default, so contexts show the generic
///   type name message even when the report is formatted with `{:?}`
///
/// # Example
///
/// ```rust
/// use rootcause::prelude::*;
///
/// #[derive(Debug)]
/// struct InternalState {
///     connection_count: usize,
///     buffer: Vec<u8>,
/// }
///
/// let state = InternalState {
///     connection_count: 42,
///     buffer: vec![1, 2, 3],
/// };
///
/// let report: Report<InternalState> = report!(state);
///
/// // Display formatting shows a generic message with the type name
/// let display_output = format!("{}", report);
/// assert!(display_output.contains("InternalState"));
/// assert!(!display_output.contains("connection_count")); // Details not shown
///
/// // Debug formatting also uses the handler's Display method (the default behavior)
/// let debug_output = format!("{:?}", report);
/// assert!(debug_output.contains("InternalState"));
/// ```
#[derive(Copy, Clone)]
pub struct Debug;

impl<C> ContextHandler<C> for Debug
where
    C: core::fmt::Debug,
{
    fn source(_context: &C) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(_context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Context of type `{}`", core::any::type_name::<C>())
    }

    fn debug(context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(context, f)
    }
}

impl<A> AttachmentHandler<A> for Debug
where
    A: core::fmt::Debug,
{
    fn display(_context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Attachment of type `{}`", core::any::type_name::<A>())
    }

    fn debug(context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(context, f)
    }
}

/// Handler for any type, regardless of implemented traits.
///
/// This is the most generic handler, working with any type without requiring `Display`,
/// `Debug`, or `Error` implementations. Both `Display` and `Debug` output show
/// "An object of type TypeName" using [`type_name`](core::any::type_name).
///
/// # When to Use
///
/// This handler is a fallback for types that don't implement any formatting traits.
/// It's automatically selected when no more specific handler applies, or can be used
/// explicitly when you want to hide the details of a type.
///
/// # Formatting Behavior
///
/// - **Display output**: "An object of type TypeName"
/// - **Debug output**: "An object of type TypeName"
/// - **Source**: Always returns `None`
///
/// # Example
///
/// ```rust
/// use rootcause::{prelude::*, handlers};
///
/// struct Opaque {
///     secret: String,
/// }
///
/// let data = Opaque {
///     secret: "password123".to_string(),
/// };
///
/// // Use the Any handler explicitly to hide internal details
/// let report: Report<Opaque, markers::Mutable, markers::SendSync> =
///     Report::new_custom::<handlers::Any>(data);
///
/// // Only shows the type name, not the secret
/// let output = format!("{}", report);
/// assert!(output.contains("Opaque"));
/// assert!(!output.contains("password123"));
/// ```
#[derive(Copy, Clone)]
pub struct Any;

impl<A> AttachmentHandler<A> for Any {
    fn display(_context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "An object of type {}", core::any::type_name::<A>())
    }

    fn debug(_context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "An object of type {}", core::any::type_name::<A>())
    }
}

impl<C> ContextHandler<C> for Any {
    fn source(_context: &C) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(_context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "An object of type {}", core::any::type_name::<C>())
    }

    fn debug(_context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "An object of type {}", core::any::type_name::<C>())
    }
}
