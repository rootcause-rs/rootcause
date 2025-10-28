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
use triomphe::Arc;
use unsize::CoerceUnsize;

use crate::{
    ReportRef,
    hooks::hook_lock::HookLock,
    markers::{Local, Uncloneable},
    report_attachment::ReportAttachmentRef,
};

type Hook = Arc<dyn ReportFormatterHook>;

static HOOK: HookLock<Hook> = HookLock::new();

pub trait ReportFormatterHook: 'static + Send + Sync {
    fn format_reports(
        &self,
        reports: &[ReportRef<'_, dyn Any, Uncloneable, Local>],
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result;

    fn format_report(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result {
        self.format_reports(&[report], formatter, report_formatting_function)
    }
}

pub struct DefaultReportFormatter {
    pub report_header: &'static str,
    pub report_line_prefix_always: &'static str,
    pub appendix_line_prefix_always: &'static str,
    pub context_standalone_formatting: NodeConfig,
    pub context_middle_formatting: NodeConfig,
    pub context_last_formatting: NodeConfig,
    pub attachment_middle_formatting: ItemFormatting,
    pub attachment_last_formatting: ItemFormatting,
    pub headered_attachment_middle_formatting: NodeConfig,
    pub headered_attachment_last_formatting: NodeConfig,
    pub headered_attachment_data_formatting: ItemFormatting,
    pub headered_attachment_data_prefix: Option<&'static str>,
    pub headered_attachment_data_suffix: Option<&'static str>,
    pub see_also_notice_middle_formatting: LineFormatting,
    pub see_also_notice_last_formatting: LineFormatting,
    pub opaque_notice_middle_formatting: LineFormatting,
    pub opaque_notice_last_formatting: LineFormatting,
    pub attachment_child_separator: Option<&'static str>,
    pub child_child_separator: Option<&'static str>,
    pub report_report_separator: &'static str,
    pub report_appendix_separator: &'static str,
    pub appendix_appendix_separator: &'static str,
    pub appendix_header: LineFormatting,
    pub appendix_body: ItemFormatting,
    pub appendices_footer: &'static str,
    pub no_appendices_footer: &'static str,
}

impl DefaultReportFormatter {
    pub const ASCII_NO_ANSI: Self = Self {
        report_header: "\n",
        report_line_prefix_always: "",
        appendix_line_prefix_always: "",
        context_standalone_formatting: NodeConfig::new(
            ("o  ", "\n"),
            ("o  ", "\n"),
            ("|  ", "\n"),
            ("|  ", "\n"),
            "",
        ),
        context_middle_formatting: NodeConfig::new(
            ("|--> o  ", "\n"),
            ("|--> o  ", "\n"),
            ("|    |  ", "\n"),
            ("|    |  ", "\n"),
            "|    ",
        ),
        context_last_formatting: NodeConfig::new(
            (r"|--> o  ", "\n"),
            (r"|--> o  ", "\n"),
            ("     |  ", "\n"),
            ("     |  ", "\n"),
            "     ",
        ),
        attachment_middle_formatting: ItemFormatting::new(
            ("|- ", "\n"),
            ("|- ", "\n"),
            ("|  ", "\n"),
            ("|  ", "\n"),
        ),
        attachment_last_formatting: ItemFormatting::new(
            (r"|- ", "\n"),
            (r"|- ", "\n"),
            ("   ", "\n"),
            ("   ", "\n"),
        ),
        headered_attachment_middle_formatting: NodeConfig::new(
            (r"|- ", "\n"),
            ("|- ", "\n"),
            ("|  ", "\n"),
            ("|  ", "\n"),
            "| ",
        ),
        headered_attachment_last_formatting: NodeConfig::new(
            (r"|- ", "\n"),
            (r"|- ", "\n"),
            ("  ", "\n"),
            ("  ", "\n"),
            "  ",
        ),
        headered_attachment_data_formatting: ItemFormatting::new(
            (r"|- ", "\n"),
            ("|- ", "\n"),
            ("|- ", "\n"),
            (r"|- ", "\n"),
        ),
        headered_attachment_data_prefix: None,
        headered_attachment_data_suffix: None,
        see_also_notice_middle_formatting: LineFormatting::new("|- See ", " below\n"),
        see_also_notice_last_formatting: LineFormatting::new(r"|- See ", " below\n"),
        opaque_notice_middle_formatting: LineFormatting::new("|- ", "\n"),
        opaque_notice_last_formatting: LineFormatting::new(r"|- ", "\n"),
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
    pub const DEFAULT: Self = Self::UNICODE_ANSI;
    pub const UNICODE_ANSI: Self = Self {
        report_header: "\n",
        report_line_prefix_always: " ",
        appendix_line_prefix_always: "",
        context_standalone_formatting: NodeConfig::new(
            ("\x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("\x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("│ \x1b[1;97m", "\x1b[0m\n"),
            ("│ \x1b[1;97m", "\x1b[0m\n"),
            "",
        ),
        context_middle_formatting: NodeConfig::new(
            ("├─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("├─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("│  │ \x1b[1;97m", "\x1b[0m\n"),
            ("│  │ \x1b[1;97m", "\x1b[0m\n"),
            "│  ",
        ),
        context_last_formatting: NodeConfig::new(
            ("╰─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("╰─ \x1b[97m● \x1b[1m", "\x1b[0m\n"),
            ("   │ \x1b[1;97m", "\x1b[0m\n"),
            ("   │ \x1b[1;97m", "\x1b[0m\n"),
            "   ",
        ),
        attachment_middle_formatting: ItemFormatting::new(
            ("├ ", "\n"),
            ("├ ", "\n"),
            ("│ ", "\n"),
            ("│ ", "\n"),
        ),
        attachment_last_formatting: ItemFormatting::new(
            ("╰ ", "\n"),
            ("╰ ", "\n"),
            ("  ", "\n"),
            ("  ", "\n"),
        ),
        headered_attachment_middle_formatting: NodeConfig::new(
            ("├ \x1b[4m", "\x1b[0m\n"),
            ("├ \x1b[4m", "\x1b[0m\n"),
            ("│\x1b[4m", "\x1b[0m\n"),
            ("│\x1b[4m", "\x1b[0m\n"),
            "│ ",
        ),
        headered_attachment_last_formatting: NodeConfig::new(
            ("╰ \x1b[4m", "\x1b[0m\n"),
            ("╰ \x1b[4m", "\x1b[0m\n"),
            (" \x1b[4m", "\x1b[0m\n"),
            (" \x1b[4m", "\x1b[0m\n"),
            "  ",
        ),
        headered_attachment_data_formatting: ItemFormatting::new(
            ("│ ", "\n"),
            ("│ ", "\n"),
            ("│ ", "\n"),
            ("│ ", "\n"),
        ),
        headered_attachment_data_prefix: None,
        headered_attachment_data_suffix: Some("╰─\n"),
        see_also_notice_middle_formatting: LineFormatting::new("├ See \x1b[4m", "\x1b[0m below\n"),
        see_also_notice_last_formatting: LineFormatting::new("╰ See \x1b[4m", "\x1b[0m below\n"),
        opaque_notice_middle_formatting: LineFormatting::new("├ ", "\n"),
        opaque_notice_last_formatting: LineFormatting::new("╰ ", "\n"),
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

#[derive(Copy, Clone)]
pub struct NodeConfig {
    pub header: ItemFormatting,
    pub prefix_children: &'static str,
}

#[derive(Copy, Clone)]
pub struct ItemFormatting {
    pub standalone_line: LineFormatting,
    pub first_line: LineFormatting,
    pub middle_line: LineFormatting,
    pub last_line: LineFormatting,
}

#[derive(Copy, Clone)]
pub struct LineFormatting {
    pub prefix: &'static str,
    pub suffix: &'static str,
}

impl LineFormatting {
    pub const fn new(prefix: &'static str, suffix: &'static str) -> Self {
        Self { prefix, suffix }
    }
}

impl ItemFormatting {
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

impl NodeConfig {
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

        let mut value_lines = tmp_value_buffer.lines().peekable();
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
            &self.config.context_standalone_formatting
        } else if is_last_child {
            &self.config.context_last_formatting
        } else {
            &self.config.context_middle_formatting
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
                &self.config.opaque_notice_middle_formatting
            } else {
                &self.config.opaque_notice_last_formatting
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
                    &self.config.attachment_last_formatting
                } else {
                    &self.config.attachment_middle_formatting
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
                    &self.config.headered_attachment_last_formatting
                } else {
                    &self.config.headered_attachment_middle_formatting
                };

                self.format_node(
                    tmp_value_buffer,
                    formatting,
                    header,
                    FormattingFunction::Display,
                    |this, tmp_value_buffer| {
                        if let Some(headered_attachment_data_prefix) =
                            this.config.headered_attachment_data_prefix
                        {
                            this.format_with_line_prefix(headered_attachment_data_prefix)?;
                        }

                        this.format_item(
                            tmp_value_buffer,
                            &self.config.headered_attachment_data_formatting,
                            attachment.format_inner(),
                            attachment_formatting_style.function,
                        )?;
                        if let Some(headered_attachment_data_suffix) =
                            this.config.headered_attachment_data_suffix
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
                    &self.config.see_also_notice_last_formatting
                } else {
                    &self.config.see_also_notice_middle_formatting
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

pub(crate) fn format_report(
    report: ReportRef<'_, dyn Any, Uncloneable, Local>,
    formatter: &mut fmt::Formatter<'_>,
    report_formatting_function: FormattingFunction,
) -> fmt::Result {
    let hook = HOOK.read().get().cloned();
    let hook = hook
        .as_deref()
        .unwrap_or(const { &DefaultReportFormatter::DEFAULT });
    hook.format_report(report, formatter, report_formatting_function)
}

pub(crate) fn format_reports(
    reports: &[ReportRef<'_, dyn Any, Uncloneable, Local>],
    formatter: &mut fmt::Formatter<'_>,
    report_formatting_function: FormattingFunction,
) -> fmt::Result {
    let hook = HOOK.read().get().cloned();
    let hook = hook
        .as_deref()
        .unwrap_or(const { &DefaultReportFormatter::DEFAULT });
    hook.format_reports(reports, formatter, report_formatting_function)
}

pub fn register_report_formatter_hook(hook: impl ReportFormatterHook) {
    *HOOK.write().get() =
        Some(Arc::new(hook).unsize(unsize::Coercion!(to dyn ReportFormatterHook)));
}
