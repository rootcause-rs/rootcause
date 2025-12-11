//! Report formatting hooks for customizing the overall report structure.
//!
//! Unlike [`attachment_formatter`] and [`context_formatter`] which control
//! individual pieces, this module controls the **entire report layout**:
//! how errors are arranged, what sections appear, colors, borders, etc.
//!
//! Use this when you need:
//! - Different output formats (JSON, plain text, HTML)
//! - ASCII-only output for compatibility
//! - Custom color schemes or visual styling
//! - Integration with logging systems
//!
//! [`attachment_formatter`]: crate::hooks::attachment_formatter
//! [`context_formatter`]: crate::hooks::context_formatter
//!
//! # Default Formatter
//!
//! By default, rootcause uses [`DefaultReportFormatter::UNICODE`] for Unicode
//! box-drawing characters without ANSI colors.
//!
//! You can switch to other variants:
//!
//! ```rust
//! use rootcause::{
//!     hooks::{Hooks, builtin_hooks::report_formatter::DefaultReportFormatter},
//!     prelude::*,
//! };
//!
//! // Switch to ASCII-only output globally (affects all reports)
//! Hooks::new()
//!     .report_formatter(DefaultReportFormatter::ASCII)
//!     .install()
//!     .expect("failed to install hooks");
//!
//! let report = report!("database connection failed");
//! println!("{}", report);
//! ```
//!
//! # Per-Report Formatting
//!
//! You can also apply a formatter to a specific report without changing the
//! global default using [`Report::format_with`]:
//!
//! ```rust
//! use rootcause::{hooks::builtin_hooks::report_formatter::DefaultReportFormatter, prelude::*};
//!
//! let report = report!("parsing error");
//!
//! // This report uses the default formatter (Unicode without colors)
//! println!("{}", report);
//!
//! // This uses ASCII-only for this specific report
//! println!("{}", report.format_with(&DefaultReportFormatter::ASCII));
//! ```
//!
//! This is useful when you need different output formats in different contexts,
//! such as:
//! - ASCII-only for log files
//! - Full Unicode+ANSI for terminal output
//! - Custom formatting for specific error types
//!
//! # Custom Formatters
//!
//! For complete control over report formatting, you can implement the
//! [`ReportFormatter`] trait. See the trait documentation for details and
//! examples of implementing custom formatters.
//!
//! The [`DefaultReportFormatter`] source code also serves as a comprehensive
//! example of a full-featured formatter implementation.
//!
//! [`Display`]: core::fmt::Display
//! [`Debug`]: core::fmt::Debug
//! [`Report::format_with`]: crate::Report::format_with
//! [`DefaultReportFormatter::UNICODE`]: crate::hooks::builtin_hooks::report_formatter::DefaultReportFormatter::UNICODE

use core::fmt;

use rootcause_internals::handlers::FormattingFunction;

use crate::{
    ReportRef,
    hooks::{HookData, builtin_hooks::report_formatter::DefaultReportFormatter, use_hooks},
    markers::{Dynamic, Local, Uncloneable},
};

/// A hook for customizing how reports are formatted and displayed.
///
/// This trait allows you to completely control the presentation of reports,
/// including their structure, layout, colors, and how multiple reports in a
/// collection are displayed together. Only one report formatter hook can be
/// active at a time.
///
/// # Examples
///
/// ```rust
/// use std::fmt;
///
/// use rootcause::{
///     ReportRef,
///     hooks::{Hooks, report_formatter::ReportFormatter},
///     markers::{Dynamic, Local, Uncloneable},
///     prelude::*,
/// };
///
/// #[derive(Debug)]
/// struct SimpleFormatter;
///
/// fn format_indented(
///     report: ReportRef<'_, Dynamic, Uncloneable, Local>,
///     indentation: usize,
///     formatter: &mut fmt::Formatter<'_>,
/// ) -> fmt::Result {
///     for _ in 0..indentation {
///         write!(formatter, "  ")?;
///     }
///     writeln!(formatter, "{}:", report.format_current_context_unhooked())?;
///     // TODO: Also format the attachments
///     for subreport in report.children() {
///         format_indented(subreport.into_uncloneable(), indentation + 1, formatter)?;
///     }
///     Ok(())
/// }
///
/// impl ReportFormatter for SimpleFormatter {
///     fn format_reports(
///         &self,
///         reports: &[ReportRef<'_, Dynamic, Uncloneable, Local>],
///         formatter: &mut fmt::Formatter<'_>,
///         _function: rootcause::handlers::FormattingFunction,
///     ) -> std::fmt::Result {
///         for (i, report) in reports.iter().enumerate() {
///             if i > 0 {
///                 writeln!(formatter)?;
///             }
///             format_indented(*report, 0, formatter)?;
///         }
///         Ok(())
///     }
/// }
///
/// Hooks::new()
///     .report_formatter(SimpleFormatter)
///     .install()
///     .expect("failed to install hooks");
/// ```
pub trait ReportFormatter: 'static + Send + Sync + fmt::Debug {
    /// Format multiple reports in a collection.
    ///
    /// This is the primary method that controls how reports are displayed. This
    /// includes how multiple reports at the "same level" are presented
    /// together.
    fn format_reports(
        &self,
        reports: &[ReportRef<'_, Dynamic, Uncloneable, Local>],
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result;

    /// Format a single report.
    ///
    /// This method provides a default implementation that calls
    /// [`format_reports`](ReportFormatter::format_reports) with a
    /// single-element slice. You typically don't need to implement this
    /// unless you want different behavior for single reports vs. report
    /// collections.
    fn format_report(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result {
        self.format_reports(&[report], formatter, report_formatting_function)
    }
}

pub(crate) fn format_report(
    report: ReportRef<'_, Dynamic, Uncloneable, Local>,
    formatter: &mut fmt::Formatter<'_>,
    report_formatting_function: FormattingFunction,
) -> fmt::Result {
    use_hooks(|hook_data: Option<&HookData>| {
        if let Some(hook_data) = hook_data
            && let Some(hook) = &hook_data.report_formatter
        {
            hook.format_report(report, formatter, report_formatting_function)
        } else {
            DefaultReportFormatter::DEFAULT.format_report(
                report,
                formatter,
                report_formatting_function,
            )
        }
    })
}

pub(crate) fn format_reports(
    reports: &[ReportRef<'_, Dynamic, Uncloneable, Local>],
    formatter: &mut fmt::Formatter<'_>,
    report_formatting_function: FormattingFunction,
) -> fmt::Result {
    use_hooks(|hook_data: Option<&HookData>| {
        if let Some(hook_data) = hook_data
            && let Some(hook) = &hook_data.report_formatter
        {
            hook.format_reports(reports, formatter, report_formatting_function)
        } else {
            DefaultReportFormatter::DEFAULT.format_reports(
                reports,
                formatter,
                report_formatting_function,
            )
        }
    })
}
