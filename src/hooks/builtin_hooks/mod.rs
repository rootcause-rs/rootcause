//! Built-in hooks provided by rootcause.
//!
//! # What's Automatic
//!
//! Rootcause automatically provides:
//! - **Location tracking** - Captures file/line where errors occur
//! - **Default report formatter** ([`DefaultReportFormatter`]) - Unicode output
//!   without colors
//!
//! Both are active even without installing any hooks. Use
//! [`Hooks::new_without_locations()`] if you don't want location tracking.
//!
//! [`Hooks::new_without_locations()`]: crate::hooks::Hooks::new_without_locations
//!
//! ## Attachment Collectors
//!
//! - **[`location`]**: Captures the source code location ([`Location`]) where
//!   each report was created. Always enabled unless you use
//!   [`Hooks::new_without_locations()`].
//!
//! [`Location`]: crate::hooks::builtin_hooks::location::Location
//!
//! ## Report Formatter
//!
//! - **[`report_formatter`]**: Controls the overall report layout and styling.
//!   [`DefaultReportFormatter`] is used by default.
//!
//! To customize, either use a different [`DefaultReportFormatter`] variant or
//! create your own by implementing the [`ReportFormatter`] trait, then install
//! it via [`Hooks::report_formatter`].
//!
//! [`DefaultReportFormatter`]: crate::hooks::builtin_hooks::report_formatter::DefaultReportFormatter
//! [`ReportFormatter`]: crate::hooks::report_formatter::ReportFormatter
//! [`Hooks::report_formatter`]: crate::hooks::Hooks::report_formatter

pub mod location;
pub mod report_formatter;
