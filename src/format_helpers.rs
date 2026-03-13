//! Utility functions for formatting data in generic fashion.
//!
//! These are used to implement the various formatting functions
//! such as [`format_current_context`](crate::Report::format_current_context) and
//! [`format_inner`](crate::report_attachment::ReportAttachment::format_inner).
//!
//! Provided here as part of the API

use core::fmt;

use rootcause_internals::handlers::FormattingFunction;

/// Simple implementation of [`Display`](fmt::Display) + [`Debug`](fmt::Debug) via
/// two provided callback functions.
#[derive(Clone, Copy)]
pub struct FormatWithFunctions<State: Copy> {
    /// The state that is formatted
    pub state: State,
    /// Callback function for the [`Display::fmt`](fmt::Display::fmt) implementation
    pub display: fn(State, &mut fmt::Formatter<'_>) -> fmt::Result,
    /// Callback function for the [`Debug::fmt`](fmt::Debug::fmt) implementation
    pub debug: fn(State, &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl<State: Copy> fmt::Display for FormatWithFunctions<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.display)(self.state, f)
    }
}

impl<State: Copy> fmt::Debug for FormatWithFunctions<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.debug)(self.state, f)
    }
}

/// Implementation of [`Display`](fmt::Display) + [`Debug`](fmt::Debug) via
/// a single callback function that chooses behavior based on a [`FormattingFunction`].
///
/// (Not actually used in the library, but included for completeness.)
#[derive(Clone, Copy)]
pub struct FormatWithFunction<State: Copy> {
    /// The state that is formatted
    pub state: State,
    /// The callback function that handles both [`Display::fmt`](fmt::Display::fmt) and [`Debug::fmt`](fmt::Debug::fmt)
    pub formatter: fn(State, &mut fmt::Formatter<'_>, FormattingFunction) -> fmt::Result,
}

impl<State: Copy> fmt::Display for FormatWithFunction<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.formatter)(self.state, f, FormattingFunction::Display)
    }
}

impl<State: Copy> fmt::Debug for FormatWithFunction<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.formatter)(self.state, f, FormattingFunction::Debug)
    }
}

/// Implementation of [`Display`](fmt::Display) + [`Debug`](fmt::Debug)
/// via two callback functions each taking an extra argument.
#[derive(Clone, Copy)]
pub struct Format2WithFunctions<State: Copy, Value: Copy> {
    /// The state that is formatted
    pub state: State,
    /// The additional value passed to the callback functions
    pub value: Value,
    /// Callback function for the [`Display::fmt`](fmt::Display::fmt) implementation
    pub display: fn(State, Value, &mut fmt::Formatter<'_>) -> fmt::Result,
    /// Callback function for the [`Debug::fmt`](fmt::Debug::fmt) implementation
    pub debug: fn(State, Value, &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl<S: Copy, V: Copy> fmt::Display for Format2WithFunctions<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.display)(self.state, self.value, f)
    }
}

impl<S: Copy, B: Copy> fmt::Debug for Format2WithFunctions<S, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.debug)(self.state, self.value, f)
    }
}

/// Implementation of [`Display`](fmt::Display) + [`Debug`](fmt::Debug)
/// via a single callback function that chooses behavior based on a [`FormattingFunction`]
/// and takes an extra argument.
#[derive(Clone, Copy)]
pub struct Format2WithFunction<State: Copy, Value: Copy> {
    /// The state that is formatted
    pub state: State,
    /// The additional value passed to the callback function
    pub value: Value,
    /// The callback function that handles both [`Display::fmt`](fmt::Display::fmt) and [`Debug::fmt`](fmt::Debug::fmt)
    pub formatter: fn(State, Value, &mut fmt::Formatter<'_>, FormattingFunction) -> fmt::Result,
}

impl<S: Copy, V: Copy> fmt::Display for Format2WithFunction<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.formatter)(self.state, self.value, f, FormattingFunction::Display)
    }
}

impl<S: Copy, V: Copy> fmt::Debug for Format2WithFunction<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.formatter)(self.state, self.value, f, FormattingFunction::Debug)
    }
}
