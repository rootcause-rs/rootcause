use std::fmt;

use opentelemetry::{SpanId, TraceFlags, TraceId, trace::SpanContext};

/// Attachment handler for trace context items.
///
/// (Yes, the name contains 'Context', it is confusing.)
pub struct TraceContextAttachment;

/// Struct containing the Open Telemetry trace settings,
/// i.e. whether the current context is a remote trace,
/// and whether it is a sampled trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceSettings {
    remote: bool,
    sampled: bool,
}

impl TraceSettings {
    pub fn new(remote: bool, sampled: bool) -> Self {
        Self { remote, sampled }
    }

    pub fn is_remote(&self) -> bool {
        self.remote
    }

    pub fn is_sampled(&self) -> bool {
        self.sampled
    }
}

impl From<&SpanContext> for TraceSettings {
    fn from(value: &SpanContext) -> Self {
        TraceSettings::new(value.is_remote(), value.is_sampled())
    }
}

impl fmt::Display for TraceSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let place = if self.is_remote() { "remote" } else { "local" };
        let sampling = if self.is_sampled() {
            "sampled"
        } else {
            "non-sampled"
        };

        write!(f, "{place} & {sampling} trace")
    }
}

/// The `traceparent` HTTP header.
///
/// In the W3C specification this takes the form of four hexadecimal
/// numbers separated by hyphens, in digit groups of 2, 32, 16, and 2.
///
/// It consists of the [`trace_id`](SpanContext::trace_id),
/// [`span_id`](SpanContext::span_id), and
/// [`trace_flags`](SpanContext::trace_flags) of the
/// [`SpanContext`].
///
/// The corresponding `tracestate` HTTP header is provided as-is
/// by Open Telemetry in [`TraceSettings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceParent {
    trace_id: TraceId,
    span_id: SpanId,
    trace_flags: TraceFlags,
}

impl TraceParent {
    pub fn new(trace_id: TraceId, span_id: SpanId, trace_flags: TraceFlags) -> Self {
        Self {
            trace_id,
            span_id,
            trace_flags,
        }
    }

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

impl From<&SpanContext> for TraceParent {
    fn from(value: &SpanContext) -> Self {
        TraceParent::new(value.trace_id(), value.span_id(), value.trace_flags())
    }
}

impl fmt::Display for TraceParent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "00-{:x}-{:x}-{:02x}",
            self.trace_id, self.span_id, self.trace_flags,
        )
    }
}
