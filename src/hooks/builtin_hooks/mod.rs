//! Built-in hooks that are automatically registered by the rootcause system.
//!
//! This module contains all the default hooks that are automatically enabled
//! when using rootcause. These hooks provide essential debugging functionality
//! by automatically collecting and attaching useful information to every report
//! that gets created.
//!
//! # Automatic Registration
//!
//! All hooks in this module are **automatically registered** when the first
//! report is created in your application. You don't need to manually register
//! them - they're enabled by default to provide a good out-of-the-box
//! debugging experience.
//!
//! # Default Hooks
//!
//! The following hooks are automatically registered as report creation hooks:
//!
//! ## Attachment Collectors
//!
//! These hooks automatically collect and attach debugging information:
//!
//! - **[`location`]**: Captures the source code location ([`core::panic::Location`])
//!   where each report was created. This helps identify exactly where in your
//!   code an error originated.
//!
//! - **[`backtrace`]** (requires `backtrace` feature): Captures a full stack
//!   backtrace when each report is created, showing the call chain that led
//!   to the error.
//!
//! ## Future Hooks
//!
//! This module is designed to house additional default hooks as they are added
//! to the system, such as:
//!
//! - Environment variable collectors
//! - System information collectors
//! - Process information collectors
//! - Custom report creation hooks for common debugging scenarios
//!
//! # Manual Usage
//!
//! While these hooks are registered automatically, you can also register them
//! manually if you need additional instances or want to use them in specific
//! contexts:
//!
//! ```rust
//! use rootcause::hooks::{
//!     builtin_hooks::location::LocationCollector,
//!     register_attachment_collector_hook,
//! };
//!
//! // Register an additional location collector instance
//! // (not usually necessary since one is registered automatically)
//! register_attachment_collector_hook(LocationCollector);
//! ```
//!
//! # Disabling Default Hooks
//!
//! Currently, there is no built-in mechanism to disable the default hooks,
//! as they provide essential debugging functionality with minimal overhead.
//! If you need to disable them for specific use cases, you would need to
//! use rootcause with a custom configuration (feature request welcome).
//!
//! # Implementation Details
//!
//! The hooks in this module are registered via the `default_hooks` function
//! in the report creation module, which is called automatically when
//! the hook system is first initialized. This ensures that all reports benefit
//! from basic debugging information without requiring any setup from users.

#[cfg(feature = "backtrace")]
pub mod backtrace;
pub mod location;
