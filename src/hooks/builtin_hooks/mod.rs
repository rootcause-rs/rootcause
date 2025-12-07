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
//! - **[`location`]**: Captures the source code location ([`Location`]) where
//!   each report was created. This helps identify exactly where in your code an
//!   error originated.
//!
//! - **[`backtrace`]** (requires `backtrace` feature): Captures a full stack
//!   [`Backtrace`] when each report is created, showing the call chain that led
//!   to the error.
//!
//! [`Location`]: crate::hooks::builtin_hooks::location::Location
//! [`Backtrace`]: crate::hooks::builtin_hooks::backtrace::Backtrace
//!
//! ## Report Formatter
//!
//! By default we install the [`DefaultReportFormatter`] as the report
//! formatting hook.
//!
//! To customize report formatting, you can either install
//! the built-in formatter with different options, or create your own custom
//! formatter by implementing the [`ReportFormatter`] trait and then
//! installing it via [`Hooks::with_report_formatter`].
//!
//! [`DefaultReportFormatter`]: crate::hooks::builtin_hooks::report_formatter::DefaultReportFormatter
//! [`ReportFormatter`]: crate::hooks::report_formatting::ReportFormatter
//! [`Hooks::with_report_formatter`]: crate::hooks::Hooks::with_report_formatter

#[cfg(feature = "backtrace")]
#[cfg_attr(docsrs, doc(cfg(feature = "backtrace")))]
pub mod backtrace;
pub mod location;
pub mod report_formatter;
