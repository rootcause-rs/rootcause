//! Formatting overrides for customizing how specific types are formatted in
//! error reports.
//!
//! This module provides a hook system that allows you to customize how specific
//! types of data are displayed when they appear in error reports. This is
//! particularly useful when you want domain-specific formatting, enhanced
//! readability, or consistent presentation across your application.
//!
//! # Overview
//!
//! The formatting override system works by registering hooks for specific types
//! that will be called whenever those types need to be formatted in an error
//! report. There are two main categories of formatting overrides:
//!
//! ## Submodules
//!
//! - [`attachment`] - Customize formatting of error report attachments
//!   (additional data)
//!   - Custom display formatting for attachment types
//!   - Control attachment placement (inline, with headers, in appendix, hidden)
//!   - Set priorities for attachment ordering
//!   - Handle sensitive data by hiding attachments
//!
//! - [`context`] - Customize formatting of error report contexts (main error
//!   types)
//!   - Custom display and debug formatting for error types
//!   - Domain-specific error presentation
//!   - Enhanced debugging information
//!
//! See the individual submodule documentation for detailed examples and
//! comprehensive API reference.

pub mod attachment;
pub mod context;
