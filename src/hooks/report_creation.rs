//! Report creation hooks for automatic report modification.
//!
//! This module provides hooks that run automatically when errors are created,
//! allowing you to attach metadata or modify reports without changing the code
//! that creates the errors.
//!
//! # Hook Types (use in order of complexity)
//!
//! 1. **Closures** - Simplest: Just return a value to attach
//!    ```rust
//!    # use rootcause::hooks::Hooks;
//!    Hooks::new().attachment_collector(|| "some data")
//!    # ;
//!    ```
//!
//! 2. **[`AttachmentCollector`]** - Simple: Collect and attach specific data
//!    automatically to every error. Use when you always want to attach the same
//!    type of information.
//!
//! 3. **[`ReportCreationHook`]** - Advanced: Full access to the report for
//!    conditional logic. Use when you need to inspect the error type or context
//!    before deciding what to attach.
//!
//! # Examples
//!
//! ## Simple: Using a Closure
//!
//! The easiest way to attach data to all errors:
//!
//! ```rust
//! use rootcause::hooks::Hooks;
//!
//! // Attach a request ID to every error
//! Hooks::new()
//!     .attachment_collector(|| format!("Request ID: {}", get_request_id()))
//!     .install()
//!     .expect("failed to install hooks");
//!
//! fn get_request_id() -> u64 { 42 }
//! ```
//!
//! ## Medium: Custom Attachment Collector
//!
//! When you need more control over formatting, implement [`AttachmentCollector`]:
//!
//! ```rust
//! use rootcause::{
//!     hooks::{Hooks, report_creation::AttachmentCollector},
//!     prelude::*,
//! };
//!
//! // Custom collector that adds environment information
//! struct EnvironmentCollector;
//!
//! impl AttachmentCollector<String> for EnvironmentCollector {
//!     type Handler = handlers::Display;
//!
//!     fn collect(&self) -> String {
//!         format!("Environment: {}", std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()))
//!     }
//! }
//!
//! Hooks::new()
//!     .attachment_collector(EnvironmentCollector)
//!     .install()
//!     .expect("failed to install hooks");
//! ```
//!
//! ## Advanced: Custom Report Creation Hook
//!
//! When you need conditional logic based on the error type, implement
//! [`ReportCreationHook`]:
//!
//! ```rust
//! use rootcause::{
//!     ReportMut,
//!     hooks::{Hooks, report_creation::ReportCreationHook},
//!     markers::{Dynamic, Local, SendSync},
//!     prelude::*,
//! };
//!
//! // Hook that adds retry hints for I/O errors
//! struct RetryHintHook;
//!
//! impl ReportCreationHook for RetryHintHook {
//!     fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, Local>) {
//!         // Check if this is an I/O error and add appropriate hint
//!         if let Some(io_err) = report.downcast_current_context::<std::io::Error>() {
//!             let hint = match io_err.kind() {
//!                 std::io::ErrorKind::TimedOut | std::io::ErrorKind::ConnectionRefused
//!                     => "Retry may succeed",
//!                 _ => "Retry unlikely to help",
//!             };
//!             report.attachments_mut().push(report_attachment!(hint).into());
//!         }
//!     }
//!
//!     fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, SendSync>) {
//!         // Same logic for Send+Sync errors
//!         if let Some(io_err) = report.downcast_current_context::<std::io::Error>() {
//!             let hint = match io_err.kind() {
//!                 std::io::ErrorKind::TimedOut | std::io::ErrorKind::ConnectionRefused
//!                     => "Retry may succeed",
//!                 _ => "Retry unlikely to help",
//!             };
//!             report.attachments_mut().push(report_attachment!(hint).into());
//!         }
//!     }
//! }
//!
//! Hooks::new()
//!     .report_creation_hook(RetryHintHook)
//!     .install()
//!     .expect("failed to install hooks");
//! ```

use alloc::boxed::Box;
use core::fmt;

use rootcause_internals::handlers::AttachmentHandler;

use crate::{
    ReportMut, handlers,
    hooks::{
        HookData,
        builtin_hooks::location::{Location, LocationHandler},
    },
    markers::{Dynamic, Local, SendSync},
    report_attachment::ReportAttachment,
};

pub(crate) trait StoredCreationHook: 'static + Send + Sync + core::fmt::Debug {
    #[track_caller]
    fn on_local_creation(&self, report: ReportMut<'_, Dynamic, Local>);

    #[track_caller]
    fn on_sendsync_creation(&self, report: ReportMut<'_, Dynamic, SendSync>);
}

/// A hook that is called whenever a report is created.
///
/// Report creation hooks provide a way to automatically modify or enhance
/// reports as they are being created, without requiring changes to the code
/// that creates the reports. This is useful for adding consistent metadata,
/// logging, or performing other side effects.
///
/// If you only need to add attachments, then consider using an
/// [`AttachmentCollector`] instead, as it gives you an easier to use API
/// for this use case.
///
/// # Examples
///
/// ```rust
/// use rootcause::{
///     ReportMut,
///     hooks::{Hooks, report_creation::ReportCreationHook},
///     markers::{Dynamic, Local, SendSync},
///     prelude::*,
/// };
///
/// struct LoggingHook;
///
/// impl ReportCreationHook for LoggingHook {
///     fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, Local>) {
///         println!("Local report created: {}", report);
///         let attachment = report_attachment!("Logged by LoggingHook");
///         report.attachments_mut().push(attachment.into());
///     }
///
///     fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, SendSync>) {
///         println!("SendSync report created: {}", report);
///         let attachment = report_attachment!("Logged by LoggingHook");
///         report.attachments_mut().push(attachment.into());
///     }
/// }
///
/// // Install the hook globally
/// Hooks::new()
///     .report_creation_hook(LoggingHook)
///     .install()
///     .expect("failed to install hooks");
/// ```
pub trait ReportCreationHook: 'static + Send + Sync {
    /// Called when a [`Local`] report is created.
    #[track_caller]
    fn on_local_creation(&self, report: ReportMut<'_, Dynamic, Local>);

    /// Called when a [`SendSync`] report is created.
    #[track_caller]
    fn on_sendsync_creation(&self, report: ReportMut<'_, Dynamic, SendSync>);
}

pub(crate) fn creation_hook_to_untyped<H>(hook: H) -> Box<dyn StoredCreationHook>
where
    H: ReportCreationHook + Send + Sync + 'static,
{
    struct Hook<H> {
        hook: H,
    }

    impl<H> core::fmt::Debug for Hook<H> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "CreationHook<{}>", core::any::type_name::<H>(),)
        }
    }

    impl<H> StoredCreationHook for Hook<H>
    where
        H: ReportCreationHook,
    {
        fn on_local_creation(&self, report: ReportMut<'_, Dynamic, Local>) {
            self.hook.on_local_creation(report);
        }

        fn on_sendsync_creation(&self, report: ReportMut<'_, Dynamic, SendSync>) {
            self.hook.on_sendsync_creation(report);
        }
    }

    let hook: Hook<H> = Hook { hook };
    Box::new(hook)
}

pub(crate) fn attachment_hook_to_untyped<A, H, C>(collector: C) -> Box<dyn StoredCreationHook>
where
    A: 'static + Send + Sync,
    H: AttachmentHandler<A>,
    C: AttachmentCollector<A> + Send + Sync + 'static,
{
    struct Hook<A, Handler, Collector> {
        collector: Collector,
        _handled_type: core::marker::PhantomData<fn(A) -> A>,
        _handler: core::marker::PhantomData<fn(Handler) -> Handler>,
    }

    impl<A, Handler, Collector> core::fmt::Debug for Hook<A, Handler, Collector> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "AttachmentCollector<{}, {}, {}>",
                core::any::type_name::<A>(),
                core::any::type_name::<Handler>(),
                core::any::type_name::<Collector>(),
            )
        }
    }

    impl<A, Handler, Collector> StoredCreationHook for Hook<A, Handler, Collector>
    where
        A: 'static + Send + Sync,
        Handler: AttachmentHandler<A>,
        Collector: AttachmentCollector<A> + Send + Sync,
    {
        #[track_caller]
        fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, Local>) {
            let attachment = self.collector.collect();
            report
                .attachments_mut()
                .push(ReportAttachment::new_local_custom::<Handler>(attachment).into_dynamic());
        }

        #[track_caller]
        fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, SendSync>) {
            let attachment = self.collector.collect();
            report
                .attachments_mut()
                .push(ReportAttachment::new_sendsync_custom::<Handler>(attachment).into_dynamic());
        }
    }

    let hook = Hook {
        collector,
        _handled_type: core::marker::PhantomData,
        _handler: core::marker::PhantomData,
    };
    let hook: Box<Hook<A, H, C>> = Box::new(hook);

    hook
}

/// A hook that collects data to be automatically attached to reports when they
/// are created.
///
/// Attachment collector hooks provide a specialized way to automatically
/// collect and attach specific types of data to all reports. Unlike
/// [`ReportCreationHook`], which provides full access to the report, attachment
/// collectors are focused solely on gathering data to be attached.
///
/// # Automatic Implementation
///
/// This trait is automatically implemented for any closure that returns a value
/// implementing [`Display`] and [`Debug`], using [`handlers::Display`] as the
/// handler:
///
/// [`Display`]: core::fmt::Display
/// [`Debug`]: core::fmt::Debug
///
/// ```rust
/// use rootcause::hooks::Hooks;
///
/// // This closure automatically implements AttachmentCollector<String>
/// Hooks::new()
///     .attachment_collector(|| "timestamp".to_string())
///     .install()
///     .expect("failed to install hooks");
/// ```
///
/// # Examples
///
/// ## Custom Collector Implementation
///
/// ```rust
/// use rootcause::{
///     hooks::{Hooks, report_creation::AttachmentCollector},
///     prelude::*,
/// };
///
/// struct SystemInfoCollector;
///
/// impl AttachmentCollector<String> for SystemInfoCollector {
///     type Handler = handlers::Display;
///
///     fn collect(&self) -> String {
///         format!(
///             "OS: {}, Arch: {}",
///             std::env::consts::OS,
///             std::env::consts::ARCH
///         )
///     }
/// }
///
/// // Install the collector globally
/// Hooks::new()
///     .attachment_collector(SystemInfoCollector)
///     .install()
///     .expect("failed to install hooks");
/// ```
///
/// ## Using a Closure
///
/// ```rust
/// use rootcause::hooks::Hooks;
///
/// // Install a closure that collects the current working directory
/// Hooks::new()
///     .attachment_collector(|| {
///         std::env::current_dir()
///             .map(|p| p.display().to_string())
///             .unwrap_or_else(|_| "unknown".to_string())
///     })
///     .install()
///     .expect("failed to install hooks");
/// ```
pub trait AttachmentCollector<A>: 'static + Send + Sync {
    /// The handler type used to format the collected data.
    type Handler: AttachmentHandler<A>;

    /// Collects the data to be attached to a report.
    ///
    /// This method is called once for each report creation and should return
    /// the data that will be attached to the report. The data will be formatted
    /// using the associated [`Handler`](Self::Handler) type.
    #[track_caller]
    fn collect(&self) -> A;
}

impl<A, F> AttachmentCollector<A> for F
where
    A: 'static + core::fmt::Display + core::fmt::Debug,
    F: 'static + Send + Sync + Fn() -> A,
{
    type Handler = handlers::Display;

    #[track_caller]
    fn collect(&self) -> A {
        (self)()
    }
}

#[track_caller]
pub(crate) fn run_creation_hooks_local(mut report: ReportMut<'_, Dynamic, Local>) {
    if let Some(hook_data) = HookData::fetch() {
        for hook in &hook_data.report_creation {
            hook.on_local_creation(report.as_mut());
        }
    } else {
        report.attachments_mut().push(
            ReportAttachment::new_local_custom::<LocationHandler>(Location::caller())
                .into_dynamic(),
        );
    }
}

#[track_caller]
pub(crate) fn run_creation_hooks_sendsync(mut report: ReportMut<'_, Dynamic, SendSync>) {
    if let Some(hook_data) = HookData::fetch() {
        for hook in &hook_data.report_creation {
            hook.on_sendsync_creation(report.as_mut());
        }
    } else {
        report.attachments_mut().push(
            ReportAttachment::new_sendsync_custom::<LocationHandler>(Location::caller())
                .into_dynamic(),
        );
    }
}
