//! Formatting overrides for customizing how specific types are formatted.
//!
//! This module provides hooks that allow you to customize how specific types
//! of attachments and contexts are displayed when they appear in reports.
//! This is useful when you want special formatting for certain data types.
//!
//! # Hook Types
//!
//! ## Attachment Formatting Overrides
//!
//! Attachment formatting overrides allow you to customize how specific types of
//! attachments are displayed and debugged. This is useful when you want
//! special formatting for certain data types.
//!
//! ```rust
//! use rootcause::{
//!     handlers::AttachmentFormattingStyle,
//!     hooks::formatting_overrides::{AttachmentFormattingOverride, register_attachment_hook},
//!     prelude::*,
//! };
//!
//! #[derive(Debug)]
//! struct CustomData(String);
//!
//! struct CustomDataHook;
//!
//! impl AttachmentFormattingOverride<CustomData> for CustomDataHook {
//!     fn display(
//!         &self,
//!         attachment: rootcause::report_attachment::ReportAttachmentRef<'_, CustomData>,
//!         _parent: Option<rootcause::hooks::formatting_overrides::AttachmentParent<'_>>,
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
//! ## Context Formatting Overrides
//!
//! Context formatting overrides allow you to customize how specific context types
//! (the main error types) are displayed when they appear in reports.
//!
//! ```rust
//! use rootcause::{
//!     hooks::formatting_overrides::{ContextFormattingOverride, register_context_hook},
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
//! impl ContextFormattingOverride<MyError> for MyErrorHook {
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

mod attachment;
mod context;

pub use attachment::*;
pub use context::*;
