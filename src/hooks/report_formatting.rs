//! Report formatting hooks for customizing the overall appearance of reports.
//!
//! This module provides hooks that allow you to completely customize how entire
//! reports are formatted, including their structure, colors, and layout.
//!
//! # Report Formatter Hooks
//!
//! Report formatting hooks allow you to control the entire presentation of a
//! report. This includes how multiple reports in a collection are displayed, how
//! individual reports are formatted, and how attachments are integrated into
//! the output.
//!
//! ```rust
//! use rootcause::{
//!     hooks::report_formatting::{ReportFormatterHook, register_report_formatter_hook},
//!     prelude::*,
//! };
//!
//! struct CompactFormatter;
//!
//! impl ReportFormatterHook for CompactFormatter {
//!     fn format_reports(
//!         &self,
//!         reports: &[rootcause::ReportRef<
//!             '_,
//!             dyn std::any::Any,
//!             rootcause::markers::Uncloneable,
//!             rootcause::markers::Local,
//!         >],
//!         formatter: &mut std::fmt::Formatter<'_>,
//!         _function: rootcause::handlers::FormattingFunction,
//!     ) -> std::fmt::Result {
//!         for (i, report) in reports.iter().enumerate() {
//!             if i > 0 {
//!                 write!(formatter, " -> ")?;
//!             }
//!             write!(formatter, "{}", report.format_current_context_unhooked())?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! register_report_formatter_hook(CompactFormatter);
//! ```
//!
//! # Default Formatter
//!
//! By default, rootcause installs a [`DefaultReportFormatter`] that provides
//! a comprehensive, multi-line report format with proper indentation,
//! attachments, and context information.

use core::{any::Any, fmt};

use rootcause_internals::handlers::FormattingFunction;
use triomphe::Arc;
use unsize::CoerceUnsize;

use crate::{
    ReportRef,
    hooks::{builtin_hooks::report_formatter::DefaultReportFormatter, hook_lock::HookLock},
    markers::{Local, Uncloneable},
};

type Hook = Arc<dyn ReportFormatterHook>;

static HOOK: HookLock<Hook> = HookLock::new();

/// A hook for customizing how reports are formatted and displayed.
///
/// This trait allows you to completely control the presentation of reports,
/// including their structure, layout, colors, and how multiple reports in a
/// collection are displayed together. Only one report formatter hook can be active
/// at a time.
///
/// # Examples
///
/// ```rust
/// use std::fmt;
///
/// use rootcause::{
///     ReportRef,
///     hooks::report_formatting::{ReportFormatterHook, register_report_formatter_hook},
///     markers::{Local, Uncloneable},
///     prelude::*,
/// };
///
/// struct SimpleFormatter;
///
/// fn format_indented(
///     report: ReportRef<'_, dyn Any, Uncloneable, Local>,
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
/// impl ReportFormatterHook for SimpleFormatter {
///     fn format_reports(
///         &self,
///         reports: &[ReportRef<'_, dyn Any, Uncloneable, Local>],
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
/// register_report_formatter_hook(SimpleFormatter);
/// ```
pub trait ReportFormatterHook: 'static + Send + Sync {
    /// Format multiple reports in a collection.
    ///
    /// This is the primary method that controls how reports are displayed. This
    /// includes how multiple reports at the "same level" are presented
    /// together.
    fn format_reports(
        &self,
        reports: &[ReportRef<'_, dyn Any, Uncloneable, Local>],
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result;

    /// Format a single report.
    ///
    /// This method provides a default implementation that calls
    /// [`format_reports`](ReportFormatterHook::format_reports) with a
    /// single-element slice. You typically don't need to override this
    /// unless you want different behavior for single reports vs. report
    /// collections.
    fn format_report(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
        report_formatting_function: FormattingFunction,
    ) -> fmt::Result {
        self.format_reports(&[report], formatter, report_formatting_function)
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

/// Registers a global report formatter hook.
///
/// This function replaces any previously registered report formatter hook with
/// the provided one. Only one report formatter hook can be active at a time,
/// so registering a new hook will override the previous one.
///
/// The hook will be used to format all reports created after registration.
/// If no custom hook is registered, the default [`DefaultReportFormatter`] is
/// used.
///
/// # Examples
///
/// ## Registering a Custom Formatter
///
/// ```rust
/// use std::fmt;
///
/// use rootcause::{
///     ReportRef,
///     hooks::report_formatting::{ReportFormatterHook, register_report_formatter_hook},
///     markers::{Local, Uncloneable},
///     prelude::*,
/// };
///
/// struct JsonFormatter;
///
/// fn to_json(report: ReportRef<'_, dyn Any, Uncloneable, Local>) -> serde_json::Value {
///     let mut obj = serde_json::Map::new();
///     obj.insert(
///         "message".to_string(),
///         serde_json::Value::String(report.format_current_context_unhooked().to_string()),
///     );
///     // TODO: Also add the attachments
///     let mut causes = vec![];
///     for subreport in report.children() {
///         causes.push(to_json(subreport.into_uncloneable()));
///     }
///     obj.insert("causes".to_string(), serde_json::Value::Array(causes));
///     serde_json::Value::Object(obj)
/// }
///
/// impl ReportFormatterHook for JsonFormatter {
///     fn format_reports(
///         &self,
///         reports: &[ReportRef<'_, dyn Any, Uncloneable, Local>],
///         formatter: &mut fmt::Formatter<'_>,
///         _function: rootcause::handlers::FormattingFunction,
///     ) -> std::fmt::Result {
///         let causes = reports.iter().map(|&report| to_json(report)).collect();
///         let json = serde_json::Value::Array(causes);
///         write!(formatter, "{}", json)
///     }
/// }
///
/// register_report_formatter_hook(JsonFormatter);
/// ```
///
/// ## Replacing an Existing Formatter
///
/// ```rust
/// use rootcause::{
///     hooks::report_formatting::{ReportFormatterHook, register_report_formatter_hook},
///     prelude::*,
/// };
///
/// struct FirstFormatter;
/// impl ReportFormatterHook for FirstFormatter {
///     fn format_reports(
///         &self,
///         reports: &[rootcause::ReportRef<
///             '_,
///             dyn std::any::Any,
///             rootcause::markers::Uncloneable,
///             rootcause::markers::Local,
///         >],
///         formatter: &mut std::fmt::Formatter<'_>,
///         _function: rootcause::handlers::FormattingFunction,
///     ) -> std::fmt::Result {
///         write!(formatter, "First formatter")
///     }
/// }
///
/// struct SecondFormatter;
/// impl ReportFormatterHook for SecondFormatter {
///     fn format_reports(
///         &self,
///         reports: &[rootcause::ReportRef<
///             '_,
///             dyn std::any::Any,
///             rootcause::markers::Uncloneable,
///             rootcause::markers::Local,
///         >],
///         formatter: &mut std::fmt::Formatter<'_>,
///         _function: rootcause::handlers::FormattingFunction,
///     ) -> std::fmt::Result {
///         write!(formatter, "Second formatter")
///     }
/// }
///
/// // Register first formatter
/// register_report_formatter_hook(FirstFormatter);
///
/// // This replaces the first formatter - only SecondFormatter will be used
/// register_report_formatter_hook(SecondFormatter);
/// ```
///
/// [`DefaultReportFormatter`]: crate::hooks::builtin_hooks::report_formatter::DefaultReportFormatter
pub fn register_report_formatter_hook(hook: impl ReportFormatterHook) {
    *HOOK.write().get() =
        Some(Arc::new(hook).unsize(unsize::Coercion!(to dyn ReportFormatterHook)));
}
