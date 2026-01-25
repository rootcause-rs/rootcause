//! Handlers that define formatting and error-chaining behavior for reports and
//! attachments.
//!
//! This module provides the core traits and types for implementing custom
//! handlers that control how context objects and attachments are formatted and
//! displayed in error reports.

use alloc::borrow::Cow;

/// Trait for implementing custom formatting and error-chaining behavior for
/// report contexts.
///
/// This trait defines how a context type should be formatted when displayed or
/// debugged as part of an error report, and how to navigate to its error source
/// (if any).
///
/// # When to Implement
///
/// You typically don't need to implement this trait directly. The rootcause
/// library provides built-in handlers (`Error`, `Display`, `Debug`, `Any`) that
/// cover most use cases.
///
/// Implement this trait when you need custom formatting behavior that the
/// built-in handlers don't provide, such as:
/// - Custom source chain navigation for types that don't implement
///   `std::error::Error`
/// - Special display formatting that differs from the type's `Display`
///   implementation
/// - Dynamic formatting based on the context value
///
/// # Required Methods
///
/// - [`source`](ContextHandler::source): Returns the underlying error source,
///   if any
/// - [`display`](ContextHandler::display): Formats the context for display
///   output
/// - [`debug`](ContextHandler::debug): Formats the context for debug output
///
/// # Optional Methods
///
/// - [`preferred_formatting_style`](ContextHandler::preferred_formatting_style):
///   Specifies whether to use display or debug formatting when embedded in a report.
///   The default implementation always prefers display formatting.
///
/// # Examples
///
/// ```
/// use std::error::Error;
///
/// use rootcause_internals::handlers::{
///     ContextFormattingStyle, ContextHandler, FormattingFunction,
/// };
///
/// // Custom context type
/// struct CustomError {
///     message: String,
///     code: i32,
/// }
///
/// // Custom handler with special formatting
/// struct CustomHandler;
///
/// impl ContextHandler<CustomError> for CustomHandler {
///     fn source(_context: &CustomError) -> Option<&(dyn Error + 'static)> {
///         None
///     }
///
///     fn display(context: &CustomError, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "Error {}: {}", context.code, context.message)
///     }
///
///     fn debug(context: &CustomError, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(
///             f,
///             "CustomError {{ code: {}, message: {:?} }}",
///             context.code, context.message
///         )
///     }
/// }
/// ```
pub trait ContextHandler<C>: 'static {
    /// Returns the underlying error source for this context, if any.
    ///
    /// This method enables error chain traversal, allowing you to navigate from
    /// a context to its underlying cause. It's used when displaying the full
    /// error chain in a report.
    ///
    /// # Returns
    ///
    /// - `Some(&dyn Error)` if this context has an underlying error source
    /// - `None` if this context is a leaf in the error chain
    ///
    /// # Examples
    ///
    /// For types implementing `std::error::Error`, delegate to their `source`
    /// method:
    ///
    /// ```
    /// use std::error::Error;
    ///
    /// use rootcause_internals::handlers::ContextHandler;
    ///
    /// struct ErrorHandler;
    ///
    /// impl<C: Error> ContextHandler<C> for ErrorHandler {
    ///     fn source(context: &C) -> Option<&(dyn Error + 'static)> {
    ///         context.source()
    ///     }
    /// #   fn display(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #       write!(f, "{}", context)
    /// #   }
    /// #   fn debug(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #       write!(f, "{:?}", context)
    /// #   }
    /// }
    /// ```
    fn source(value: &C) -> Option<&(dyn core::error::Error + 'static)>;

    /// Formats the context using display-style formatting.
    ///
    /// This method is called when the context needs to be displayed as part of
    /// an error report. It should produce human-readable output suitable for
    /// end users.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause_internals::handlers::ContextHandler;
    ///
    /// struct DisplayHandler;
    ///
    /// impl<C: std::fmt::Display + std::fmt::Debug> ContextHandler<C> for DisplayHandler {
    ///     fn source(_context: &C) -> Option<&(dyn std::error::Error + 'static)> {
    ///         None
    ///     }
    ///
    ///     fn display(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         std::fmt::Display::fmt(context, f)
    ///     }
    /// #   fn debug(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #       std::fmt::Debug::fmt(context, f)
    /// #   }
    /// }
    /// ```
    fn display(value: &C, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// Formats the context using debug-style formatting.
    ///
    /// This method is called when the context needs to be debug-formatted. It
    /// should produce detailed output suitable for developers, potentially
    /// including internal state and implementation details.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause_internals::handlers::ContextHandler;
    ///
    /// struct DebugHandler;
    ///
    /// impl<C: std::fmt::Debug> ContextHandler<C> for DebugHandler {
    ///     fn source(_context: &C) -> Option<&(dyn std::error::Error + 'static)> {
    ///         None
    ///     }
    ///
    ///     fn display(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "Context of type `{}`", core::any::type_name::<C>())
    ///     }
    ///
    ///     fn debug(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         std::fmt::Debug::fmt(context, f)
    ///     }
    /// }
    /// ```
    fn debug(value: &C, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// Specifies the preferred formatting style when this context is embedded
    /// in a report.
    ///
    /// This method allows the handler to choose between display and debug
    /// formatting based on how the report itself is being formatted. The
    /// default implementation always returns
    /// [`FormattingFunction::Display`], meaning the context will use
    /// its [`display`](ContextHandler::display) method even when the report is
    /// being debug-formatted.
    ///
    /// # Arguments
    ///
    /// - `value`: The context value
    /// - `report_formatting_function`: How the report itself is being formatted
    ///   ([`Display`](core::fmt::Display) or [`Debug`](core::fmt::Debug))
    ///
    /// # Default Behavior
    ///
    /// The default implementation ignores the report's formatting style and
    /// always uses display formatting. This is the behavior of all built-in
    /// handlers.
    ///
    /// # Examples
    ///
    /// Custom handler that mirrors the report's formatting:
    ///
    /// ```
    /// use rootcause_internals::handlers::{
    ///     ContextFormattingStyle, ContextHandler, FormattingFunction,
    /// };
    ///
    /// struct MirrorHandler;
    ///
    /// impl<C: std::fmt::Display + std::fmt::Debug> ContextHandler<C> for MirrorHandler {
    ///     fn source(_context: &C) -> Option<&(dyn std::error::Error + 'static)> {
    ///         None
    ///     }
    ///
    ///     fn display(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         std::fmt::Display::fmt(context, f)
    ///     }
    ///
    ///     fn debug(context: &C, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         std::fmt::Debug::fmt(context, f)
    ///     }
    ///
    ///     fn preferred_formatting_style(
    ///         _value: &C,
    ///         report_formatting_function: FormattingFunction,
    ///     ) -> ContextFormattingStyle {
    ///         // Use the same formatting as the report
    ///         ContextFormattingStyle {
    ///             function: report_formatting_function,
    ///             follow_source: false,
    ///             follow_source_depth: None,
    ///         }
    ///     }
    /// }
    /// ```
    fn preferred_formatting_style(
        value: &C,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let _ = (value, report_formatting_function);
        ContextFormattingStyle::default()
    }
}

/// Trait for implementing custom formatting behavior for report attachments.
///
/// This trait defines how an attachment type should be formatted when displayed
/// or debugged as part of an error report. Unlike [`ContextHandler`], this
/// trait also allows specifying placement preferences (inline vs appendix).
///
/// # When to Implement
///
/// You typically don't need to implement this trait directly. The rootcause
/// library provides built-in handlers that cover most use cases. Implement this
/// trait when you need:
/// - Custom formatting for attachment types
/// - Special placement logic (e.g., large data in appendices)
/// - Dynamic formatting based on attachment content
///
/// # Required Methods
///
/// - [`display`](AttachmentHandler::display): Formats the attachment for
///   display output
/// - [`debug`](AttachmentHandler::debug): Formats the attachment for debug
///   output
///
/// # Optional Methods
///
/// - [`preferred_formatting_style`](AttachmentHandler::preferred_formatting_style):
///   Specifies formatting preferences including placement (inline/appendix) and
///   whether to use display or debug formatting. The default implementation prefers
///   inline display formatting.
///
/// # Examples
///
/// ```
/// use rootcause_internals::handlers::{
///     AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
///     FormattingFunction,
/// };
///
/// // Attachment type with potentially large data
/// struct LargeData {
///     records: Vec<String>,
/// }
///
/// // Handler that moves large attachments to appendix
/// struct LargeDataHandler;
///
/// impl AttachmentHandler<LargeData> for LargeDataHandler {
///     fn display(attachment: &LargeData, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{} records", attachment.records.len())
///     }
///
///     fn debug(attachment: &LargeData, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "LargeData {{ {} records }}", attachment.records.len())
///     }
///
///     fn preferred_formatting_style(
///         attachment: &LargeData,
///         _report_formatting: FormattingFunction,
///     ) -> AttachmentFormattingStyle {
///         if attachment.records.len() > 10 {
///             // Large data goes to appendix
///             AttachmentFormattingStyle {
///                 placement: AttachmentFormattingPlacement::Appendix {
///                     appendix_name: "Large Data Records".into(),
///                 },
///                 function: FormattingFunction::Display,
///                 priority: 0,
///             }
///         } else {
///             // Small data shows inline
///             AttachmentFormattingStyle::default()
///         }
///     }
/// }
/// ```
pub trait AttachmentHandler<A>: 'static {
    /// Formats the attachment using display-style formatting.
    ///
    /// This method is called when the attachment needs to be displayed as part
    /// of an error report. It should produce human-readable output suitable
    /// for end users.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause_internals::handlers::AttachmentHandler;
    ///
    /// struct ConfigData {
    ///     key: String,
    ///     value: String,
    /// }
    ///
    /// struct ConfigHandler;
    ///
    /// impl AttachmentHandler<ConfigData> for ConfigHandler {
    ///     fn display(attachment: &ConfigData, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "{} = {}", attachment.key, attachment.value)
    ///     }
    /// #   fn debug(attachment: &ConfigData, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #       write!(f, "ConfigData {{ key: {:?}, value: {:?} }}", attachment.key, attachment.value)
    /// #   }
    /// }
    /// ```
    fn display(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// Formats the attachment using debug-style formatting.
    ///
    /// This method is called when the attachment needs to be debug-formatted.
    /// It should produce detailed output suitable for developers.
    fn debug(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// Specifies the preferred formatting style and placement for this
    /// attachment.
    ///
    /// This method allows the handler to control:
    /// - **Placement**: Whether the attachment appears inline, in an appendix,
    ///   or is hidden
    /// - **Formatting**: Whether to use display or debug formatting
    /// - **Priority**: The order in which attachments are displayed (higher =
    ///   earlier)
    ///
    /// The default implementation returns inline display formatting with
    /// priority 0.
    ///
    /// # Examples
    ///
    /// Attachment that hides sensitive data:
    ///
    /// ```
    /// use rootcause_internals::handlers::{
    ///     AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
    ///     FormattingFunction,
    /// };
    ///
    /// struct ApiKey(String);
    ///
    /// struct SecureHandler;
    ///
    /// impl AttachmentHandler<ApiKey> for SecureHandler {
    ///     fn display(_attachment: &ApiKey, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "[REDACTED]")
    ///     }
    ///
    ///     fn debug(_attachment: &ApiKey, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "ApiKey([REDACTED])")
    ///     }
    ///
    ///     fn preferred_formatting_style(
    ///         _attachment: &ApiKey,
    ///         _report_formatting: FormattingFunction,
    ///     ) -> AttachmentFormattingStyle {
    ///         // Hide this attachment completely in production
    ///         AttachmentFormattingStyle {
    ///             placement: AttachmentFormattingPlacement::Hidden,
    ///             function: FormattingFunction::Display,
    ///             priority: 0,
    ///         }
    ///     }
    /// }
    /// ```
    fn preferred_formatting_style(
        value: &A,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let _ = (value, report_formatting_function);
        AttachmentFormattingStyle::default()
    }
}

/// Formatting preferences for a context when displayed in a report.
///
/// This struct allows a [`ContextHandler`] to specify how it prefers to be
/// formatted when its context is displayed as part of an error report. The
/// formatting system may or may not respect these preferences depending on the
/// formatter implementation.
///
/// # Fields
///
/// - `function`: Whether to use [`Display`](core::fmt::Display) or
///   [`Debug`](core::fmt::Debug) formatting
///
/// # Default
///
/// The default is to use [`FormattingFunction::Display`].
///
/// # Examples
///
/// ```
/// use rootcause_internals::handlers::{ContextFormattingStyle, FormattingFunction};
///
/// // Prefer display formatting (the default)
/// let style = ContextFormattingStyle::default();
/// assert_eq!(style.function, FormattingFunction::Display);
///
/// // Explicitly request debug formatting
/// let debug_style = ContextFormattingStyle {
///     function: FormattingFunction::Debug,
///     follow_source: false,
///     follow_source_depth: None,
/// };
/// ```
#[allow(missing_copy_implementations)]
#[derive(Clone, Debug, Default)]
pub struct ContextFormattingStyle {
    /// The preferred formatting function to use
    pub function: FormattingFunction,
    /// Whether to follow the [`core::error::Error`] source chain when
    /// formatting
    pub follow_source: bool,
    /// The maximum depth to follow the [`core::error::Error`] source chain when
    /// formatting. Setting to `None` means unlimited depth.
    pub follow_source_depth: Option<usize>,
}

/// Formatting preferences for an attachment when displayed in a report.
///
/// This struct allows an [`AttachmentHandler`] to specify how and where it
/// prefers to be displayed when included in an error report. The formatting
/// system may or may not respect these preferences depending on the formatter
/// implementation.
///
/// # Fields
///
/// - `placement`: Where the attachment should appear (inline, appendix, hidden,
///   etc.)
/// - `function`: Whether to use [`Display`](core::fmt::Display) or
///   [`Debug`](core::fmt::Debug) formatting
/// - `priority`: Display order preference (higher values appear earlier)
///
/// # Default
///
/// The default is inline display formatting with priority 0.
///
/// # Examples
///
/// ```
/// use rootcause_internals::handlers::{
///     AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction,
/// };
///
/// // Default: inline display formatting
/// let style = AttachmentFormattingStyle::default();
/// assert_eq!(style.placement, AttachmentFormattingPlacement::Inline);
/// assert_eq!(style.function, FormattingFunction::Display);
/// assert_eq!(style.priority, 0);
///
/// // High-priority attachment in appendix
/// let appendix_style = AttachmentFormattingStyle {
///     placement: AttachmentFormattingPlacement::Appendix {
///         appendix_name: "Stack Trace".into(),
///     },
///     function: FormattingFunction::Debug,
///     priority: 10,
/// };
/// ```
#[derive(Clone, Debug, Default)]
pub struct AttachmentFormattingStyle {
    /// The preferred attachment placement
    pub placement: AttachmentFormattingPlacement,
    /// The preferred formatting function to use
    pub function: FormattingFunction,
    /// The preferred formatting priority. Higher priority means
    /// a preference for being printed earlier in the report
    pub priority: i32,
}

/// Specifies whether to use display or debug formatting for a context or
/// attachment.
///
/// This enum is used by handlers to indicate their formatting preference when
/// a context or attachment is displayed as part of an error report. The actual
/// formatting system may or may not respect this preference.
///
/// # Variants
///
/// - **`Display`** (default): Use the `display` method
/// - **`Debug`**: Use the `debug` method
///
/// # Examples
///
/// ```
/// use rootcause_internals::handlers::FormattingFunction;
///
/// let display_formatting = FormattingFunction::Display;
/// let debug_formatting = FormattingFunction::Debug;
///
/// // Display is the default
/// assert_eq!(FormattingFunction::default(), FormattingFunction::Display);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum FormattingFunction {
    /// Prefer display formatting via the `display` method.
    #[default]
    Display,
    /// Prefer debug formatting via the `debug` method.
    Debug,
}

/// Specifies where an attachment should be placed when displayed in a report.
///
/// This enum allows attachments to indicate their preferred placement in error
/// reports. Different placements are suitable for different types of content:
///
/// - **Inline**: Short, contextual information that flows with the error
///   message
/// - **InlineWithHeader**: Multi-line content that needs a header for clarity
/// - **Appendix**: Large or detailed content better suited to a separate
///   section
/// - **Opaque**: Content that shouldn't be shown but should be counted
/// - **Hidden**: Content that shouldn't appear at all
///
/// The actual formatting system may or may not respect these preferences
/// depending on the implementation.
///
/// # Examples
///
/// ```
/// use rootcause_internals::handlers::AttachmentFormattingPlacement;
///
/// // Default is inline
/// let inline = AttachmentFormattingPlacement::default();
/// assert_eq!(inline, AttachmentFormattingPlacement::Inline);
///
/// // Attachment with header
/// let with_header = AttachmentFormattingPlacement::InlineWithHeader {
///     header: "Request Details".into(),
/// };
///
/// // Large content in appendix
/// let appendix = AttachmentFormattingPlacement::Appendix {
///     appendix_name: "Full Stack Trace".into(),
/// };
///
/// // Sensitive data that should be hidden
/// let hidden = AttachmentFormattingPlacement::Hidden;
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum AttachmentFormattingPlacement {
    /// Display the attachment inline with the error message.
    ///
    /// Suitable for short, contextual information that naturally flows with the
    /// error text. This is the default placement.
    #[default]
    Inline,

    /// Display the attachment inline but preceded by a header.
    ///
    /// Useful for multi-line content that benefits from a descriptive header,
    /// such as configuration snippets or multi-field data structures.
    InlineWithHeader {
        /// The header text to display above the attachment
        header: Cow<'static, str>,
    },

    /// Display the attachment in a separate appendix section.
    ///
    /// Suitable for large or detailed content that would disrupt the flow of
    /// the main error message, such as full stack traces, large data dumps,
    /// or detailed diagnostic information.
    Appendix {
        /// The name of the appendix section for this attachment
        appendix_name: Cow<'static, str>,
    },

    /// Don't display the attachment, but count it in a summary.
    ///
    /// The attachment won't be shown directly, but may appear in a message like
    /// "3 additional opaque attachments". Useful for numerous low-priority
    /// attachments that would clutter the output.
    Opaque,

    /// Don't display the attachment at all.
    ///
    /// The attachment is completely hidden and won't appear in any form. Useful
    /// for sensitive data that should be excluded from error reports, or for
    /// attachments meant only for programmatic access.
    Hidden,
}
