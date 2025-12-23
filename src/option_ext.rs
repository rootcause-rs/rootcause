//! Extension traits for `Option` types to integrate with rootcause error
//! reporting.
//!
//! This module provides the [`OptionExt`] trait, which adds error handling
//! methods to `Option` types. When an `Option` is `None`, these methods
//! automatically create a [`Report`] containing a [`NoneError`] that captures
//! type information about the expected value.
//!
//! # Quick Start
//!
//! ```
//! use rootcause::{option_ext::OptionExt, prelude::*};
//!
//! fn get_config_value() -> Option<String> {
//!     None
//! }
//!
//! # fn example() -> Result<String, Report<&'static str>> {
//! // Convert None to a Report
//! let result = get_config_value().context("Failed to load configuration")?;
//! # Ok(result)
//! # }
//! ```
//!
//! # Available Methods
//!
//! The [`OptionExt`] trait provides methods similar to
//! [`ResultExt`](crate::prelude::ResultExt), including:
//!
//! - **[`ok_or_report()`](OptionExt::ok_or_report)** - Convert `None` to
//!   `Report<NoneError>`
//! - **[`context()`](OptionExt::context)** - Add context when `None`
//! - **Local variants** - `local_*` methods for non-`Send + Sync` types
//!
//! # Thread Safety
//!
//! All methods have both thread-safe (`Send + Sync`) and local
//! (non-thread-safe) variants. Use the `local_*` methods when working with
//! types that cannot be sent across threads, such as `Rc` or `Cell`.
//!
//! # Usage Considerations
//!
//! Some developers prefer to keep `Option` and `Result` handling visually
//! distinct in their code. Using [`OptionExt`] can make it less obvious when
//! you're working with an `Option` versus a `Result`, since both can use
//! similar error handling methods like `.context()`. If code clarity is a
//! concern, consider using explicit conversion with
//! [`.ok_or_report()`](OptionExt::ok_or_report) followed by standard `Result`
//! methods, or using [`Option::ok_or`] or [`Option::ok_or_else`] with your own
//! error types.

use crate::{
    Report, ReportConversion, handlers,
    markers::{Local, Mutable, SendSync},
};

/// Error type representing a missing `Option` value.
///
/// This error is automatically created when using [`OptionExt`] methods on a
/// `None` value. It captures the type name of the expected value for
/// better debugging.
///
/// # Examples
///
/// ```
/// use rootcause::option_ext::NoneError;
///
/// let error = NoneError::new::<String>();
/// assert!(format!("{error}").contains("String"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoneError {
    /// The type name of the expected value.
    type_name: &'static str,
}

impl NoneError {
    /// Creates a new `NoneError` for the given type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::option_ext::NoneError;
    ///
    /// let error = NoneError::new::<String>();
    /// let message = format!("{error}");
    /// assert!(message.contains("String"));
    /// ```
    #[must_use]
    pub fn new<T: ?Sized>() -> Self {
        Self {
            type_name: core::any::type_name::<T>(),
        }
    }

    #[must_use]
    #[track_caller]
    fn new_sendsync_report<T: ?Sized>() -> Report<NoneError, Mutable, SendSync> {
        Report::new(Self::new::<T>())
    }

    #[must_use]
    #[track_caller]
    fn new_local_report<T: ?Sized>() -> Report<NoneError, Mutable, Local> {
        Report::new_local(Self::new::<T>())
    }
}

impl core::fmt::Display for NoneError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Expected value of type {}, but found None",
            self.type_name
        )
    }
}

impl core::error::Error for NoneError {}

/// Extension trait for `Option` that provides error handling and reporting
/// functionality.
///
/// This trait adds methods to `Option` types that allow you to convert `None`
/// values into [`Report`]s with additional context and attachments. It provides
/// both thread-safe (`Send + Sync`) and local-only variants of each method.
///
/// When a `None` value is encountered, a [`NoneError`] is automatically
/// created to represent the missing value, capturing type information for
/// better debugging.
///
/// The methods in this trait fall into several categories:
///
/// - **Converting to reports**: [`ok_or_report`](OptionExt::ok_or_report)
///   converts `None` into a [`Report<NoneError>`]
/// - **Adding context**: [`context`](OptionExt::context),
///   [`context_with`](OptionExt::context_with), and variants add a new context
///   layer when the option is `None`
///
/// Each context method has a `local_*` variant for working with types that are
/// not `Send + Sync`.
///
/// # Examples
///
/// ```
/// use rootcause::option_ext::OptionExt;
///
/// let value: Option<String> = None;
/// let result = value.ok_or_report();
/// assert!(result.is_err());
/// ```
pub trait OptionExt<V> {
    /// Converts `None` into a [`Report<NoneError>`].
    ///
    /// If the option is `Some`, returns the value. If the option is `None`,
    /// creates a [`Report`] containing a [`NoneError`] that captures the
    /// type information of the expected value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     option_ext::{NoneError, OptionExt},
    ///     prelude::*,
    /// };
    ///
    /// let value: Option<String> = None;
    /// let result: Result<String, Report<NoneError>> = value.ok_or_report();
    /// assert!(result.is_err());
    /// ```
    #[track_caller]
    fn ok_or_report(self) -> Result<V, Report<NoneError, Mutable, SendSync>>;

    /// Converts `None` into a new [`Report`] using the provided context. The
    /// [`NoneError`] is set as a child of the new [`Report`].
    ///
    /// If the option is `Some`, returns the value unchanged. If the option is
    /// `None`, creates a [`NoneError`] and adds the provided context as
    /// the primary error message, with the [`NoneError`] becoming part of
    /// the error chain.
    ///
    /// See also [`local_context`](OptionExt::local_context) for a
    /// non-thread-safe version that works with types that are not
    /// `Send + Sync`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{option_ext::OptionExt, prelude::*};
    ///
    /// fn get_user_id() -> Option<u64> {
    ///     None
    /// }
    ///
    /// let result: Result<u64, Report<&str>> = get_user_id().context("Failed to get user ID");
    /// ```
    #[track_caller]
    fn context<C>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        C: Send + Sync + core::fmt::Display + core::fmt::Debug;

    /// Converts `None` into a new [`Report`] using context generated by the
    /// provided closure. The [`NoneError`] is set as a child of the new
    /// [`Report`].
    ///
    /// This is similar to [`context`](OptionExt::context), but the context is
    /// computed lazily using a closure. This can be useful when computing
    /// the context is expensive, as the closure will only be called if the
    /// option is `None`.
    ///
    /// See also [`local_context_with`](OptionExt::local_context_with) for a
    /// non-thread-safe version that works with types that are not
    /// `Send + Sync`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{option_ext::OptionExt, prelude::*};
    ///
    /// fn expensive_context_computation() -> String {
    ///     format!("Failed at {}", "12:34:56")
    /// }
    ///
    /// let value: Option<u64> = None;
    /// let result: Result<u64, Report<String>> = value.context_with(expensive_context_computation);
    /// ```
    #[track_caller]
    fn context_with<C, F>(self, context: F) -> Result<V, Report<C, Mutable, SendSync>>
    where
        F: FnOnce() -> C,
        C: Send + Sync + core::fmt::Display + core::fmt::Debug;

    /// Converts `None` into a new [`Report`] using the provided context and a
    /// custom handler. The [`NoneError`] is set as a child of the new
    /// [`Report`].
    ///
    /// This is similar to [`context`](OptionExt::context), but uses a custom
    /// [`ContextHandler`] to control how the context is formatted and
    /// displayed.
    ///
    /// See also [`local_context_custom`](OptionExt::local_context_custom) for a
    /// non-thread-safe version that works with types that are not
    /// `Send + Sync`.
    ///
    /// [`ContextHandler`]: handlers::ContextHandler
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{handlers, option_ext::OptionExt, prelude::*};
    ///
    /// #[derive(Debug)]
    /// struct ErrorContext {
    ///     code: u32,
    ///     message: String,
    /// }
    ///
    /// let value: Option<String> = None;
    /// let result: Result<String, Report<ErrorContext>> =
    ///     value.context_custom::<handlers::Debug, _>(ErrorContext {
    ///         code: 404,
    ///         message: "Not found".to_string(),
    ///     });
    /// ```
    #[track_caller]
    fn context_custom<H, C>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        C: Send + Sync,
        H: handlers::ContextHandler<C>;

    /// Converts `None` into a new [`Report`] using context generated by the
    /// provided closure and a custom handler. The [`NoneError`] is set as
    /// a child of the new [`Report`].
    ///
    /// This is similar to [`context_with`](OptionExt::context_with), but uses a
    /// custom [`ContextHandler`] to control how the context is formatted and
    /// displayed.
    ///
    /// See also [`local_context_custom_with`](OptionExt::local_context_custom_with)
    /// for a non-thread-safe version that works with types that are not
    /// `Send + Sync`.
    ///
    /// [`ContextHandler`]: handlers::ContextHandler
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{handlers, option_ext::OptionExt, prelude::*};
    ///
    /// #[derive(Debug)]
    /// struct ErrorContext {
    ///     timestamp: String,
    ///     operation: String,
    /// }
    ///
    /// fn expensive_computation() -> ErrorContext {
    ///     ErrorContext {
    ///         timestamp: "12:34:56".to_string(),
    ///         operation: "lookup".to_string(),
    ///     }
    /// }
    ///
    /// let value: Option<u64> = None;
    /// let result: Result<u64, Report<ErrorContext>> =
    ///     value.context_custom_with::<handlers::Debug, _, _>(expensive_computation);
    /// ```
    #[track_caller]
    fn context_custom_with<H, C, F>(self, context: F) -> Result<V, Report<C, Mutable, SendSync>>
    where
        F: FnOnce() -> C,
        C: Send + Sync,
        H: handlers::ContextHandler<C>;

    /// Converts `None` to a different context type using [`ReportConversion`].
    ///
    /// If `None`, creates a [`Report<NoneError>`] and transforms it using
    /// the [`ReportConversion`] implementation. Implement
    /// [`ReportConversion`] once to define conversions, then use `context_to()`
    /// at call sites. The target type `C` is typically inferred from the return
    /// type.
    ///
    /// See also: [`local_context_to`](OptionExt::local_context_to) (non-`Send +
    /// Sync` version).
    ///
    /// [`ReportConversion`]: crate::ReportConversion
    ///
    /// # Examples
    ///
    /// ```
    /// # use rootcause::{ReportConversion, markers, option_ext::{OptionExt, NoneError}, prelude::*};
    /// #[derive(Debug)]
    /// enum AppError {
    ///     MissingValue
    /// }
    ///
    /// # impl std::fmt::Display for AppError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "error") }
    /// # }
    /// impl<O, T> ReportConversion<NoneError, O, T> for AppError
    ///   where AppError: markers::ObjectMarkerFor<T>
    /// {
    ///     fn convert_report(report: Report<NoneError, O, T>) -> Report<Self, markers::Mutable, T>
    ///     {
    ///         report.context(AppError::MissingValue)
    ///     }
    /// }
    ///
    /// let value: Option<String> = None;
    /// let result: Result<String, Report<AppError>> = value.context_to();
    /// ```
    #[track_caller]
    fn context_to<C>(self) -> Result<V, Report<C, Mutable, SendSync>>
    where
        C: ReportConversion<NoneError, Mutable, SendSync>;

    // Local variants (non-Send + Sync)

    /// Converts `None` into a new local (non-thread-safe) [`Report`] using
    /// the provided context. The [`NoneError`] is set as a child of the new
    /// [`Report`].
    ///
    /// This is the non-`Send + Sync` version of
    /// [`context`](OptionExt::context). Use this when working with context
    /// types that cannot be sent across thread boundaries.
    ///
    /// See also [`context`](OptionExt::context) for a thread-safe version that
    /// returns a [`Report`] that can be sent across thread boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use rootcause::{option_ext::OptionExt, prelude::*};
    ///
    /// let value: Option<String> = None;
    /// let result: Result<String, Report<Rc<&str>, _, markers::Local>> =
    ///     value.local_context(Rc::from("Failed to get value"));
    /// ```
    #[track_caller]
    fn local_context<C>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        C: core::fmt::Display + core::fmt::Debug;

    /// Converts `None` into a new local (non-thread-safe) [`Report`] using
    /// context generated by the provided closure. The [`NoneError`] is set
    /// as a child of the new [`Report`].
    ///
    /// This is the non-`Send + Sync` version of
    /// [`context_with`](OptionExt::context_with). Use this when working with
    /// context types that cannot be sent across thread boundaries.
    ///
    /// See also [`context_with`](OptionExt::context_with) for a thread-safe
    /// version that returns a [`Report`] that can be sent across thread
    /// boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use rootcause::{option_ext::OptionExt, prelude::*};
    ///
    /// fn expensive_computation() -> Rc<String> {
    ///     Rc::new(format!("Failed at {}", "12:34:56"))
    /// }
    ///
    /// let value: Option<u64> = None;
    /// let result: Result<u64, Report<Rc<String>, _, markers::Local>> =
    ///     value.local_context_with(expensive_computation);
    /// ```
    #[track_caller]
    fn local_context_with<C, F>(self, context: F) -> Result<V, Report<C, Mutable, Local>>
    where
        F: FnOnce() -> C,
        C: core::fmt::Display + core::fmt::Debug;

    /// Converts `None` into a new local (non-thread-safe) [`Report`] using the
    /// provided context and a custom handler. The [`NoneError`] is set as a
    /// child of the new [`Report`].
    ///
    /// This is the non-`Send + Sync` version of
    /// [`context_custom`](OptionExt::context_custom). Use this when working
    /// with context types that cannot be sent across thread boundaries.
    ///
    /// See also [`context_custom`](OptionExt::context_custom) for a thread-safe
    /// version that returns a [`Report`] that can be sent across thread
    /// boundaries.
    ///
    /// [`ContextHandler`]: handlers::ContextHandler
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use rootcause::{handlers, option_ext::OptionExt, prelude::*};
    ///
    /// #[derive(Debug)]
    /// struct ErrorContext {
    ///     data: Rc<String>,
    /// }
    ///
    /// let value: Option<String> = None;
    /// let result: Result<String, Report<ErrorContext, _, markers::Local>> = value
    ///     .local_context_custom::<handlers::Debug, _>(ErrorContext {
    ///         data: Rc::new("context".to_string()),
    ///     });
    /// ```
    #[track_caller]
    fn local_context_custom<H, C>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        C: 'static,
        H: handlers::ContextHandler<C>;

    /// Converts `None` into a new local (non-thread-safe) [`Report`] using
    /// context generated by the provided closure and a custom handler. The
    /// [`NoneError`] is set as a child of the new [`Report`].
    ///
    /// This is the non-`Send + Sync` version of
    /// [`context_custom_with`](OptionExt::context_custom_with). Use this when
    /// working with context types that cannot be sent across thread boundaries.
    ///
    /// See also [`context_custom_with`](OptionExt::context_custom_with) for a
    /// thread-safe version that returns a [`Report`] that can be sent across
    /// thread boundaries.
    ///
    /// [`ContextHandler`]: handlers::ContextHandler
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use rootcause::{handlers, option_ext::OptionExt, prelude::*};
    ///
    /// #[derive(Debug)]
    /// struct ErrorContext {
    ///     data: Rc<String>,
    /// }
    ///
    /// fn expensive_computation() -> ErrorContext {
    ///     ErrorContext {
    ///         data: Rc::new("context".to_string()),
    ///     }
    /// }
    ///
    /// let value: Option<u64> = None;
    /// let result: Result<u64, Report<ErrorContext, _, markers::Local>> =
    ///     value.local_context_custom_with::<handlers::Debug, _, _>(expensive_computation);
    /// ```
    #[track_caller]
    fn local_context_custom_with<H, C, F>(self, context: F) -> Result<V, Report<C, Mutable, Local>>
    where
        F: FnOnce() -> C,
        C: 'static,
        H: handlers::ContextHandler<C>;

    /// Converts `None` to a different context type using [`ReportConversion`].
    ///
    /// This is the non-`Send + Sync` version of
    /// [`context_to`](OptionExt::context_to). Use this when working with types
    /// that cannot be sent across thread boundaries.
    ///
    /// See also [`context_to`](OptionExt::context_to) for a thread-safe version
    /// that returns a [`Report`] that can be sent across thread boundaries.
    ///
    /// [`ReportConversion`]: crate::ReportConversion
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::rc::Rc;
    /// # use rootcause::{ReportConversion, markers, option_ext::{OptionExt, NoneError}, prelude::*};
    /// #[derive(Debug)]
    /// enum AppError {
    ///     MissingValue
    /// }
    /// # impl std::fmt::Display for AppError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "error") }
    /// # }
    ///
    /// impl<O> ReportConversion<NoneError, O, markers::Local> for AppError
    /// {
    ///     fn convert_report(report: Report<NoneError, O, markers::Local>) -> Report<Self, markers::Mutable, markers::Local>
    ///     {
    ///         report.context(AppError::MissingValue)
    ///     }
    /// }
    ///
    /// let value: Option<Rc<String>> = None;
    /// let result: Result<Rc<String>, Report<AppError, _, markers::Local>> = value.local_context_to();
    /// ```
    #[track_caller]
    fn local_context_to<C>(self) -> Result<V, Report<C, Mutable, Local>>
    where
        C: ReportConversion<NoneError, Mutable, Local>;
}

impl<V> OptionExt<V> for Option<V> {
    #[inline]
    fn ok_or_report(self) -> Result<V, Report<NoneError, Mutable, SendSync>> {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_sendsync_report::<V>()),
        }
    }

    #[inline]
    fn context<C>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        C: Send + Sync + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_sendsync_report::<V>().context(context)),
        }
    }

    #[inline]
    fn context_with<C, F>(self, context: F) -> Result<V, Report<C, Mutable, SendSync>>
    where
        F: FnOnce() -> C,
        C: Send + Sync + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_sendsync_report::<V>().context(context())),
        }
    }

    #[inline]
    fn context_custom<H, C>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        C: Send + Sync,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_sendsync_report::<V>().context_custom::<H, _>(context)),
        }
    }

    #[inline]
    fn context_custom_with<H, C, F>(self, context: F) -> Result<V, Report<C, Mutable, SendSync>>
    where
        F: FnOnce() -> C,
        C: Send + Sync,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_sendsync_report::<V>().context_custom::<H, _>(context())),
        }
    }

    #[inline]
    fn context_to<C>(self) -> Result<V, Report<C, Mutable, SendSync>>
    where
        C: ReportConversion<NoneError, Mutable, SendSync>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_sendsync_report::<V>().context_to()),
        }
    }

    #[inline]
    fn local_context<C>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        C: core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_local_report::<V>().context(context)),
        }
    }

    #[inline]
    fn local_context_with<C, F>(self, context: F) -> Result<V, Report<C, Mutable, Local>>
    where
        F: FnOnce() -> C,
        C: core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_local_report::<V>().context(context())),
        }
    }

    #[inline]
    fn local_context_custom<H, C>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        C: 'static,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_local_report::<V>().context_custom::<H, _>(context)),
        }
    }

    #[inline]
    fn local_context_custom_with<H, C, F>(self, context: F) -> Result<V, Report<C, Mutable, Local>>
    where
        F: FnOnce() -> C,
        C: 'static,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_local_report::<V>().context_custom::<H, _>(context())),
        }
    }

    #[inline]
    fn local_context_to<C>(self) -> Result<V, Report<C, Mutable, Local>>
    where
        C: ReportConversion<NoneError, Mutable, Local>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError::new_local_report::<V>().context_to()),
        }
    }
}
