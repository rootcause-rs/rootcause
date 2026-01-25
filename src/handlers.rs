//! Handlers that control how errors and attachments are formatted and
//! displayed.
//!
//! Handlers determine how context objects and attachments are formatted when
//! displaying or debugging error reports. The rootcause library provides
//! several built-in handlers that cover common use cases.
//!
//! # What Are Handlers?
//!
//! Handlers are types that implement the [`ContextHandler`] and/or
//! [`AttachmentHandler`] traits. They define how to format your error contexts
//! and attachments, including:
//! - How to display an error context (via [`Display`](core::fmt::Display))
//! - How to debug-format an error context (via [`Debug`](core::fmt::Debug))
//! - How to navigate to the error's source (via
//!   [`Error::source`](core::error::Error::source))
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
//! [`AttachmentHandler::preferred_formatting_style`] methods allow handlers to
//! specify whether they prefer [`Display`](core::fmt::Display) or
//! [`Debug`](core::fmt::Debug) formatting when shown in a report. The default
//! behavior is to always use `Display` formatting, regardless of how the report
//! itself is being formatted.
//!
//! All built-in handlers ([`Error`], [`Display`], [`struct@Debug`], [`Any`])
//! use this default behavior, which means they use their `display` method even
//! when the report is being debug-formatted with `{:?}`.
//!
//! ## 2. Attachment Placement
//!
//! For attachments, handlers can also specify placement preferences via
//! [`AttachmentFormattingStyle`]:
//! - **Inline**: Rendered directly in the error chain
//! - **InlineWithHeader**: Rendered inline but with a header (for multi-line
//!   content)
//! - **Appendix**: Rendered in a separate appendix section
//! - **Opaque**: Not shown, but counted in a summary
//! - **Hidden**: Not shown at all
//!
//! # Built-in Handlers
//!
//! ## [`Error`]
//!
//! For types implementing [`std::error::Error`](core::error::Error). Delegates
//! to the type's `Display`, `Debug`, and `source` implementations. This is the
//! default handler for error types.
//!
//! ## [`Display`]
//!
//! For types implementing [`Display`](core::fmt::Display) and
//! [`Debug`](core::fmt::Debug). Useful for custom context types that aren't
//! errors. Always returns `None` for `source`.
//!
//! ## [`struct@Debug`]
//!
//! For types implementing [`Debug`](core::fmt::Debug). Uses debug formatting
//! for the `debug` method and shows "Context of type `TypeName`" for the
//! `display` method. Useful for types that don't implement `Display`.
//!
//! ## [`Any`]
//!
//! For any type. Shows "An object of type TypeName" for both `display` and
//! `debug`. Used when no other formatting is available.
//!
//! # When Handlers Are Selected
//!
//! Handlers are typically selected automatically by the
//! [`report!`](crate::report!) macro based on the traits implemented by your
//! context type. You can also specify a handler explicitly using
//! [`Report::new_custom`](crate::Report::new_custom).
//!
//! # Examples
//!
//! ```
//! use std::io;
//!
//! use rootcause::prelude::*;
//!
//! // Error handler (automatic for std::error::Error types)
//! let io_err: io::Error = io::Error::new(io::ErrorKind::NotFound, "file.txt");
//! let report: Report<io::Error> = report!(io_err);
//!
//! // Display handler (automatic for Display + Debug types)
//! let msg: String = "Configuration invalid".to_string();
//! let report2: Report<String> = report!(msg);
//! ```

use core::marker::PhantomData;

use alloc::{fmt, string::ToString};
pub use rootcause_internals::handlers::{
    AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
    ContextFormattingStyle, ContextHandler, FormattingFunction,
};

/// Handler for types implementing [`std::error::Error`](core::error::Error).
///
/// This handler delegates to the error type's existing implementations of
/// [`Error::source`](core::error::Error::source),
/// [`Display`](core::fmt::Display), and [`Debug`](core::fmt::Debug). This is
/// the default handler for any type that implements the `Error` trait.
///
/// # When to Use
///
/// This handler is automatically selected by the [`report!`](crate::report!)
/// macro when you create a report from a type implementing `std::error::Error`.
/// You rarely need to specify it explicitly.
///
/// # Example
///
/// ```
/// use std::io;
///
/// use rootcause::prelude::*;
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

    fn preferred_formatting_style(
        _value: &C,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: report_formatting_function,
            ..Default::default()
        }
    }
}

/// Handler for types implementing [`Display`](core::fmt::Display) and
/// [`Debug`](core::fmt::Debug).
///
/// This handler delegates to the type's `Display` and `Debug` implementations
/// for formatting. This is suitable for custom context types that aren't errors
/// but can be meaningfully displayed. The [`source`](ContextHandler::source)
/// method always returns `None` since these types don't have error sources.
///
/// # When to Use
///
/// This handler is automatically selected for types that implement `Display`
/// and `Debug` but not `std::error::Error`. This is ideal for custom context
/// types like configuration objects, request parameters, or descriptive
/// messages.
///
/// # Examples
///
/// ```
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

    fn preferred_formatting_style(
        _value: &C,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: report_formatting_function,
        }
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

    fn preferred_formatting_style(
        _value: &A,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            function: report_formatting_function,
            ..Default::default()
        }
    }
}

/// Handler for types implementing [`Debug`](core::fmt::Debug) but not
/// [`Display`](core::fmt::Display).
///
/// This handler uses the type's `Debug` implementation for the `debug` method,
/// but shows a generic message for the `display` method. This is useful for
/// types that have debug information but don't implement `Display`.
///
/// # When to Use
///
/// This handler is automatically selected for types that implement `Debug` but
/// not `Display`.
///
/// # Formatting Behavior
///
/// - **Display output**: Shows "Context/Attachment of type `TypeName`"
/// - **Debug output**: Uses the type's `Debug` implementation
/// - **Source**: Always returns `None`
///
/// # Example
///
/// ```
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

    fn preferred_formatting_style(
        _value: &C,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: report_formatting_function,
            ..Default::default()
        }
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

    fn preferred_formatting_style(
        _value: &A,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            function: report_formatting_function,
            ..Default::default()
        }
    }
}

/// Handler for types implementing [`Debug`](core::fmt::Debug) but not
/// [`Display`](core::fmt::Display), and preventing debug output in report
/// formatting.
///
/// # When to Use
///
/// When having reports with contexts or attachments where displaying them as
/// debug in the report printout may lead to security issues, but where
/// accessing the debug formatting is still useful programmatically.
///
/// # Formatting Behavior
///
/// - **Display output**: Shows "Context/Attachment of type `TypeName`"
/// - **Debug output**: Uses the type's `Debug` implementation
/// - **Source**: Always returns `None`
/// - **Preferred formatting**: Always `Display`, so contexts show the generic
///   type name message even when the report is formatted with `{:?}`
///
/// # Example
///
/// ```
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
pub struct RedactDebug;

impl<C> ContextHandler<C> for RedactDebug
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

    fn preferred_formatting_style(
        _value: &C,
        _report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: FormattingFunction::Display,
            ..Default::default()
        }
    }
}

impl<A> AttachmentHandler<A> for RedactDebug
where
    A: core::fmt::Debug,
{
    fn display(_context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Attachment of type `{}`", core::any::type_name::<A>())
    }

    fn debug(context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(context, f)
    }

    fn preferred_formatting_style(
        _value: &A,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            function: FormattingFunction::Display,
            ..Default::default()
        }
    }
}

/// Attachment and context handler combinator for forcing [`derive@Debug`]-based
/// formatting.
///
/// # When to Use
///
/// For any case where an attachment needs to be shown in debug-view form even
/// if the report is printed as display.
///
/// **Be aware:** if your find yourself chaining multiple handler combinators,
/// consider writing your own instead.
///
/// # Formatting Behavior
///
/// - **Display output:** delegates to the wrapped handler's debug
///   implementation
/// - **Debug output:** delegates to the wrapped handler's debug implementation
/// - **Source
///
/// The default wrapped handler is [`struct@Debug`].
///
/// # Example
/// ```
/// use rootcause::{handlers, prelude::*};
///
/// let report: Report<&'static str, markers::Mutable, markers::SendSync> =
///     Report::new_custom::<handlers::Display>("I am a normal error!")
///         .attach_custom::<handlers::ForceDebug, _>(Some(42));
///
/// let output = format!("{}", report);
/// assert!(output.contains("Some(42)"));
/// ```
pub struct ForceDebug;

impl<A: 'static + Debug> AttachmentHandler<A> for ForceDebug {
    fn display(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        H::debug(value, formatter)
    }

    fn debug(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        H::debug(value, formatter)
    }

    fn preferred_formatting_style(
        value: &A,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            function: FormattingFunction::Debug,
            ..H::preferred_formatting_style(value, report_formatting_function)
        }
    }
}

impl<C: 'static + Debug> ContextHandler<T> for ForceDebug {
    fn source(value: &T) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(&value, formatter)
    }

    fn debug(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(&value, formatter)
    }

    fn preferred_formatting_style(
        _value: &T,
        _report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        ContextFormattingStyle {
            function: FormattingFunction::Debug,
            ..H::preferred_formatting_style(value, report_formatting_function)
        }
    }
}

/// Handler for any type, regardless of implemented traits.
///
/// This is the most generic handler, working with any type without requiring
/// `Display`, `Debug`, or `Error` implementations. Both `Display` and `Debug`
/// output show "An object of type TypeName" using
/// [`type_name`](core::any::type_name).
///
/// # When to Use
///
/// This handler is a fallback for types that don't implement any formatting
/// traits. It's automatically selected when no more specific handler applies,
/// or can be used explicitly when you want to hide the details of a type.
///
/// # Formatting Behavior
///
/// - **Display output**: "An object of type TypeName"
/// - **Debug output**: "An object of type TypeName"
/// - **Source**: Always returns `None`
///
/// # Example
///
/// ```
/// use rootcause::{handlers, prelude::*};
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
        write!(f, "Aattachment of type {}", core::any::type_name::<A>())
    }

    fn debug(_context: &A, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Attachment of type {}", core::any::type_name::<A>())
    }
}

impl<C> ContextHandler<C> for Any {
    fn source(_context: &C) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(_context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Context of type {}", core::any::type_name::<C>())
    }

    fn debug(_context: &C, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Context of type {}", core::any::type_name::<C>())
    }
}

/// Attachment handler for attachments that are not human-facing and/or
/// exclusively accessed programmatically.
///
/// This is a universally applicable handler similar to the [`Any`]. However
/// it sets the preferred formatting placement to [`Hidden`](AttachmentFormattingPlacement::Hidden),
/// meaning it is not formatted.
///
/// If manually formatted, outputs the empty string.
///
/// # When to Use
///
/// For attaching arbitrary non-human-interpretable data to a report, such as
/// marker types, a timestamp (i.e. an integer number of nanoseconds, not a
/// human-readable date), or some kind of internal identifier that is
/// meaningless for humans to inspect.
///
/// It is not useful as a context handler, and thus does not implement its use
/// as one.
///
/// # Formatting Behavior
///
/// - **Display output:** empty string
/// - **Debug output:** empty string
/// - **Formatting placement:** hidden
///
/// # Example
/// ```
/// use rootcause::{handlers, prelude::*};
///
/// let report: Report<&'static str, markers::Mutable, markers::SendSync> =
///     Report::new_custom::<handlers::Display>("I am a normal error!")
///         .attach_custom::<handlers::Invisible, &'static str>("Internal data");
///
/// // Does not show
/// let output = format!("{}", report);
/// assert!(!output.contains("Internal data"));
/// ```
#[derive(Copy, Clone)]
pub struct Invisible;

impl<T: 'static> AttachmentHandler<T> for Invisible {
    fn display(_value: &T, _formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }

    fn debug(_value: &T, _formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }

    fn preferred_formatting_style(
        _value: &T,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Hidden,
            function: report_formatting_function,
            priority: i32::MIN,
        }
    }
}
