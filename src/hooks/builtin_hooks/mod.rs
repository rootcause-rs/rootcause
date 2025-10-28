//! Built-in hooks that are automatically registered by the rootcause system.
//!
//! This module contains all the default hooks that are automatically enabled
//! when using rootcause.
//!
//! ## Attachment Collectors
//!
//! These hooks automatically collect and attach debugging information on report
//! creation:
//!
//! - **[`location`]**: Captures the source code location
//!   ([`core::panic::Location`]) where each report was created. This helps
//!   identify exactly where in your code an error originated.
//!
//! - **[`backtrace`]** (requires `backtrace` feature): Captures a full stack
//!   backtrace when each report is created, showing the call chain that led to
//!   the error.
//!
//! ## Report Formatter
//!
//! By default we install the [`DefaultReportFormatter`] as the report
//! formatting hook.
//!
//! To customize report formatting, you can implement you can either install
//! the built-in formatter with different options, or create your own custom
//! formatter by implementing the [`ReportFormatterHook`] trait and then
//! registering it via [`register_report_formatter_hook`].
//!
//! [`DefaultReportFormatter`]: crate::hooks::builtin_hooks::report_formatter::DefaultReportFormatter
//! [`ReportFormatterHook`]: crate::hooks::report_formatting::ReportFormatterHook
//! [`register_report_formatter_hook`]: crate::hooks::report_formatting::register_report_formatter_hook

#[cfg(feature = "backtrace")]
pub mod backtrace;
pub mod location;
pub mod report_formatter;
