//! Handlers used to implement or override the behavior of
//! [`core::error::Error`], [`core::fmt::Display`] and [`core::fmt::Debug`] when
//! creating an attachment or report.

/// Handler used to implement or override the behavior of
/// [`core::error::Error`], [`core::fmt::Display`] and [`core::fmt::Debug`] when
/// creating a report.
pub trait ContextHandler<C>: 'static {
    /// The function used when calling [`RawReportRef::context_source`]
    ///
    /// [`RawReportRef::context_source`]: crate::report::RawReportRef::context_source
    fn source(value: &C) -> Option<&(dyn core::error::Error + 'static)>;

    /// The function used when calling [`RawReportRef::context_display`]
    ///
    /// [`RawReportRef::context_display`]: crate::report::RawReportRef::context_display
    fn display(value: &C, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// The function used when calling [`RawReportRef::context_debug`]
    ///
    /// [`RawReportRef::context_debug`]: crate::report::RawReportRef::context_debug
    fn debug(value: &C, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// The formatting style preferred by the context when formatted as part of
    /// a report.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this context
    ///   will be embedded is being formatted using [`Display`] formatting or
    ///   [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    fn preferred_formatting_style(
        value: &C,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let _ = (value, report_formatting_function);
        ContextFormattingStyle::default()
    }
}

/// Handler used to implement or override the behavior of [`core::fmt::Display`]
/// and [`core::fmt::Debug`] when creating an attachment.
pub trait AttachmentHandler<A>: 'static {
    /// The function used when calling [`RawAttachmentRef::attachment_display`]
    ///
    /// [`RawAttachmentRef::attachment_display`]: crate::attachment::RawAttachmentRef::attachment_display
    fn display(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// The function used when calling [`RawAttachmentRef::attachment_debug`]
    ///
    /// [`RawAttachmentRef::attachment_debug`]: crate::attachment::RawAttachmentRef::attachment_debug
    fn debug(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;

    /// The function used when calling
    /// [`RawAttachmentRef::preferred_formatting_style`]
    ///
    /// The formatting style preferred by the attachment when formatted as part
    /// of a report.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this
    ///   attachment will be embedded is being formatted using [`Display`]
    ///   formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    ///
    /// [`RawAttachmentRef::preferred_formatting_style`]: crate::attachment::RawAttachmentRef::preferred_formatting_style
    fn preferred_formatting_style(
        value: &A,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let _ = (value, report_formatting_function);
        AttachmentFormattingStyle::default()
    }
}

/// Struct for contexts to specify how they prefer to be
/// formatted when they are formatted as part of a report.
///
/// Whether this is respected or not, and what constitutes an "appendix" is
/// up to the code that does the formatting for reports.
#[derive(Copy, Clone, Debug, Default)]
pub struct ContextFormattingStyle {
    /// The preferred formatting function to use
    pub function: FormattingFunction,
}

/// Struct for attachments to specify how and where the attachment prefers to be
/// formatted when they are formatted as part of a report.
///
/// Whether this is respected or not, and what constitutes an "appendix" is
/// up to the code that does the formatting for reports.
#[derive(Copy, Clone, Debug, Default)]
pub struct AttachmentFormattingStyle {
    /// The preferred attachment placement
    pub placement: AttachmentFormattingPlacement,
    /// The preferred formatting function to use
    pub function: FormattingFunction,
    /// The preferred formatting priority. Higher priority means
    /// a preference for being printed earlier in the report
    pub priority: i32,
}

/// Enum for deciding which function to prefer when a context/attachment
/// is formatted as part of a report.
///
/// Whether this is respected or not is up to the code that does the formatting
/// for reports.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum FormattingFunction {
    /// The context prefers to be rendered inline using the
    /// [`ContextHandler::display`]/[`AttachmentHandler::display`] methods.
    #[default]
    Display,
    /// The context prefers to be rendered inline using the
    /// [`ContextHandler::debug`]/[`AttachmentHandler::debug`] methods
    Debug,
}

/// Enum for attachments to specify the placement they prefer to be
/// formatted when they are formatted as part of a report.
///
/// Whether this is respected or not, and what constitutes an "appendix" is
/// up to the code that does the formatting for reports.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum AttachmentFormattingPlacement {
    /// The attachment prefers to be rendered inline
    #[default]
    Inline,
    /// The attachment prefers to be rendered inline under a sub-header. This
    /// can be useful for attachments rendering as multiple lines
    InlineWithHeader {
        /// The header used to render the attachment below
        header: &'static str,
    },
    /// The attachment prefers to be rendered as an appendix
    Appendix {
        /// In case the report formatter uses named appendices, then this
        /// is the name preferred for this attachment
        appendix_name: &'static str,
    },
    /// The attachment prefers not to be rendered, but would like to show up in
    /// an "$N additional opaque attachments" message
    Opaque,
    /// The attachment prefers not to be rendered at all
    Hidden,
}
