//! Hooks system for customizing report creation and formatting behavior.
//!
//! This module provides a flexible hook system that allows you to customize how
//! reports are created and formatted. The hooks system is organized into
//! several specialized modules:
//!
//! # Modules
//!
//! - **[`report_creation`]**: Hooks that run when reports are created, allowing
//!   you to automatically attach debugging information or modify reports as
//!   they're being created.
//!
//! - **[`formatting_overrides`]**: Hooks that customize how specific types of
//!   attachments and contexts are formatted when they appear in reports.
//!
//! - **[`report_formatting`]**: Hooks that control the overall formatting and
//!   layout of entire reports, including structure, colors, and presentation.
//!
//! - **[`builtin_hooks`]**: Default hooks that are automatically registered,
//!   including location collectors, backtrace collectors, and the default
//!   report formatter.

pub mod builtin_hooks;
pub mod formatting_overrides;
pub mod report_creation;
pub mod report_formatting;

mod hook_lock;
