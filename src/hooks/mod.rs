//! Hooks system for customizing report creation and formatting behavior.
//!
//! This module provides a flexible hook system that allows you to customize how
//! reports are created and formatted. There are several types of hooks
//! available:
//!
//! - **Attachment Hooks**: Customize how specific attachment types are
//!   formatted
//! - **Context Hooks**: Customize how specific context types are formatted
//! - **Report Creation Hooks**: Run custom logic when reports are created
//! - **Report Formatting Hooks**: Customize the overall formatting of entire
//!   reports
//!
//! ## Hook Types
//!
//! ### Attachment Hooks
//!
//! Attachment hooks allow you to customize how specific types of attachments
//! are displayed and debugged. This is useful when you want special formatting
//! for certain data types.
//!
//! ```rust
//! use rootcause::{
//!     handlers::AttachmentFormattingStyle,
//!     hooks::handler_overrides::{AttachmentHandlerOverride, register_attachment_hook},
//!     prelude::*,
//! };
//!
//! #[derive(Debug)]
//! struct CustomData(String);
//!
//! struct CustomDataHook;
//!
//! impl AttachmentHandlerOverride<CustomData> for CustomDataHook {
//!     fn display(
//!         &self,
//!         attachment: rootcause::report_attachment::ReportAttachmentRef<'_, CustomData>,
//!         _parent: Option<rootcause::hooks::handler_overrides::AttachmentParent<'_>>,
//!         formatter: &mut std::fmt::Formatter<'_>,
//!     ) -> std::fmt::Result {
//!         write!(formatter, "Custom: {}", attachment.inner().0)
//!     }
//!
//!     fn preferred_formatting_style(
//!         &self,
//!         _attachment: rootcause::report_attachment::ReportAttachmentRef<'_, dyn std::any::Any>,
//!         _report_formatting_function: rootcause::handlers::FormattingFunction,
//!     ) -> AttachmentFormattingStyle {
//!         AttachmentFormattingStyle {
//!             placement: rootcause::handlers::AttachmentFormattingPlacement::InlineWithHeader {
//!                 header: "Custom Data",
//!             },
//!             function: rootcause::handlers::FormattingFunction::Display,
//!             priority: 0,
//!         }
//!     }
//! }
//!
//! // Register the hook globally
//! register_attachment_hook::<CustomData, _>(CustomDataHook);
//! ```
//!
//! ### Context Hooks
//!
//! Context hooks allow you to customize how specific context types (the main
//! error types) are displayed when they appear in reports.
//!
//! ```rust
//! use rootcause::{
//!     hooks::handler_overrides::{ContextHandlerOverride, register_context_hook},
//!     prelude::*,
//! };
//!
//! #[derive(Debug)]
//! struct MyError {
//!     code: u32,
//!     message: String,
//! }
//!
//! struct MyErrorHook;
//!
//! impl ContextHandlerOverride<MyError> for MyErrorHook {
//!     fn display(
//!         &self,
//!         report: rootcause::ReportRef<
//!             '_,
//!             MyError,
//!             rootcause::markers::Uncloneable,
//!             rootcause::markers::Local,
//!         >,
//!         formatter: &mut std::fmt::Formatter<'_>,
//!     ) -> std::fmt::Result {
//!         let context = report.current_context();
//!         write!(formatter, "Error {}: {}", context.code, context.message)
//!     }
//! }
//!
//! register_context_hook::<MyError, _>(MyErrorHook);
//! ```
//!
//! ### Report Creation Hooks
//!
//! Report creation hooks run automatically whenever a new report is created.
//! This is useful for automatically collecting debug information like
//! backtraces or caller location.
//!
//! ```rust
//! use rootcause::{
//!     hooks::report_creation::{ReportCreationHook, register_report_creation_hook},
//!     prelude::*,
//! };
//!
//! struct DebugInfoCollector;
//!
//! impl ReportCreationHook for DebugInfoCollector {
//!     fn on_sendsync_creation(
//!         &self,
//!         mut report: rootcause::ReportMut<'_, dyn std::any::Any, rootcause::markers::SendSync>,
//!     ) {
//!         // Automatically attach debug information
//!         let attachment =
//!             rootcause::report_attachment::ReportAttachment::new("Debug info collected");
//!         report.attachments_mut().push(attachment.into_dyn_any());
//!     }
//!
//!     fn on_local_creation(
//!         &self,
//!         mut report: rootcause::ReportMut<'_, dyn std::any::Any, rootcause::markers::Local>,
//!     ) {
//!         let attachment =
//!             rootcause::report_attachment::ReportAttachment::new("Local debug info");
//!         report.attachments_mut().push(attachment.into_dyn_any());
//!     }
//! }
//!
//! register_report_creation_hook(DebugInfoCollector);
//! ```
//!
//! ### Report Formatting Hooks
//!
//! Report formatting hooks allow you to completely customize how entire reports
//! are formatted, including their structure, colors, and layout.
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
//! ## Built-in Attachment Collectors
//!
//! The library includes several built-in attachment collectors that
//! automatically gather useful debugging information:
//!
//! - **Location Collector**: Automatically captures the source location where
//!   reports are created
//! - **Backtrace Collector** (with `backtrace` feature): Automatically captures
//!   stack backtraces
//!
//! These are enabled by default but can be customized or disabled through the
//! hook system.
//!
//! ## Hook Registration
//!
//! All hooks are registered globally and apply to all reports created after
//! registration. Hook registration is thread-safe and can be done at any time,
//! though it's typically done during application initialization.

pub mod builtin_hooks;
pub mod handler_overrides;
pub mod report_creation;
pub mod report_formatting;

mod hook_lock;
