//! Hooks system for customizing report creation and formatting behavior.
//!
//! # When to Use Hooks
//!
//! **Most users don't need hooks** - the defaults work well. Use hooks when you need to:
//! - Automatically attach data to ALL errors (request IDs, timestamps, environment info)
//! - Integrate with custom logging or observability systems
//! - Change how reports are formatted globally (different colors, layout, structure)
//! - Redact or transform sensitive data in error messages
//!
//! **If you just need to customize a single error**, use `.attach()` or handlers
//! (see [`examples/custom_handler.rs`]) instead of hooks.
//!
//! [`examples/custom_handler.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/custom_handler.rs
//!
//! # Modules
//!
//! - **[`report_creation`]**: Automatically add data to every report as it's created
//!   (e.g., request IDs, correlation IDs, environment variables)
//!
//! - **[`formatting_overrides`]**: Control how specific types appear in error messages
//!   (e.g., redact passwords, format timestamps, control attachment placement)
//!
//! - **[`report_formatting`]**: Change the entire report layout and structure
//!   (e.g., JSON output for logging, compact format, custom colors)
//!
//! - **[`builtin_hooks`]**: Default hooks that are automatically registered
//!   (location collectors, backtrace collectors, and the default formatter)
//!
//! See [`examples/report_creation_hook.rs`] and [`examples/formatting_hooks.rs`]
//! for complete examples.
//!
//! [`examples/report_creation_hook.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/report_creation_hook.rs
//! [`examples/formatting_hooks.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/formatting_hooks.rs

pub mod builtin_hooks;
pub mod formatting_overrides;
pub mod report_creation;
pub mod report_formatting;

mod hook_lock;
