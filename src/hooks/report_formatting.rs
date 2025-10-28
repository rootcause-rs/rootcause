//! Report formatting hooks for customizing the overall appearance of reports.
//!
//! This module provides hooks that allow you to completely customize how entire
//! reports are formatted, including their structure, colors, and layout.
//!
//! # Report Formatter Hooks
//!
//! Report formatting hooks allow you to control the entire presentation of a
//! report. This includes how multiple reports in a chain are displayed, how
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
