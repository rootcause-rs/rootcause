//! Source code location attachment collector.
//!
//! This module provides functionality to automatically capture and attach
//! source code location information (file, line, column) to reports when they
//! are created.

use rootcause_internals::handlers::{AttachmentFormattingStyle, AttachmentHandler};

use crate::hooks::report_creation::AttachmentCollectorHook;

/// Source code location information.
///
/// Represents the file, line, and column where a report was created.
/// This information is automatically captured using
/// [`core::panic::Location::caller()`].
#[derive(Copy, Clone, Debug)]
pub struct Location {
    /// The source file path where the report was created.
    pub file: &'static str,
    /// The line number where the report was created.
    pub line: u32,
}

/// Handler for formatting [`Location`] attachments.
///
/// This handler formats location information as `filename:line` for both
/// [`Display`] and [`Debug`] formatting.
///
/// [`Display`]: core::fmt::Display
/// [`Debug`]: core::fmt::Debug
#[derive(Copy, Clone)]
pub struct LocationHandler;
impl AttachmentHandler<Location> for LocationHandler {
    fn display(value: &Location, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}:{}", value.file, value.line)
    }

    fn debug(value: &Location, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(value, formatter)
    }

    fn preferred_formatting_style(
        _value: &Location,
        _report_formatting_function: rootcause_internals::handlers::FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            priority: 20,
            ..Default::default()
        }
    }
}

/// Attachment collector for capturing source code location information.
///
/// When registered as a report creation hook, this collector automatically
/// captures the source location where each report is created and attaches
/// it as a [`Location`] attachment.
///
/// ## Example
///
/// ```rust
/// use rootcause::hooks::{
///     builtin_hooks::location::LocationCollector,
///     report_creation::register_attachment_collector_hook,
/// };
///
/// // Register to automatically collect location for all reports
/// register_attachment_collector_hook(LocationCollector);
/// ```
#[derive(Copy, Clone)]
pub struct LocationCollector;

impl AttachmentCollectorHook<Location> for LocationCollector {
    type Handler = LocationHandler;

    fn collect(&self) -> Location {
        let location = core::panic::Location::caller();
        Location {
            file: location.file(),
            line: location.line(),
        }
    }
}
