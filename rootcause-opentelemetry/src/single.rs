use std::fmt;

use opentelemetry::{
    Context,
    trace::{SpanContext, TraceContextExt, TraceState},
};
use rootcause::{
    ReportMut,
    handlers::{
        AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
        FormattingFunction,
    },
    hooks::report_creation::ReportCreationHook,
    markers::{Dynamic, ObjectMarkerFor},
};

use crate::types::TraceContextAttachment;

impl AttachmentHandler<SpanContext> for TraceContextAttachment {
    fn display(value: &SpanContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Traceparent: 00-{:x}-{:x}-{:02x}",
            value.trace_id(),
            value.span_id(),
            value.trace_flags(),
        )?;

        if value.trace_state() != &TraceState::NONE {
            write!(formatter, "\nTracestate: {}", value.trace_state().header())?;
        }

        match (value.is_remote(), value.is_sampled()) {
            (true, true) => write!(formatter, "\n(remote & sampled)"),
            (true, false) => write!(formatter, "\n(remote)"),
            (false, true) => write!(formatter, "\n(sampled)"),
            (false, false) => Ok(()),
        }
    }

    fn debug(value: &SpanContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        value: &SpanContext,
        function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            function,
            placement: if value.is_valid() {
                AttachmentFormattingPlacement::InlineWithHeader {
                    header: "Trace context:",
                }
            } else {
                AttachmentFormattingPlacement::Hidden
            },
            priority: -5,
        }
    }
}

/// Collector for the [`SpanContext`] object available at time
/// of report creation, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanContextCollector;

impl ReportCreationHook for SpanContextCollector {
    fn on_local_creation(
        &self,
        report: rootcause::ReportMut<'_, rootcause::markers::Dynamic, rootcause::markers::Local>,
    ) {
        on_creation(report);
    }

    fn on_sendsync_creation(
        &self,
        report: rootcause::ReportMut<'_, rootcause::markers::Dynamic, rootcause::markers::SendSync>,
    ) {
        on_creation(report);
    }
}

fn on_creation<T>(report: ReportMut<'_, Dynamic, T>)
where
    SpanContext: ObjectMarkerFor<T>,
{
    if Context::current().has_active_span() {
        let context = Context::current();
        let span_ref = context.span();
        let span_context_ref = span_ref.span_context();
        if span_context_ref.is_valid() {
            let span_context = span_context_ref.clone();
            let _ = report.attach_custom::<TraceContextAttachment, _>(span_context);
        }
    }
}
