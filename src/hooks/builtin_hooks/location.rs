//! Source code location attachment collector.
//!
//! This module provides functionality to automatically capture and attach
//! source code location information (file, line, column) to reports when they
//! are created.

use core::fmt;

use rootcause_internals::handlers::{AttachmentFormattingStyle, AttachmentHandler};

use crate::hooks::report_creation::AttachmentCollector;

/// Location in source code where a report was created.
///
/// This struct captures file and line information, typically used for
/// debugging and error tracking.
///
/// # Examples
///
/// ```
/// use rootcause::hooks::builtin_hooks::location::Location;
///
/// let loc = Location::caller();
/// println!("Error occurred at {}:{}", loc.file, loc.line);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    /// The source file path where the report was created.
    pub file: &'static str,
    /// The line number where the report was created.
    pub line: u32,
}

impl Location {
    /// Capture the caller's source code location.
    ///
    /// This function uses Rust's built-in `core::panic::Location::caller()` to
    /// obtain the file and line number of the code that invoked it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::hooks::builtin_hooks::location::Location;
    ///
    /// let loc = Location::caller();
    /// assert!(loc.file.ends_with(".rs"));
    /// assert!(loc.line > 0);
    /// ```
    #[track_caller]
    pub const fn caller() -> Self {
        let location = core::panic::Location::caller();
        Location {
            file: location.file(),
            line: location.line(),
        }
    }
}

/// Implementation of [`fmt::Display`] for [`Location`]
///
/// Uses the formatting convetion of `filename:line`
impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// Handler for formatting [`Location`] attachments.
///
/// This handler formats location information as `filename:line` for both
/// [`Display`] and [`Debug`] formatting.
///
/// # Examples
///
/// ```
/// use rootcause::{
///     hooks::builtin_hooks::location::{Location, LocationHandler},
///     prelude::*,
/// };
///
/// let report = report!("error").attach_custom::<LocationHandler, _>(Location::caller());
/// ```
///
/// [`Display`]: core::fmt::Display
/// [`Debug`]: core::fmt::Debug
#[derive(Copy, Clone)]
pub struct LocationHandler;
impl AttachmentHandler<Location> for LocationHandler {
    fn display(value: &Location, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Display::fmt(value, formatter)
    }

    fn debug(value: &Location, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Display::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        _value: &Location,
        _report_formatting_function: rootcause_internals::handlers::FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: rootcause_internals::handlers::AttachmentFormattingPlacement::Inline,
            priority: 20,
            function: rootcause_internals::handlers::FormattingFunction::Display,
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
/// ```
/// use rootcause::hooks::{Hooks, builtin_hooks::location::LocationHook};
///
/// // Install hooks with location collector
/// Hooks::new()
///     .attachment_collector(LocationHook)
///     .install()
///     .ok();
/// ```
#[derive(Copy, Clone)]
pub struct LocationHook;

impl AttachmentCollector<Location> for LocationHook {
    type Handler = LocationHandler;

    fn collect(&self) -> Location {
        Location::caller()
    }
}
