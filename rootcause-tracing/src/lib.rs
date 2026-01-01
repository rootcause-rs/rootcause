#![deny(
    missing_docs,
    unsafe_code,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::broken_intra_doc_links,
    missing_copy_implementations,
    unused_doc_comments
)]

//! Tracing span capture for rootcause error reports.
//!
//! This crate automatically captures tracing span context when errors occur,
//! helping you understand which operation was being performed.
//!
//! # How It Works
//!
//! You add [`RootcauseLayer`] to your tracing subscriber alongside your
//! existing layers (formatting, filtering, log forwarding, etc.). While your
//! other layers do their work, `RootcauseLayer` quietly captures span field
//! values in the background for use in error reports.
//!
//! # Quick Start
//!
//! ```
//! use rootcause::hooks::Hooks;
//! use rootcause_tracing::{RootcauseLayer, SpanCollector};
//! use tracing_subscriber::{Registry, layer::SubscriberExt};
//!
//! // 1. Set up tracing with RootcauseLayer (required)
//! let subscriber = Registry::default()
//!     .with(RootcauseLayer) // Captures span field values for error reports
//!     .with(tracing_subscriber::fmt::layer()); // Your normal console output
//! tracing::subscriber::set_global_default(subscriber).expect("failed to set subscriber");
//!
//! // 2. Install hook to capture spans for all errors (optional)
//! Hooks::new()
//!     .report_creation_hook(SpanCollector::new())
//!     .install()
//!     .expect("failed to install hooks");
//!
//! // 3. Use normally - spans are captured automatically
//! #[tracing::instrument(fields(user_id = 42))]
//! fn example() -> rootcause::Report {
//!     rootcause::report!("something went wrong")
//! }
//! println!("{}", example());
//! ```
//!
//! Output:
//! ```text
//!  ● something went wrong
//!  ├ src/main.rs:10
//!  ╰ Tracing spans
//!    │ example{user_id=42}
//!    ╰─
//! ```
//!
//! ## Manual Attachment
//!
//! To attach spans selectively instead of automatically:
//!
//! ```
//! use rootcause::{Report, report};
//! use rootcause_tracing::SpanExt;
//!
//! #[tracing::instrument]
//! fn operation() -> Result<(), Report> {
//!     Err(report!("operation failed"))
//! }
//!
//! let result = operation().attach_span();
//! ```
//!
//! **Note:** [`RootcauseLayer`] must be in your subscriber setup either way.
//!
//! # Environment Variables
//!
//! - `ROOTCAUSE_TRACING` - Comma-separated options:
//!   - `leafs` - Only capture tracing spans for leaf errors (errors without
//!     children)

use std::{fmt, sync::OnceLock};

use rootcause::{
    Report, ReportMut,
    handlers::{
        AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
        FormattingFunction,
    },
    hooks::report_creation::ReportCreationHook,
    markers::{self, Dynamic, ObjectMarkerFor},
    report_attachment::ReportAttachment,
};
use tracing::{
    Span,
    field::{Field, Visit},
};

/// Handler for formatting [`Span`] attachments.
#[derive(Copy, Clone)]
pub struct SpanHandler;

/// Captured field values for a span.
#[derive(Clone)]
struct CapturedFields(String);

impl fmt::Display for CapturedFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AttachmentHandler<Span> for SpanHandler {
    fn display(value: &Span, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use tracing_subscriber::registry::LookupSpan;

        let result = value.with_subscriber(|(span_id, dispatch)| -> fmt::Result {
            let Some(subscriber) = dispatch.downcast_ref::<tracing_subscriber::Registry>() else {
                write!(f, "No tracing registry subscriber found")?;
                return Ok(());
            };

            let Some(span_ref) = subscriber.span(&span_id) else {
                write!(f, "No span found for ID")?;
                return Ok(());
            };

            let mut first = true;

            for ancestor in span_ref.scope() {
                if !first {
                    writeln!(f)?;
                }
                first = false;
                write!(f, "{}", ancestor.name())?;

                let extensions = ancestor.extensions();
                if let Some(fields) = extensions.get::<CapturedFields>() {
                    if !fields.0.is_empty() {
                        write!(f, "{{{}}}", fields.0)?;
                    }
                } else if let Some(metadata) = value.metadata() {
                    let fields = metadata.fields();
                    if !fields.is_empty() {
                        write!(f, "{{")?;
                        let mut first_field = true;
                        for field in fields {
                            if !first_field {
                                write!(f, " ")?;
                            }
                            write!(f, "{}", field.name())?;
                            first_field = false;
                        }
                        write!(f, "}}")?;
                    }
                }
            }

            Ok(())
        });

        if let Some(result) = result {
            result
        } else {
            write!(f, "Span disabled")
        }
    }

    fn debug(value: &Span, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        span: &Span,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: if span.is_none() {
                AttachmentFormattingPlacement::Hidden
            } else {
                AttachmentFormattingPlacement::InlineWithHeader {
                    header: "Tracing spans:",
                }
            },
            priority: 9, // Slightly lower priority than backtraces (10)
            ..Default::default()
        }
    }
}

/// A tracing layer that captures span field values for error reports.
///
/// **Required for rootcause-tracing.** Add this to your subscriber alongside
/// your other layers (formatting, filtering, log forwarding, etc.). It runs in
/// the background, capturing span field values without affecting your other
/// layers.
///
/// # Examples
///
/// ```
/// use rootcause_tracing::RootcauseLayer;
/// use tracing_subscriber::{Registry, layer::SubscriberExt};
///
/// let subscriber = Registry::default()
///     .with(RootcauseLayer) // Captures span data for error reports
///     .with(tracing_subscriber::fmt::layer()); // Example: console output
///
/// tracing::subscriber::set_global_default(subscriber).expect("failed to set subscriber");
/// ```
#[derive(Copy, Clone, Debug, Default)]
pub struct RootcauseLayer;

impl<S> tracing_subscriber::Layer<S> for RootcauseLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();

        // Use a simple visitor to format field values
        struct FieldVisitor<'a> {
            output: &'a mut String,
            first: bool,
        }

        impl<'a> Visit for FieldVisitor<'a> {
            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                use std::fmt::Write;
                if !self.first {
                    let _ = write!(self.output, " ");
                }
                self.first = false;
                let _ = write!(self.output, "{}={:?}", field.name(), value);
            }
        }

        let mut buf = String::new();
        let mut visitor = FieldVisitor {
            output: &mut buf,
            first: true,
        };
        attrs.record(&mut visitor);
        extensions.insert(CapturedFields(buf));
    }
}

/// Attachment collector for capturing tracing spans.
///
/// When registered as a report creation hook, this collector automatically
/// captures the current tracing span and attaches it as a [`Span`] attachment.
///
/// # Examples
///
/// Basic usage with default settings:
///
/// ```
/// use rootcause::hooks::Hooks;
/// use rootcause_tracing::SpanCollector;
///
/// Hooks::new()
///     .report_creation_hook(SpanCollector::new())
///     .install()
///     .expect("failed to install hooks");
/// ```
///
/// Custom configuration:
///
/// ```
/// use rootcause::hooks::Hooks;
/// use rootcause_tracing::SpanCollector;
///
/// let collector = SpanCollector {
///     capture_span_for_reports_with_children: true,
/// };
///
/// Hooks::new()
///     .report_creation_hook(collector)
///     .install()
///     .expect("failed to install hooks");
/// ```
#[derive(Copy, Clone)]
pub struct SpanCollector {
    /// Whether to capture spans for all reports or only leaf reports (those
    /// without children).
    ///
    /// When `true`, all reports get span attachments. When `false`, only leaf
    /// reports do.
    pub capture_span_for_reports_with_children: bool,
}

#[derive(Debug)]
struct RootcauseTracingEnvOptions {
    span_leafs_only: bool,
}

impl RootcauseTracingEnvOptions {
    fn get() -> &'static Self {
        static ROOTCAUSE_TRACING_FLAGS: OnceLock<RootcauseTracingEnvOptions> = OnceLock::new();

        ROOTCAUSE_TRACING_FLAGS.get_or_init(|| {
            let mut span_leafs_only = false;

            if let Some(var) = std::env::var_os("ROOTCAUSE_TRACING") {
                for v in var.to_string_lossy().split(',') {
                    if v.eq_ignore_ascii_case("leafs") {
                        span_leafs_only = true;
                    }
                }
            }

            RootcauseTracingEnvOptions { span_leafs_only }
        })
    }
}

impl SpanCollector {
    /// Creates a new [`SpanCollector`] with default settings.
    ///
    /// Configuration is controlled by environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `ROOTCAUSE_TRACING` - Comma-separated options:
    ///   - `leafs` - Only capture tracing spans for leaf errors (errors without
    ///     children)
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::hooks::Hooks;
    /// use rootcause_tracing::SpanCollector;
    ///
    /// // Respects ROOTCAUSE_TRACING environment variable
    /// Hooks::new()
    ///     .report_creation_hook(SpanCollector::new())
    ///     .install()
    ///     .expect("failed to install hooks");
    /// ```
    pub fn new() -> Self {
        let env_options = RootcauseTracingEnvOptions::get();
        let capture_span_for_reports_with_children = !env_options.span_leafs_only;

        Self {
            capture_span_for_reports_with_children,
        }
    }
}

impl Default for SpanCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportCreationHook for SpanCollector {
    fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, markers::Local>) {
        let do_capture =
            self.capture_span_for_reports_with_children || report.children().is_empty();
        if do_capture {
            let span = Span::current();
            if !span.is_none() {
                let attachment = ReportAttachment::new_custom::<SpanHandler>(span);
                report.attachments_mut().push(attachment.into_dynamic());
            }
        }
    }

    fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, markers::SendSync>) {
        let do_capture =
            self.capture_span_for_reports_with_children || report.children().is_empty();
        if do_capture {
            let span = Span::current();
            if !span.is_none() {
                let attachment = ReportAttachment::new_custom::<SpanHandler>(span);
                report.attachments_mut().push(attachment.into_dynamic());
            }
        }
    }
}

/// Extension trait for attaching tracing spans to reports.
///
/// This trait provides methods to easily attach the current tracing span
/// to a report or to the error contained within a `Result`.
///
/// # Examples
///
/// Attach tracing span to a report:
///
/// ```
/// use rootcause::report;
/// use rootcause_tracing::SpanExt;
///
/// #[tracing::instrument]
/// fn example() {
///     let report = report!("An error occurred").attach_span();
/// }
/// ```
///
/// Attach tracing span to a `Result`:
///
/// ```
/// use rootcause::{Report, report};
/// use rootcause_tracing::SpanExt;
///
/// #[tracing::instrument]
/// fn might_fail() -> Result<(), Report> {
///     Err(report!("operation failed").into_dynamic())
/// }
///
/// let result = might_fail().attach_span();
/// ```
pub trait SpanExt: Sized {
    /// Attaches the current tracing span to the report.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::report;
    /// use rootcause_tracing::SpanExt;
    ///
    /// #[tracing::instrument]
    /// fn example() {
    ///     let report = report!("error").attach_span();
    /// }
    /// ```
    fn attach_span(self) -> Self;
}

impl<C: ?Sized, T> SpanExt for Report<C, markers::Mutable, T>
where
    Span: ObjectMarkerFor<T>,
{
    fn attach_span(mut self) -> Self {
        let span = Span::current();
        if !span.is_disabled() {
            self = self.attach_custom::<SpanHandler, _>(span);
        }
        self
    }
}

impl<C: ?Sized, V, T> SpanExt for Result<V, Report<C, markers::Mutable, T>>
where
    Span: ObjectMarkerFor<T>,
{
    fn attach_span(self) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(report) => Err(report.attach_span()),
        }
    }
}
