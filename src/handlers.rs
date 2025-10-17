//! Handlers used to implement or override the behavior of [`core::error::Error`], [`core::fmt::Debug`] or
//! [`core::fmt::Display`] when creating an attachment or report.

pub use rootcause_internals::handlers::{
    AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
    ContextFormattingStyle, ContextHandler, FormattingFunction,
};

/// A handler that implements [`ContextHandler<C>`] for any `C` that implements [`core::error::Error`], by delegating
/// to [`Error::source`], [`Display::fmt`] and [`Debug::fmt`].
///
/// [`Error::source`]: core::error::Error::source
/// [`Display::fmt`]: core::fmt::Display::fmt
/// [`Debug::fmt`]: core::fmt::Debug::fmt
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

/// A handler that implements [`ContextHandler`] and [`AttachmentHandler`].
///
/// [`ContextHandler<C>`] is implemented for any `C` that implements [`core::fmt::Display`] and [`core::fmt::Debug`], and similarly
/// [`AttachmentHandler<A>`] is implemented for any `A` that implements those same traits.
///
/// The methods [`ContextHandler::display`], [`ContextHandler::debug`], [`AttachmentHandler::display`]
/// and [`AttachmentHandler::debug`] are implemented by delegating to [`Display::fmt`] and [`Debug::fmt`].
///
/// The [`ContextHandler::source`] method always returns `None`.
///
/// [`Display::fmt`]: core::fmt::Display::fmt
/// [`Debug::fmt`]: core::fmt::Debug::fmt
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

/// A handler that implements [`ContextHandler`] and [`AttachmentHandler`].
///
/// [`ContextHandler<C>`] is implemented for any `C` that implements [`core::fmt::Debug`], and similarly
/// [`AttachmentHandler<A>`] is implemented for any `A` that implements that same trait.
///
/// The methods [`ContextHandler::debug`] and [`AttachmentHandler::debug`] are implemented
/// by delegating to [`Debug::fmt`].
///
/// The [`ContextHandler::source`] method always returns `None`.
///
/// The [`ContextHandler::display`] and [`AttachmentHandler::display`] methods always output the string "An object of type {...}".
///
/// [`Debug::fmt`]: core::fmt::Debug::fmt
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

/// A handler that implements [`ContextHandler`] and [`AttachmentHandler`].
///
/// [`ContextHandler<C>`] is implemented for any `C`, and similarly
/// [`AttachmentHandler<A>`] is implemented for any `A`.
///
/// The [`ContextHandler::source`] method always returns `None`.
///
/// The [`ContextHandler::display`], [`ContextHandler::debug`], [`AttachmentHandler::display`]
/// and [`AttachmentHandler::debug`] methods always output the string "An object of type {...}".
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
