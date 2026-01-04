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
/// This handler uses the type's `Debug` implementation for the `debug` method,
/// but shows a generic message like "Context of type `TypeName`" for the
/// `display` method. This is useful for types that have debug information but
/// don't implement `Display`.
///
/// # When to Use
///
/// This handler is automatically selected for types that implement `Debug` but
/// not `Display`. This is useful for internal data structures or types where
/// displaying the full debug output as the primary message would be too
/// verbose.
///
/// # Formatting Behavior
///
/// - **`display` method**: Shows "Context of type `TypeName`"
/// - **`debug` method**: Uses the type's `Debug` implementation
/// - **`source` method**: Always returns `None`
/// - **Preferred formatting**: Uses `Display` by default, so contexts show the
///   generic type name message even when the report is formatted with `{:?}`
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

/// Attachment handler for attachments that are not human-facing and/or exclusively acessed programmatically.
///
/// This is also a universally applicable handler, since it literally implements no functionality. It both displays and debugs as the empty string. However, in the preferred formatting options it specifies the attachment as [`Hidden`](AttachmentFormattingPlacement::Hidden) so as to never actually show up in a formatted report.
///
/// # When to Use
///
/// For attaching arbitrary non-human-interpretable data to a report, such as marker types, a timestamp (i.e. an integer number of nanoseconds, not a human-readable date), or some kind of internal identifier that is meaningless for humans.
///
/// It is not useful as a context handler, and thus does not implement its use as one.
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
/// let report : Report<&'static str, markers::Mutable, markers::SendSync>
///   = Report::new_custom::<handlers::Display>("I am a normal error!")
///   .attach_custom::<handlers::Hidden, &'static str>("Internal data");
///
/// // Does not show
/// let output = format!("{}", report);
/// assert!(!output.contains("Internal data"));
/// ```
#[derive(Copy, Clone)]
pub struct Hidden;

impl<T: 'static> AttachmentHandler<T> for Hidden {
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

/// Attachment handler combinator for attachments that are intended to be hidden from view by default, but still inspectable.
///
/// The preferred formatting options of the wrapped handler are preserved, exception the formatting placement is changed so the attachment only shows up as a summary count.
///
/// # When to Use
///
/// Same usecases as [`Hidden`] but for when you want humans to be aware of the existence of the attachment, and possibly still want to be able to format the wrapped attachment. The default wrapped handler is [`Display`].
///
/// It is not useful as a context handler, and thus does not implement its use as one.
///
/// **Be aware:** if your find yourself chaining multiple attachment handler combinators, consider writing your own instead.
///
/// # Formatting Behavior
///
/// - **Display output:** delegates to the wrapped handler
/// - **Debug output:** delegates to the wrapped handler
/// - **Formatting placement:** delegates to wrapped handler but changes the `placement` to [`Opaque`](AttachmentFormattingPlacement::Hidden)
///
/// The default wrapped handler is [`Display`].
///
/// # Example
/// ```
/// use rootcause::{handlers, prelude::*};
///
/// let report : Report<&'static str, markers::Mutable, markers::SendSync>
///   = Report::new_custom::<handlers::Display>("I am a normal error!")
///     .attach_custom::<handlers::Opaque, i32>(42069);
///
/// // Does not show
/// let output = format!("{}", report);
/// assert!(output.contains("1 additional opaque attachment"));
/// assert!(!output.contains("42069"));
/// ```
#[derive(Copy, Clone)]
pub struct Opaque<H = Display>(PhantomData<H>);

impl<T: 'static, H: AttachmentHandler<T>> AttachmentHandler<T> for Opaque<H> {
    fn display(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        H::display(value, formatter)
    }

    fn debug(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        H::debug(value, formatter)
    }

    fn preferred_formatting_style(
        value: &T,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let mut res = H::preferred_formatting_style(value, report_formatting_function);
        res.placement = AttachmentFormattingPlacement::Opaque;
        res
    }
}

/// Attachment handler combinator for rendering attachments with a customizable header in the formatting, useful to annotate free-form data with distinct semantic meaning.
///
/// # When to Use
///
/// For the case of attaching multiple data that are all the same type or could be confused for one another, but which are semantically distinct and needs a human-readable indicator of what is what.
///
/// It is not useful as a context handler, and thus does not implement its use as one.
///
/// **Be aware:** if your find yourself chaining multiple attachment handler combinators, consider writing your own instead.
///
/// # Formatting Behavior
///
/// - **Display output:** second element of the attachment tuple delegated to wrapped handler
/// - **Debug output:** second element of the attachment tuple delegated to wrapped handler
/// - **Formatting placement:** delegates to wrapped hander but as `InlineWithHeader` and header title given by the first element of the attachment tuple
///
/// The default wrapped handler is [`Display`].
///
/// # Example
/// ```
/// use rootcause::{handlers, prelude::*};
///
/// let report : Report<&'static str, markers::Mutable, markers::SendSync>
///   = Report::new_custom::<handlers::Display>("I am a normal error!")
///     .attach_custom::<handlers::Header, _>(("Funny number", 42069));
///
/// let output = format!("{}", report);
/// assert!(output.contains("Funny number"));
/// assert!(output.contains("42069"));
/// ```
#[allow(dead_code)]
pub struct WithHeader<H = Display>(PhantomData<H>);
impl<T: 'static, H: AttachmentHandler<T>> AttachmentHandler<(&'static str, T)> for WithHeader<H> {
    fn display(
        value: &(&'static str, T),
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        H::display(&value.1, formatter)
    }

    fn debug(
        value: &(&'static str, T),
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        H::debug(&value.1, formatter)
    }

    fn preferred_formatting_style(
        value: &(&'static str, T),
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let mut res = H::preferred_formatting_style(&value.1, report_formatting_function);
        res.placement = AttachmentFormattingPlacement::InlineWithHeader { header: value.0 };
        res
    }
}

/// Attachment handler combinator for rendering attachments as appendices with a customizable header, useful to annotate large free-form data with distinct semantic meaning.
///
/// # When to Use
///
/// For the case of attaching multiple data that are all the same type or could be confused for one another, but which are semantically distinct and needs a human-readable indicator of what is what.
///
/// It is not useful as a context handler, and thus does not implement its use as one.
///
/// **Be aware:** if your find yourself chaining multiple attachment handler combinators, consider writing your own instead.
///
/// # Formatting Behavior
///
/// - **Display output:** second element of the attachment tuple delegated to wrapped handler
/// - **Debug output:** second element of the attachment tuple delegated to wrapped handler
/// - **Formatting placement:** delegates to wrapped hander but as [`Appendix`](AttachmentFormattingPlacement::Appendix) and with `appendix_name`` given by the first element of the attachment tuple
///
/// The default wrapped handler is [`Display`].
///
/// # Example
/// ```
/// use rootcause::{handlers, prelude::*};
///
/// let report : Report<&'static str, markers::Mutable, markers::SendSync>
///   = Report::new_custom::<handlers::Display>("I am a normal error!")
///     .attach_custom::<handlers::Appendix, _>(("Placeholder text",
/// "Lorem ipsum dolor sit amet, consectetur adipiscing elit.
/// Aliquam eu lobortis enim. Quisque tempus ligula vehicula leo viverra efficitur.
/// Ut id rhoncus lectus, maximus tristique nibh. Ut non orci justo.
/// Nunc non iaculis lorem. Phasellus nibh mi, feugiat sed diam a, venenatis posuere metus.
/// Sed pharetra dapibus odio in ultricies."));
///
/// let output = format!("{}", report);
/// assert!(output.contains("See Placeholder text #1 below"));
/// assert!(output.contains("Lorem ipsum"));
/// ```
#[allow(dead_code)]
pub struct Appendix<H = Display>(PhantomData<H>);
impl<T: 'static, H: AttachmentHandler<T>> AttachmentHandler<(&'static str, T)> for Appendix<H> {
    fn display(
        value: &(&'static str, T),
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        H::display(&value.1, formatter)
    }

    fn debug(
        value: &(&'static str, T),
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        H::debug(&value.1, formatter)
    }

    fn preferred_formatting_style(
        value: &(&'static str, T),
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let mut res = H::preferred_formatting_style(&value.1, report_formatting_function);
        res.placement = AttachmentFormattingPlacement::Appendix {
            appendix_name: value.0,
        };
        res
    }
}

/// Attachment handler combinator for reordering attachments.
///
/// # When to Use
///
/// For any case where an attachment is dramaticall more or less important than the other attachments. Positive priorities come earlier in the report printout.
///
/// Note that the base priority of an attachment is zero, while the common [`Location`](crate::hooks::builtin_hooks::location::Location) has a priority of 20. By default this combinator sets the priority to 10.
///
/// It is not useful as a context handler, and thus does not implement its use as one.
///
/// **Be aware:** if your find yourself chaining multiple attachment handler combinators, consider writing your own instead.
///
/// # Formatting Behavior
///
/// - **Display output:** delegates to the wrapped handler
/// - **Debug output:** delegates to the wrapped handler
/// - **Formatting placement:** delegates to wrapped handler but changes the priority
///
/// The default wrapped handler is [`Display`].
///
/// # Example
/// ```
/// use rootcause::{handlers, prelude::*};
///
/// let report : Report<&'static str, markers::Mutable, markers::SendSync>
///   = Report::new_custom::<handlers::Display>("I am a normal error!")
///     .attach("Less important")
///     .attach("Less important 2")
///     .attach_custom::<handlers::Priority, _>("More important");
///
/// let output = format!("{}", report);
/// // same priority = order of attachment
/// assert!(output.find("Less important").unwrap() < output.find("Less important 2").unwrap());
/// // different priority = highest first
/// assert!(output.find("More important").unwrap() < output.find("Less important").unwrap());
/// ```
pub struct Priority<H = Display, const N: i32 = 10>(PhantomData<H>);
impl<T: 'static, H: AttachmentHandler<T>, const N: i32> AttachmentHandler<T>
    for Priority<H, { N }>
{
    fn display(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        H::display(value, formatter)
    }

    fn debug(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        H::debug(value, formatter)
    }

    fn preferred_formatting_style(
        value: &T,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let mut res = H::preferred_formatting_style(value, report_formatting_function);
        res.priority = N;
        res
    }
}
