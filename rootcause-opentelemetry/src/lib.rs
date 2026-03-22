//! # Open telemetry tracing contexts for Rootcause reports.
//!
//! This crate provides report creation hooks for adding the
//! [Tracing Context][Tracing Context] provided by Open Telemetry to
//! reports.
//!
//! Two collectors are provided, one which collects the whole
//! [`SpanContext`][SpanContext] as a single attachment, and
//! one which splits it up into three separate attachments,
//! for the purpose of providing a slightly prettier formatting.
//!
//! [Tracing Context]: https://www.w3.org/TR/trace-context/
//! [SpanContext]: opentelemetry::trace::SpanContext

mod separate;
mod single;
mod types;
pub use opentelemetry::trace::TraceState;
pub use separate::TraceContextCollector;
pub use single::SpanContextCollector;
pub use types::TraceContextAttachment;
pub use types::TraceParent;
pub use types::TraceSettings;
