use std::fmt;

use opentelemetry::{
    Context,
    trace::{TraceContextExt, TraceState},
};
use rootcause::{
    ReportMut,
    handlers::{
        AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
        FormattingFunction,
    },
    hooks::report_creation::ReportCreationHook,
    markers::{Dynamic, Local, ObjectMarkerFor, SendSync},
};

use crate::types::{TraceContextAttachment, TraceParent, TraceSettings};

impl AttachmentHandler<TraceParent> for TraceContextAttachment {
    fn display(value: &TraceParent, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Display::fmt(value, formatter)
    }

    fn debug(value: &TraceParent, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        value: &TraceParent,
        function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            function,
            placement: if value.is_valid() {
                AttachmentFormattingPlacement::InlineWithHeader {
                    header: "Traceparent:",
                }
            } else {
                AttachmentFormattingPlacement::Hidden
            },
            priority: -5,
        }
    }
}

impl AttachmentHandler<TraceState> for TraceContextAttachment {
    fn display(value: &TraceState, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&value.header())
    }

    fn debug(value: &TraceState, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        value: &TraceState,
        function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let _ = value;
        AttachmentFormattingStyle {
            function,
            placement: if value == &TraceState::NONE {
                AttachmentFormattingPlacement::Hidden
            } else {
                AttachmentFormattingPlacement::InlineWithHeader {
                    header: "Tracestate:",
                }
            },
            priority: -5,
        }
    }
}

impl AttachmentHandler<TraceSettings> for TraceContextAttachment {
    fn display(value: &TraceSettings, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(value, formatter)
    }

    fn debug(value: &TraceSettings, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        value: &TraceSettings,
        function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        let _ = value;
        AttachmentFormattingStyle {
            function,
            placement: AttachmentFormattingPlacement::Inline,
            priority: -5,
        }
    }
}

/// Collector for the [`TraceParent`], [`TraceState`], and [`TraceSettings`]
/// objects available at time of report creation, if any.
///
/// This collector creates three separate attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceContextCollector;

impl ReportCreationHook for TraceContextCollector {
    fn on_local_creation(&self, report: ReportMut<'_, Dynamic, Local>) {
        on_creation(report);
    }

    fn on_sendsync_creation(&self, report: ReportMut<'_, Dynamic, SendSync>) {
        on_creation(report);
    }
}

fn on_creation<T>(report: ReportMut<'_, Dynamic, T>)
where
    TraceSettings: ObjectMarkerFor<T>,
    TraceState: ObjectMarkerFor<T>,
    TraceParent: ObjectMarkerFor<T>,
{
    if Context::current().has_active_span() {
        let context = Context::current();
        let span_ref = context.span();
        let span_context_ref = span_ref.span_context();
        if span_context_ref.is_valid() {
            let tracestate = span_context_ref.trace_state().clone();

            let traceparent = TraceParent::from(span_context_ref);
            let trace_settings = TraceSettings::from(span_context_ref);

            let _ = report
                .attach_custom::<TraceContextAttachment, _>(traceparent)
                .attach_custom::<TraceContextAttachment, _>(tracestate)
                .attach_custom::<TraceContextAttachment, _>(trace_settings);
        }
    }
}
