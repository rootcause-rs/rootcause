//! Default report formatter implementation for the rootcause error handling
//! library.
//!
//! This module provides [`DefaultReportFormatter`], a comprehensive and
//! configurable report formatter that handles the visual presentation of error
//! reports with hierarchical structures, attachments, and appendices.
//!
//! The formatter supports multiple output styles:
//! - **Unicode with ANSI colors** ([`DefaultReportFormatter::UNICODE_ANSI`]) -
//!   Rich visual experience for modern terminals
//! - **ASCII-only** ([`DefaultReportFormatter::ASCII_NO_ANSI`]) - Compatible
//!   with basic terminals and text-only outputs
//!
//! # Usage
//!
//! This formatter is the one used by default when no other formatter is
//! registered. It is also possible to explicitly register it:
//!
//! ```rust
//! use rootcause::hooks::{
//!     builtin_hooks::report_formatter::DefaultReportFormatter,
//!     report_formatting::register_report_formatter_hook,
//! };
//!
//! // Use the default Unicode + ANSI configuration
//! register_report_formatter_hook(DefaultReportFormatter::default());
//!
//! // Or use ASCII-only for compatibility
//! register_report_formatter_hook(DefaultReportFormatter::ASCII_NO_ANSI);
//! ```
//!
//! # Configuration Structures
//!
//! The formatter uses a hierarchy of configuration structures:
//! - [`LineFormatting`] - Basic line prefix/suffix formatting
//! - [`ItemFormatting`] - Multi-line item formatting with different rules for
//!   first/middle/last lines
//! - [`NodeConfig`] - Hierarchical node formatting with header and child
//!   indentation
//! - [`DefaultReportFormatter`] - Main configuration struct combining all
//!   formatting options

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    any::Any,
    fmt::{self, Formatter, Write},
};

use indexmap::IndexMap;
use rootcause_internals::handlers::{
    AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction,
};

use crate::{
    ReportRef,
    hooks::report_formatting::ReportFormatterHook,
    markers::{Local, Uncloneable},
    report_attachment::ReportAttachmentRef,
};

/// The default report formatter implementation that provides comprehensive
/// formatting for error reports with configurable styling and layout options.
///
/// This formatter supports both ASCII-only output (suitable for environments
/// without Unicode or ANSI support) and Unicode output with ANSI color codes
/// for enhanced visual presentation. It handles hierarchical report structures
/// with contexts, attachments, and appendices.
///
/// # Examples
///
/// Basic usage with default formatting:
/// ```rust
/// use rootcause::hooks::{
///     builtin_hooks::report_formatter::DefaultReportFormatter,
///     report_formatting::register_report_formatter_hook,
/// };
///
/// register_report_formatter_hook(DefaultReportFormatter::default());
/// // Use with report formatting system
/// ```
///
/// Using ASCII-only formatting for compatibility:
/// ```rust
/// use rootcause::hooks::{
///     builtin_hooks::report_formatter::DefaultReportFormatter,
///     report_formatting::register_report_formatter_hook,
/// };
///
/// register_report_formatter_hook(DefaultReportFormatter::ASCII_NO_ANSI);
/// // Use in environments without Unicode/ANSI support
/// ```
pub struct DefaultReportFormatter {
    /// Header text displayed at the beginning of report output
    pub report_header: &'static str,

    /// Prefix applied to every line of report content
    pub report_line_prefix_always: &'static str,

    /// Prefix applied to every line of appendix content
    pub appendix_line_prefix_always: &'static str,

    /// Formatting configuration for standalone report nodes, i.e. those without
    /// siblings
    pub report_node_standalone_formatting: NodeConfig,

    /// Formatting configuration for report nodes that have siblings below them
    pub report_node_middle_formatting: NodeConfig,

    /// Formatting configuration for the last report node in a sequence of more
    /// than one sibling
    pub report_node_last_formatting: NodeConfig,

    /// Formatting configuration for attachment items that output in [`Inline`]
    /// mode and have more data below them from the same report node
    ///
    /// [`Inline`]: AttachmentFormattingPlacement::Inline
    pub attachment_inline_formatting_middle: ItemFormatting,

    /// Formatting configuration for attachment items that output in [`Inline`]
    /// mode and are the last piece of data for the report node
    ///
    /// [`Inline`]: AttachmentFormattingPlacement::Inline
    pub attachment_inline_formatting_last: ItemFormatting,

    /// Formatting configuration for attachment items that output in
    /// [`InlineWithHeader`] mode and have more data below them from the
    /// same report node
    ///
    /// [`InlineWithHeader`]: AttachmentFormattingPlacement::InlineWithHeader
    pub attachment_headered_formatting_middle: NodeConfig,

    /// Formatting configuration for attachment items that output in
    /// [`InlineWithHeader`] mode and are the last piece of data for the
    /// report node
    ///
    /// [`InlineWithHeader`]: AttachmentFormattingPlacement::InlineWithHeader
    pub attachment_headered_formatting_last: NodeConfig,

    /// Formatting configuration for the data of the attachments that output in
    /// [`InlineWithHeader`] mode.
    ///
    /// [`InlineWithHeader`]: AttachmentFormattingPlacement::InlineWithHeader
    pub attachment_headered_formatting_data: ItemFormatting,

    /// Optional prefix text for the data of the attachments that output in
    /// [`InlineWithHeader`] mode.
    ///
    /// [`InlineWithHeader`]: AttachmentFormattingPlacement::InlineWithHeader
    pub attachment_headered_data_prefix: Option<&'static str>,

    /// Optional suffix text for the data of the attachments that output in
    /// [`InlineWithHeader`] mode.
    ///
    /// [`InlineWithHeader`]: AttachmentFormattingPlacement::InlineWithHeader
    pub attachment_headered_data_suffix: Option<&'static str>,

    /// The formatting for the "see also" notice when this notice *is not* the
    /// last piece of data to render for the report node.
    ///
    /// The "see also" notice occur when an attachment item outputs in
    /// [`Appendix`] mode.
    ///
    /// [`Appendix`]: AttachmentFormattingPlacement::Appendix
    pub notice_see_also_middle_formatting: LineFormatting,

    /// The formatting for the "see also" notice when this notice *is* the last
    /// piece of data to render for the report node.
    ///
    /// The "see also" notice occur when an attachment item outputs in
    /// [`Appendix`] mode.
    ///
    /// [`Appendix`]: AttachmentFormattingPlacement::Appendix
    pub notice_see_also_last_formatting: LineFormatting,

    /// The formatting for the `N additional opaque attachments` notice when
    /// this notice *is not* the last piece of data to render for the report
    /// node.
    ///
    /// This notice occurs when one or more attachment items output in
    /// [`Opaque`] mode.
    ///
    /// [`Opaque`]: AttachmentFormattingPlacement::Opaque
    pub notice_opaque_middle_formatting: LineFormatting,

    /// The formatting for the `N additional opaque attachments` notice when
    /// this notice *is* the last piece of data to render for the report
    /// node.
    ///
    /// This notice occurs when one or more attachment items output in
    /// [`Opaque`] mode.
    ///
    /// [`Opaque`]: AttachmentFormattingPlacement::Opaque
    pub notice_opaque_last_formatting: LineFormatting,

    /// Optional separator inserted between attachments and child contexts
    pub attachment_child_separator: Option<&'static str>,

    /// Optional separator inserted between sibling child contexts
    pub child_child_separator: Option<&'static str>,

    /// Separator text inserted between multiple reports
    pub report_report_separator: &'static str,

    /// Separator text inserted between report content and appendices
    pub report_appendix_separator: &'static str,

    /// Separator text inserted between multiple appendices
    pub appendix_appendix_separator: &'static str,

    /// Formatting configuration for appendix headers
    pub appendix_header: LineFormatting,

    /// Formatting configuration for appendix body content
    pub appendix_body: ItemFormatting,

    /// Footer text displayed after all appendices
    pub appendices_footer: &'static str,

    /// Footer text displayed when there are no appendices
    pub no_appendices_footer: &'static str,
}

impl DefaultReportFormatter {
    /// A predefined configuration that uses only ASCII characters without ANSI
    /// color codes.
    ///
    /// This configuration is suitable for environments that don't support
    /// Unicode characters or ANSI color codes, such as basic terminals, log
    /// files, or text-only outputs. Uses simple ASCII box-drawing
    /// alternatives like `|-`, `o`, and `--`.
    pub const ASCII_NO_ANSI: Self = Self {
        report_header: "\n",
        report_line_prefix_always: "",
        appendix_line_prefix_always: "",
        report_node_standalone_formatting: NodeConfig::new(
            ("o  ", "\n"),
            ("o  ", "\n"),
            ("|  ", "\n"),
            ("|  ", "\n"),
            "",
        ),
        report_node_middle_formatting: NodeConfig::new(
            ("|--> o  ", "\n"),
            ("|--> o  ", "\n"),
            ("|    |  ", "\n"),
            ("|    |  ", "\n"),
            "|    ",
        ),
        report_node_last_formatting: NodeConfig::new(
            (r"|--> o  ", "\n"),
            (r"|--> o  ", "\n"),
            ("     |  ", "\n"),
            ("     |  ", "\n"),
            "     ",
        ),
        attachment_inline_formatting_middle: ItemFormatting::new(
            ("|- ", "\n"),
            ("|- ", "\n"),
            ("|  ", "\n"),
            ("|  ", "\n"),
        ),
        attachment_inline_formatting_last: ItemFormatting::new(
            (r"|- ", "\n"),
            (r"|- ", "\n"),
            ("   ", "\n"),
            ("   ", "\n"),
        ),
        attachment_headered_formatting_middle: NodeConfig::new(
            (r"|- ", "\n"),
            ("|- ", "\n"),
            ("|  ", "\n"),
            ("|  ", "\n"),
            "| ",
        ),
        attachment_headered_formatting_last: NodeConfig::new(
            (r"|- ", "\n"),
            (r"|- ", "\n"),
            ("  ", "\n"),
            ("  ", "\n"),
            "  ",
        ),
        attachment_headered_formatting_data: ItemFormatting::new(
            (r"|- ", "\n"),
            ("|- ", "\n"),
            ("|- ", "\n"),
            (r"|- ", "\n"),
        ),
        attachment_headered_data_prefix: None,
        attachment_headered_data_suffix: None,
        notice_see_also_middle_formatting: LineFormatting::new("|- See ", " below\n"),
        notice_see_also_last_formatting: LineFormatting::new(r"|- See ", " below\n"),
        notice_opaque_middle_formatting: LineFormatting::new("|- ", "\n"),
        notice_opaque_last_formatting: LineFormatting::new(r"|- ", "\n"),
        attachment_child_separator: None,
        child_child_separator: None,
        report_report_separator: "--\n",
        report_appendix_separator: "----------------------------------------\n",
        appendix_appendix_separator: "----------------------------------------\n",
        appendix_header: LineFormatting::new(" ", "\n\n"),
        appendix_body: ItemFormatting::new((" ", "\n"), (" ", "\n"), (" ", "\n"), (" ", "\n")),
        appendices_footer: "----------------------------------------\n",
        no_appendices_footer: "",
    };
    /// The default formatter configuration, which is an alias for
    /// [`UNICODE_ANSI`](Self::UNICODE_ANSI).
    ///
    /// This provides the best visual experience for terminals that support
    /// Unicode and ANSI colors.
    pub const DEFAULT: Self = Self::UNICODE_ANSI;
    /// A predefined configuration that uses Unicode box-drawing characters with
    /// ANSI color codes.
    ///
    /// This configuration provides the richest visual experience with Unicode
    /// box-drawing characters (like `├`, `╰`, `│`) and ANSI color codes for
    /// enhanced readability. Suitable for modern terminals and development
    /// environments.
    pub const UNICODE_ANSI: Self = Self {
        report_header: "\n",
        report_line_prefix_always: " ",
        appendix_line_prefix_always: "",
        report_node_standalone_formatting: NodeConfig::new(
            ("\x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("\x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("│ \x1b[1;97m", "\x1b[0m\n"),
            ("│ \x1b[1;97m", "\x1b[0m\n"),
            "",
        ),
        report_node_middle_formatting: NodeConfig::new(
            ("├─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("├─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("│  │ \x1b[1;97m", "\x1b[0m\n"),
            ("│  │ \x1b[1;97m", "\x1b[0m\n"),
            "│  ",
        ),
        report_node_last_formatting: NodeConfig::new(
            ("╰─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("╰─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("   │ \x1b[1;97m", "\x1b[0m\n"),
            ("   │ \x1b[1;97m", "\x1b[0m\n"),
            "   ",
        ),
        attachment_inline_formatting_middle: ItemFormatting::new(
            ("├ ", "\n"),
            ("├ ", "\n"),
            ("│ ", "\n"),
            ("│ ", "\n"),
        ),
        attachment_inline_formatting_last: ItemFormatting::new(
            ("╰ ", "\n"),
            ("╰ ", "\n"),
            ("  ", "\n"),
            ("  ", "\n"),
        ),
        attachment_headered_formatting_middle: NodeConfig::new(
            ("├ \x1b[4m", "\x1b[0m\n"),
            ("├ \x1b[4m", "\x1b[0m\n"),
            ("│\x1b[4m", "\x1b[0m\n"),
            ("│\x1b[4m", "\x1b[0m\n"),
            "│ ",
        ),
        attachment_headered_formatting_last: NodeConfig::new(
            ("╰ \x1b[4m", "\x1b[0m\n"),
            ("╰ \x1b[4m", "\x1b[0m\n"),
            (" \x1b[4m", "\x1b[0m\n"),
            (" \x1b[4m", "\x1b[0m\n"),
            "  ",
        ),
        attachment_headered_formatting_data: ItemFormatting::new(
            ("│ ", "\n"),
            ("│ ", "\n"),
            ("│ ", "\n"),
            ("│ ", "\n"),
        ),
        attachment_headered_data_prefix: None,
        attachment_headered_data_suffix: Some("╰─\n"),
        notice_see_also_middle_formatting: LineFormatting::new("├ See \x1b[4m", "\x1b[0m below\n"),
        notice_see_also_last_formatting: LineFormatting::new("╰ See \x1b[4m", "\x1b[0m below\n"),
        notice_opaque_middle_formatting: LineFormatting::new("├ ", "\n"),
        notice_opaque_last_formatting: LineFormatting::new("╰ ", "\n"),
        attachment_child_separator: Some("│\n"),
        child_child_separator: Some("│\n"),
        report_report_separator: "━━\n",
        report_appendix_separator: "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
        appendix_appendix_separator: "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
        appendix_header: LineFormatting::new(" \x1b[4m", "\x1b[0m\n\n"),
        appendix_body: ItemFormatting::new((" ", "\n"), (" ", "\n"), (" ", "\n"), (" ", "\n")),
        appendices_footer: "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
        no_appendices_footer: "",
    };
}

impl Default for DefaultReportFormatter {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Configuration for formatting individual lines with prefix and suffix text.
///
/// This is the fundamental building block for all report formatting, allowing
/// each line to be decorated with consistent prefix and suffix strings that
/// can include tree characters, ANSI color codes, or other visual elements.
///
/// # Examples
///
/// Creating line formatting with tree characters:
/// ```rust
/// use rootcause::hooks::builtin_hooks::report_formatter::LineFormatting;
///
/// let formatting = LineFormatting::new("├─ ", "\n");
/// // This will format lines as: "├─ content\n"
/// ```
///
/// Creating line formatting with ANSI colors:
/// ```rust
/// use rootcause::hooks::builtin_hooks::report_formatter::LineFormatting;
///
/// let formatting = LineFormatting::new("\x1b[31m● ", "\x1b[0m\n");
/// // This will format lines as: "\x1b[31m● content\x1b[0m\n" (red bullet)
/// ```
#[derive(Copy, Clone)]
pub struct LineFormatting {
    /// Text prefix prepended to the beginning of each line
    pub prefix: &'static str,
    /// Text suffix appended to the end of each line
    pub suffix: &'static str,
}

impl LineFormatting {
    /// Creates a new line formatting configuration with the specified prefix
    /// and suffix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Text to prepend to each formatted line
    /// * `suffix` - Text to append to each formatted line
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::builtin_hooks::report_formatter::LineFormatting;
    ///
    /// let formatting = LineFormatting::new("→ ", "\n");
    /// ```
    pub const fn new(prefix: &'static str, suffix: &'static str) -> Self {
        Self { prefix, suffix }
    }
}
/// Configuration for formatting items that may span multiple lines, with
/// different formatting rules based on the line's position within the item.
///
/// This allows for sophisticated formatting where the first, middle, and last
/// lines of multi-line content can have different prefixes and suffixes,
/// enabling tree-like structures and visual connection between related lines.
///
/// # Examples
///
/// Creating formatting for a tree structure:
/// ```rust
/// use rootcause::hooks::builtin_hooks::report_formatter::ItemFormatting;
///
/// let formatting = ItemFormatting::new(
///     ("● ", "\n"), // single line: "● content\n"
///     ("┌ ", "\n"), // first line:  "┌ first line\n"
///     ("│ ", "\n"), // middle line: "│ middle line\n"
///     ("└ ", "\n"), // last line:   "└ last line\n"
/// );
/// ```
#[derive(Copy, Clone)]
pub struct ItemFormatting {
    /// Formatting applied when the item consists of only one line
    pub standalone_line: LineFormatting,
    /// Formatting applied to the first line of a multi-line item
    pub first_line: LineFormatting,
    /// Formatting applied to middle lines of a multi-line item
    pub middle_line: LineFormatting,
    /// Formatting applied to the last line of a multi-line item
    pub last_line: LineFormatting,
}

impl ItemFormatting {
    /// Creates a new item formatting configuration with line-specific
    /// formatting rules.
    ///
    /// # Arguments
    ///
    /// * `standalone_line` - Prefix and suffix for single-line items: `(prefix,
    ///   suffix)`
    /// * `first_line` - Prefix and suffix for the first line of multi-line
    ///   items: `(prefix, suffix)`
    /// * `middle_line` - Prefix and suffix for middle lines of multi-line
    ///   items: `(prefix, suffix)`
    /// * `last_line` - Prefix and suffix for the last line of multi-line items:
    ///   `(prefix, suffix)`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::builtin_hooks::report_formatter::ItemFormatting;
    ///
    /// let formatting = ItemFormatting::new(
    ///     ("■ ", "\n"), // standalone: "■ content\n"
    ///     ("┌ ", "\n"), // first:      "┌ first line\n"
    ///     ("│ ", "\n"), // middle:     "│ middle line\n"
    ///     ("└ ", "\n"), // last:       "└ last line\n"
    /// );
    /// ```
    pub const fn new(
        standalone_line: (&'static str, &'static str),
        first_line: (&'static str, &'static str),
        middle_line: (&'static str, &'static str),
        last_line: (&'static str, &'static str),
    ) -> Self {
        Self {
            standalone_line: LineFormatting::new(standalone_line.0, standalone_line.1),
            first_line: LineFormatting::new(first_line.0, first_line.1),
            middle_line: LineFormatting::new(middle_line.0, middle_line.1),
            last_line: LineFormatting::new(last_line.0, last_line.1),
        }
    }
}

/// Configuration for formatting hierarchical nodes that contain both header
/// content and child elements with specific indentation.
///
/// A node represents a hierarchical element in the report structure, such as a
/// context or a headered attachment, that has both a header line and potential
/// child content that needs to be indented relative to the header.
///
/// # Examples
///
/// Creating a node configuration with tree-like formatting:
/// ```rust
/// use rootcause::hooks::builtin_hooks::report_formatter::{ItemFormatting, NodeConfig};
///
/// let config = NodeConfig::new(
///     ("● ", "\n"),  // standalone header
///     ("├─ ", "\n"), // first line of multi-line header
///     ("│  ", "\n"), // middle lines of multi-line header
///     ("╰─ ", "\n"), // last line of multi-line header
///     "   ",         // prefix for child content
/// );
/// ```
#[derive(Copy, Clone)]
pub struct NodeConfig {
    /// Formatting configuration for the node's header content
    pub header: ItemFormatting,
    /// Text prefix applied to all child content lines for proper indentation
    pub prefix_children: &'static str,
}

impl NodeConfig {
    /// Creates a new node configuration with header formatting and child
    /// indentation.
    ///
    /// # Arguments
    ///
    /// * `standalone_line` - Prefix and suffix for single-line headers:
    ///   `(prefix, suffix)`
    /// * `first_line` - Prefix and suffix for the first line of multi-line
    ///   headers: `(prefix, suffix)`
    /// * `middle_line` - Prefix and suffix for middle lines of multi-line
    ///   headers: `(prefix, suffix)`
    /// * `last_line` - Prefix and suffix for the last line of multi-line
    ///   headers: `(prefix, suffix)`
    /// * `prefix_children` - Text prefix applied to all child content for
    ///   proper indentation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::builtin_hooks::report_formatter::NodeConfig;
    ///
    /// let config = NodeConfig::new(
    ///     ("◆ ", "\n"), // standalone header
    ///     ("◆ ", "\n"), // first line of a multi-line header
    ///     ("│ ", "\n"), // middle lines of multi-line header
    ///     ("╰ ", "\n"), // last line of multi-line header
    ///     "  ",         // indent for children
    /// );
    /// ```
    pub const fn new(
        standalone_line: (&'static str, &'static str),
        first_line: (&'static str, &'static str),
        middle_line: (&'static str, &'static str),
        last_line: (&'static str, &'static str),
        prefix_children: &'static str,
    ) -> Self {
        Self {
            header: ItemFormatting::new(standalone_line, first_line, middle_line, last_line),
            prefix_children,
        }
    }
}
type Appendices<'a> = IndexMap<
    &'static str,
    Vec<(ReportAttachmentRef<'a, dyn Any>, FormattingFunction)>,
    rustc_hash::FxBuildHasher,
>;

struct DefaultFormatterState<'a, 'b> {
    config: &'a DefaultReportFormatter,
    appendices: Appendices<'a>,
    line_prefix: String,
    formatter: &'a mut Formatter<'b>,
    report_formatting_function: FormattingFunction,
}

impl ReportFormatterHook for DefaultReportFormatter {
    fn format_reports(
        &self,
        reports: &[ReportRef<'_, dyn Any, Uncloneable, Local>],
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result {
        formatter.write_str(self.report_header)?;
        DefaultFormatterState::new(self, formatter, report_formatting_function)
            .format_reports(reports)
    }
}

type TmpValueBuffer = String;
type TmpAttachmentsBuffer<'a> = Vec<(AttachmentFormattingStyle, ReportAttachmentRef<'a, dyn Any>)>;

impl<'a, 'b> DefaultFormatterState<'a, 'b> {
    fn new(
        config: &'a DefaultReportFormatter,
        formatter: &'a mut Formatter<'b>,
        report_formatting_function: FormattingFunction,
    ) -> Self {
        Self {
            config,
            appendices: IndexMap::default(),
            line_prefix: String::new(),
            formatter,
            report_formatting_function,
        }
    }

    fn format_with_line_prefix(&mut self, line: &str) -> fmt::Result {
        self.formatter.write_str(&self.line_prefix)?;
        self.formatter.write_str(line)?;
        Ok(())
    }

    fn format_node(
        &mut self,
        tmp_value_buffer: &mut TmpValueBuffer,
        formatting: &NodeConfig,
        value: impl fmt::Display + fmt::Debug,
        function: FormattingFunction,
        children: impl FnOnce(&mut Self, &mut String) -> fmt::Result,
    ) -> fmt::Result {
        self.format_item(tmp_value_buffer, &formatting.header, value, function)?;

        let len_before = self.line_prefix.len();
        self.line_prefix.push_str(formatting.prefix_children);
        let result = children(self, tmp_value_buffer);
        self.line_prefix.truncate(len_before);
        result
    }

    fn format_item(
        &mut self,
        tmp_value_buffer: &mut TmpValueBuffer,
        formatting: &ItemFormatting,
        value: impl fmt::Display + fmt::Debug,
        function: FormattingFunction,
    ) -> fmt::Result {
        let mut is_first = true;
        tmp_value_buffer.clear();
        match function {
            FormattingFunction::Display => write!(tmp_value_buffer, "{value}")?,
            FormattingFunction::Debug => write!(tmp_value_buffer, "{value:?}")?,
        }

        let mut value_lines = tmp_value_buffer.trim_end().lines().peekable();
        while let Some(value_line) = value_lines.next() {
            let is_last = value_lines.peek().is_none();
            let line_formatting = match (is_first, is_last) {
                (true, true) => &formatting.standalone_line,
                (true, false) => &formatting.first_line,
                (false, false) => &formatting.middle_line,
                (false, true) => &formatting.last_line,
            };
            self.format_line(line_formatting, value_line)?;
            is_first = false;
        }
        Ok(())
    }

    fn format_line(
        &mut self,
        line_info: &LineFormatting,
        line: impl core::fmt::Display,
    ) -> fmt::Result {
        self.formatter.write_str(&self.line_prefix)?;
        self.formatter.write_str(line_info.prefix)?;
        line.fmt(self.formatter)?;
        self.formatter.write_str(line_info.suffix)?;
        Ok(())
    }

    fn format_reports(
        &mut self,
        reports: &[ReportRef<'a, dyn Any, Uncloneable, Local>],
    ) -> fmt::Result {
        let mut tmp_value_buffer = TmpValueBuffer::default();
        let mut tmp_attachments_buffer = TmpAttachmentsBuffer::default();
        let mut is_first = true;
        self.line_prefix = self.config.report_line_prefix_always.to_string();
        for &report in reports.iter() {
            if is_first {
                is_first = false;
            } else {
                self.formatter
                    .write_str(self.config.report_report_separator)?;
            }
            self.format_report_node(
                &mut tmp_value_buffer,
                &mut tmp_attachments_buffer,
                report,
                true,
                true,
            )?;
        }
        self.line_prefix = self.config.appendix_line_prefix_always.to_string();
        self.format_appendices(&mut tmp_value_buffer)?;
        Ok(())
    }

    fn format_report_node(
        &mut self,
        tmp_value_buffer: &mut TmpValueBuffer,
        tmp_attachments_buffer: &mut TmpAttachmentsBuffer<'a>,
        report: ReportRef<'a, dyn Any, Uncloneable, Local>,
        is_first_child: bool,
        is_last_child: bool,
    ) -> fmt::Result {
        let formatting = if is_first_child && is_last_child {
            &self.config.report_node_standalone_formatting
        } else if is_last_child {
            &self.config.report_node_last_formatting
        } else {
            &self.config.report_node_middle_formatting
        };
        self.format_node(
            tmp_value_buffer,
            formatting,
            report.format_current_context(),
            report
                .preferred_context_formatting_style(self.report_formatting_function)
                .function,
            |this, tmp_value_buffer| {
                this.format_node_data(tmp_value_buffer, tmp_attachments_buffer, report)
            },
        )?;
        Ok(())
    }

    fn format_node_data(
        &mut self,
        tmp_value_buffer: &mut TmpValueBuffer,
        tmp_attachments_buffer: &mut TmpAttachmentsBuffer<'a>,
        report: ReportRef<'a, dyn Any, Uncloneable, Local>,
    ) -> fmt::Result {
        let has_children = !report.children().is_empty();
        let has_attachments = !report.attachments().is_empty();

        let mut opaque_attachment_count = 0;
        tmp_attachments_buffer.clear();
        tmp_attachments_buffer.extend(
            report
                .attachments()
                .iter()
                .map(|attachment| {
                    (
                        attachment.preferred_formatting_style(self.report_formatting_function),
                        attachment,
                    )
                })
                .filter(
                    |(formatting_style, _attachment)| match formatting_style.placement {
                        AttachmentFormattingPlacement::Opaque => {
                            opaque_attachment_count += 1;
                            false
                        }
                        AttachmentFormattingPlacement::Hidden => false,
                        _ => true,
                    },
                ),
        );
        tmp_attachments_buffer
            .sort_by_key(|(style1, _attachment)| core::cmp::Reverse(style1.priority));
        for (attachment_index, &(attachment_formatting_style, attachment)) in
            tmp_attachments_buffer.iter().enumerate()
        {
            let is_last_attachment = attachment_index + 1 == tmp_attachments_buffer.len();
            self.format_attachment(
                tmp_value_buffer,
                attachment_formatting_style,
                attachment,
                is_last_attachment && !has_children,
            )?;
        }

        if opaque_attachment_count != 0 {
            let item_info = if has_children {
                &self.config.notice_opaque_middle_formatting
            } else {
                &self.config.notice_opaque_last_formatting
            };
            self.format_line(
                item_info,
                format_args!(
                    "{opaque_attachment_count} additional opaque {word}",
                    word = if opaque_attachment_count == 1 {
                        "attachment"
                    } else {
                        "attachments"
                    },
                ),
            )?;
        }

        if has_attachments
            && has_children
            && let Some(attachment_child_separator) = self.config.attachment_child_separator
        {
            self.format_with_line_prefix(attachment_child_separator)?;
        }

        for (report_index, child) in report.children().iter().enumerate() {
            if report_index != 0
                && let Some(child_child_separator) = self.config.child_child_separator
            {
                self.format_with_line_prefix(child_child_separator)?;
            }
            let is_first_child = report_index == 0;
            let is_last_child = report_index + 1 == report.children().len();
            self.format_report_node(
                tmp_value_buffer,
                tmp_attachments_buffer,
                child.into_uncloneable(),
                is_first_child,
                is_last_child,
            )?;
        }

        Ok(())
    }

    fn format_attachment(
        &mut self,
        tmp_value_buffer: &mut TmpValueBuffer,
        attachment_formatting_style: AttachmentFormattingStyle,
        attachment: ReportAttachmentRef<'a, dyn Any>,
        is_last: bool,
    ) -> fmt::Result {
        match attachment_formatting_style.placement {
            AttachmentFormattingPlacement::Inline => {
                let formatting = if is_last {
                    &self.config.attachment_inline_formatting_last
                } else {
                    &self.config.attachment_inline_formatting_middle
                };
                self.format_item(
                    tmp_value_buffer,
                    formatting,
                    attachment.format_inner(),
                    attachment_formatting_style.function,
                )?;
            }
            AttachmentFormattingPlacement::InlineWithHeader { header } => {
                let formatting = if is_last {
                    &self.config.attachment_headered_formatting_last
                } else {
                    &self.config.attachment_headered_formatting_middle
                };

                self.format_node(
                    tmp_value_buffer,
                    formatting,
                    header,
                    FormattingFunction::Display,
                    |this, tmp_value_buffer| {
                        if let Some(attachment_headered_data_prefix) =
                            this.config.attachment_headered_data_prefix
                        {
                            this.format_with_line_prefix(attachment_headered_data_prefix)?;
                        }

                        this.format_item(
                            tmp_value_buffer,
                            &self.config.attachment_headered_formatting_data,
                            attachment.format_inner(),
                            attachment_formatting_style.function,
                        )?;
                        if let Some(headered_attachment_data_suffix) =
                            this.config.attachment_headered_data_suffix
                        {
                            this.format_with_line_prefix(headered_attachment_data_suffix)?;
                        }

                        Ok(())
                    },
                )?;
            }
            AttachmentFormattingPlacement::Appendix { appendix_name } => {
                let appendices = self.appendices.entry(appendix_name).or_default();
                appendices.push((attachment, attachment_formatting_style.function));
                let formatting = if is_last {
                    &self.config.notice_see_also_last_formatting
                } else {
                    &self.config.notice_see_also_middle_formatting
                };
                let line = format_args!("{appendix_name} #{}", appendices.len());
                self.format_line(formatting, line)?;
            }
            AttachmentFormattingPlacement::Opaque | AttachmentFormattingPlacement::Hidden => {}
        }
        Ok(())
    }

    fn format_appendices(&mut self, tmp_value_buffer: &mut TmpValueBuffer) -> fmt::Result {
        let appendices = core::mem::take(&mut self.appendices);

        if appendices.is_empty() {
            self.formatter.write_str(self.config.no_appendices_footer)?;
            return Ok(());
        }

        self.formatter
            .write_str(self.config.report_appendix_separator)?;

        let mut is_first = true;
        for (appendix_name, appendices) in &appendices {
            for (appendix_index, &(attachment, formatting_function)) in
                appendices.iter().enumerate()
            {
                if is_first {
                    is_first = false;
                } else {
                    self.formatter
                        .write_str(self.config.appendix_appendix_separator)?;
                }

                let line = format_args!("{appendix_name} #{}", appendix_index + 1);
                self.format_line(&self.config.appendix_header, line)?;
                self.format_item(
                    tmp_value_buffer,
                    &self.config.appendix_body,
                    attachment.format_inner(),
                    formatting_function,
                )?;
            }
        }
        self.formatter.write_str(self.config.appendices_footer)?;
        Ok(())
    }
}
