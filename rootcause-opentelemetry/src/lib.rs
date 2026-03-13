use core::fmt;
use std::fmt::write;

use opentelemetry::{Context, SpanId, TraceFlags, TraceId, trace::{SpanContext, TraceContextExt, TraceState}};
use rootcause::{
    ReportMut,
    handlers::{
        AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler, Display, FormattingFunction
    },
    hooks::report_creation::ReportCreationHook,
    markers::{Dynamic, Local, SendSync},
};

struct TraceContext;

impl AttachmentHandler<SpanContext> for TraceContext {
    fn display(value: &SpanContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {

        write!(
            formatter,
            "Traceparent: 00-{:x}-{:x}-{:02x}",
            value.trace_id(),
            value.span_id(),
            value.trace_flags(),
        )?;

        if value.trace_state() != &TraceState::NONE {
            write!(formatter,
                "\nTracestate: {}",
                value.trace_state().header()
            )?;
        }

        match (value.is_remote(), value.is_sampled()) {
            (true, true) => write!(formatter, "\n(remote & sampled)"),
            (true, false) => write!(formatter, "\n(remote)"),
            (false, true) => write!(formatter, "\n(sampled)"),
            (false, false) => Ok(())
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
            priority: 8, // just slightly below tracing spans (9)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceParent {
    trace_id: TraceId,
    span_id: SpanId,
    trace_flags: TraceFlags
}

impl TraceParent {
    pub fn is_valid(&self) -> bool {
        self.trace_id != TraceId::INVALID && self.span_id != SpanId::INVALID
    }

    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    pub fn span_id(&self) -> SpanId {
        self.span_id
    }

    pub fn trace_flags(&self) -> TraceFlags {
        self.trace_flags
    }
}

impl fmt::Display for TraceParent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "00-{:x}-{:x}-{:02x}",
            self.trace_id,
            self.span_id,
            self.trace_flags,
        )
    }
}

impl AttachmentHandler<TraceParent> for TraceContext {
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
                AttachmentFormattingPlacement::InlineWithHeader { header: "Traceparent:" }
            } else {
                AttachmentFormattingPlacement::Hidden
            },
            priority: 7
        }
    }
}

impl AttachmentHandler<TraceState> for TraceContext {
    fn display(value: &TraceState, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            &value.header()
        )
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
                AttachmentFormattingPlacement::InlineWithHeader { header: "Tracestate:" }
            },
            priority: 6
        }
    }
}

struct SpanContextCollector;

impl ReportCreationHook for SpanContextCollector {
    fn on_local_creation(&self, report: ReportMut<'_, Dynamic, Local>) {
        if Context::current().has_active_span() {
            let context =  Context::current().span().span_context()
            if context.is_valid() {
                let mut context = context.clone();
                let tracestate = context.trace_state().clone();
                let traceparent = TraceParent {
                    trace_id: todo!(),
                    span_id: todo!(),
                    trace_flags: todo!(),
                }
            }
        }
    }

    fn on_sendsync_creation(&self, report: ReportMut<'_, Dynamic, SendSync>) {
        todo!()
    }
}

struct TraceContextCollector;

impl ReportCreationHook for TraceContextCollector {
    fn on_local_creation(&self, report: ReportMut<'_, Dynamic, Local>) {
        if Context::current().has_active_span() {
            let context =  Context::current().span().span_context()
            if context.is_valid() {
                let mut context = context.clone();
                let tracestate = context.tra
            }
        }
    }

    fn on_sendsync_creation(&self, report: ReportMut<'_, Dynamic, SendSync>) {
        todo!()
    }
}