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
