//! Report creation hooks for automatic report modification.
//!
//! This module provides a system for registering hooks that are automatically
//! executed when reports are created. These hooks can add additional context
//! or attachments to reports automatically, without requiring manual
//! intervention from the code creating the report.
//!
//! # Hook Types
//!
//! There are two main types of hooks:
//!
//! 1. [`ReportCreationHook`]: General hooks that get access to the entire
//!    report during creation and can perform arbitrary operations.
//!
//! 2. [`AttachmentCollectorHook`]: Specialized hooks that collect specific
//!    types of data and automatically attach them to reports.
//!
//! Internally the attachment collector hooks are converted to report creation
//! hooks and registered using the same system. They exist to provide a simpler
//! API for the common use case of adding attachments.
//!
//! # Examples
//!
//! ## Registering a Custom Report Creation Hook
//!
//! ```rust
//! use rootcause::{
//!     ReportMut,
//!     hooks::report_creation::{ReportCreationHook, register_report_creation_hook},
//!     markers::{Local, SendSync},
//!     prelude::*,
//! };
//!
//! struct MyHook;
//!
//! impl ReportCreationHook for MyHook {
//!     fn on_local_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, Local>) {
//!         // Add custom logic for local reports
//!         let attachment = report_attachment!("Custom local context");
//!         report.attachments_mut().push(attachment.into());
//!     }
//!
//!     fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, SendSync>) {
//!         // Add custom logic for send+sync reports
//!         let attachment = report_attachment!("Custom sendsync context");
//!         report.attachments_mut().push(attachment.into());
//!     }
//! }
//!
//! // Register the hook
//! register_report_creation_hook(MyHook);
//! ```
//!
//! ## Registering an Attachment Collector Hook
//!
//! ```rust
//! use rootcause::{
//!     hooks::report_creation::{AttachmentCollectorHook, register_attachment_collector_hook},
//!     prelude::*,
//! };
//!
//! struct ProcessIdCollector;
//!
//! #[derive(Debug)]
//! struct ProcessId(u32);
//!
//! impl std::fmt::Display for ProcessId {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         write!(f, "Process ID: {}", self.0)
//!     }
//! }
//!
//! impl AttachmentCollectorHook<u32> for ProcessIdCollector {
//!     type Handler = handlers::Display;
//!
//!     fn collect(&self) -> u32 {
//!         std::process::id()
//!     }
//! }
//!
//! // Register the collector
//! register_attachment_collector_hook(ProcessIdCollector);
//! ```
//!
//! ## Using a Closure as an Attachment Collector
//!
//! ```rust
//! use rootcause::hooks::report_creation::register_attachment_collector_hook;
//!
//! // Register a simple closure that collects the current timestamp
//! register_attachment_collector_hook(|| {
//!     std::time::SystemTime::now()
//!         .duration_since(std::time::UNIX_EPOCH)
//!         .unwrap()
//!         .as_secs()
//! });
//! ```

use alloc::{boxed::Box, vec::Vec};
use core::{any::Any, fmt, panic::Location};

use rootcause_internals::handlers::AttachmentHandler;

#[cfg(feature = "backtrace")]
use crate::hooks::builtin_hooks::backtrace::BacktraceCollector;
use crate::{
    ReportMut, handlers,
    hooks::{
        builtin_hooks::location::{LocationCollector, LocationHandler},
        hook_lock::{HookLock, HookLockReadGuard},
    },
    markers::{Local, SendSync},
    report_attachment::ReportAttachment,
};

type HookSet = Vec<Box<dyn UntypedReportCreationHook>>;

static HOOKS: HookLock<HookSet> = HookLock::new();

trait UntypedReportCreationHook: 'static + Send + Sync + core::fmt::Display {
    #[track_caller]
    fn on_local_creation(&self, report: ReportMut<'_, dyn Any, Local>);
    #[track_caller]
    fn on_sendsync_creation(&self, report: ReportMut<'_, dyn Any, SendSync>);
}

/// A hook that is called whenever a report is created.
///
/// Report creation hooks provide a way to automatically modify or enhance
/// reports as they are being created, without requiring changes to the code
/// that creates the reports. This is useful for adding consistent metadata,
/// logging, or performing other side effects.
///
/// If you only need to add attachments, then consider using an
/// [`AttachmentCollectorHook`] instead, as it gives you an easier to use API
/// for this use case.
///
/// # Examples
///
/// ```rust
/// use rootcause::{
///     ReportMut,
///     hooks::report_creation::{ReportCreationHook, register_report_creation_hook},
///     markers::{Local, SendSync},
///     prelude::*,
/// };
///
/// struct LoggingHook;
///
/// impl ReportCreationHook for LoggingHook {
///     fn on_local_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, Local>) {
///         println!("Local report created: {}", report);
///         let attachment = report_attachment!("Logged by LoggingHook");
///         report.attachments_mut().push(attachment.into());
///     }
///
///     fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, SendSync>) {
///         println!("SendSync report created: {}", report);
///         let attachment = report_attachment!("Logged by LoggingHook");
///         report.attachments_mut().push(attachment.into());
///     }
/// }
///
/// // Register the hook to activate it
/// register_report_creation_hook(LoggingHook);
/// ```
pub trait ReportCreationHook: 'static + Send + Sync {
    /// Called when a [`Local`] report is created.
    #[track_caller]
    fn on_local_creation(&self, report: ReportMut<'_, dyn Any, Local>);

    /// Called when a [`SendSync`] report is created.
    #[track_caller]
    fn on_sendsync_creation(&self, report: ReportMut<'_, dyn Any, SendSync>);
}

#[track_caller]
fn creation_hook_to_untyped<H>(hook: H) -> Box<dyn UntypedReportCreationHook>
where
    H: ReportCreationHook + Send + Sync + 'static,
{
    struct Hook<H> {
        hook: H,
        added_at: &'static Location<'static>,
    }

    impl<H> core::fmt::Display for Hook<H> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Report creation hook {} registered at {}:{}",
                core::any::type_name::<H>(),
                self.added_at.file(),
                self.added_at.line()
            )
        }
    }

    impl<H> UntypedReportCreationHook for Hook<H>
    where
        H: ReportCreationHook,
    {
        fn on_local_creation(&self, report: ReportMut<'_, dyn Any, Local>) {
            self.hook.on_local_creation(report);
        }

        fn on_sendsync_creation(&self, report: ReportMut<'_, dyn Any, SendSync>) {
            self.hook.on_sendsync_creation(report);
        }
    }

    let hook: Hook<H> = Hook {
        hook,
        added_at: Location::caller(),
    };
    Box::new(hook)
}

#[track_caller]
fn attachment_hook_to_untyped<A, H, C>(collector: C) -> Box<dyn UntypedReportCreationHook>
where
    A: 'static + Send + Sync,
    H: AttachmentHandler<A>,
    C: AttachmentCollectorHook<A> + Send + Sync + 'static,
{
    struct Hook<A, H, C>
    where
        A: 'static,
        H: 'static,
    {
        collector: C,
        added_at: &'static Location<'static>,
        _handled_type: core::marker::PhantomData<fn(A) -> A>,
        _handler: core::marker::PhantomData<fn(H) -> H>,
    }

    impl<A, H, C> UntypedReportCreationHook for Hook<A, H, C>
    where
        A: 'static + Send + Sync,
        H: AttachmentHandler<A>,
        C: AttachmentCollectorHook<A> + Send + Sync,
    {
        #[track_caller]
        fn on_local_creation(&self, mut report: ReportMut<'_, dyn Any, Local>) {
            let attachment = self.collector.collect();
            report
                .attachments_mut()
                .push(ReportAttachment::new_local_custom::<H>(attachment).into_dyn_any());
        }

        #[track_caller]
        fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn Any, SendSync>) {
            let attachment = self.collector.collect();
            report
                .attachments_mut()
                .push(ReportAttachment::new_sendsync_custom::<H>(attachment).into_dyn_any());
        }
    }
    impl<A, H, C> core::fmt::Display for Hook<A, H, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Attachment collector hook {} for attachment type {} with handler {} registered at {}:{}",
                core::any::type_name::<C>(),
                core::any::type_name::<A>(),
                core::any::type_name::<H>(),
                self.added_at.file(),
                self.added_at.line()
            )
        }
    }

    let hook = Hook {
        collector,
        added_at: core::panic::Location::caller(),
        _handled_type: core::marker::PhantomData,
        _handler: core::marker::PhantomData,
    };
    let hook: Box<Hook<A, H, C>> = Box::new(hook);

    hook
}

#[track_caller]
fn default_hooks() -> HookSet {
    #[allow(unused_mut)]
    let mut hooks = alloc::vec![attachment_hook_to_untyped::<_, LocationHandler, _>(
        LocationCollector,
    )];

    #[cfg(feature = "backtrace")]
    hooks.push(creation_hook_to_untyped(BacktraceCollector::new_from_env()));

    hooks
}

/// Registers a report creation hook that will be called whenever a report is
/// created.
///
/// Once registered, the hook will be automatically invoked for every report
/// that gets created in the application.
///
/// # Registration Order
///
/// Hooks are called in the order they were registered. Earlier registered hooks
/// will execute before later ones.
///
/// # Performance Considerations
///
/// Registered hooks will be called for *every* report creation, so they should
/// be designed to be fast. Heavy operations in hooks can significantly impact
/// the performance of error reporting throughout the application.
///
/// # Examples
///
/// ```rust
/// use rootcause::{
///     ReportMut,
///     hooks::report_creation::{ReportCreationHook, register_report_creation_hook},
///     markers::{Local, SendSync},
///     prelude::*,
///     report_attachment::ReportAttachment,
/// };
///
/// struct CountingHook {
///     counter: std::sync::atomic::AtomicUsize,
/// }
///
/// impl ReportCreationHook for CountingHook {
///     fn on_local_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, Local>) {
///         let count = self
///             .counter
///             .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
///         let attachment = report_attachment!("Report #{}", count);
///         report.attachments_mut().push(attachment.into());
///     }
///
///     fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn std::any::Any, SendSync>) {
///         let count = self
///             .counter
///             .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
///         let attachment = report_attachment!("Report #{}", count);
///         report.attachments_mut().push(attachment.into());
///     }
/// }
///
/// // Register the hook - it will now be called for all future reports
/// register_report_creation_hook(CountingHook {
///     counter: std::sync::atomic::AtomicUsize::new(0),
/// });
/// ```
#[track_caller]
pub fn register_report_creation_hook<H>(hook: H)
where
    H: ReportCreationHook + Send + Sync + 'static,
{
    HOOKS
        .write()
        .get()
        .get_or_insert_with(default_hooks)
        .push(creation_hook_to_untyped(hook));
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
/// use rootcause::hooks::report_creation::register_attachment_collector_hook;
///
/// // This closure automatically implements AttachmentCollectorHook<String>
/// register_attachment_collector_hook(|| "timestamp".to_string());
/// ```
///
/// # Examples
///
/// ## Custom Collector Implementation
///
/// ```rust
/// use rootcause::{
///     hooks::report_creation::{AttachmentCollectorHook, register_attachment_collector_hook},
///     prelude::*,
/// };
///
/// struct SystemInfoCollector;
///
/// impl AttachmentCollectorHook<String> for SystemInfoCollector {
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
/// // Register the collector
/// register_attachment_collector_hook(SystemInfoCollector);
/// ```
///
/// ## Using a Closure
///
/// ```rust
/// use rootcause::hooks::report_creation::register_attachment_collector_hook;
///
/// // Register a closure that collects the current working directory
/// register_attachment_collector_hook(|| {
///     std::env::current_dir()
///         .map(|p| p.display().to_string())
///         .unwrap_or_else(|_| "unknown".to_string())
/// });
/// ```
pub trait AttachmentCollectorHook<A>: 'static + Send + Sync {
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

impl<A, F> AttachmentCollectorHook<A> for F
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

/// Registers an attachment collector hook that will automatically collect and
/// attach data to reports.
///
/// Once registered, the collector will be automatically invoked for every
/// report that gets created. The collected data will be formatted using the
/// collector's associated handler and attached to the report.
///
/// Note that internally this function converts the attachment collector into
/// a report creation hook and registers it using the same system as
/// [`register_report_creation_hook`].
///
/// # Registration Order
///
/// Attachment collectors are called in the order they were registered, after
/// any general report creation hooks have been executed.
///
/// # Performance Considerations
///
/// Registered collectors will be called for *every* report creation, so they
/// should be designed to be fast. Heavy operations in collectors can
/// significantly impact the performance of error reporting throughout the
/// application.
///
/// # Examples
///
/// ## Registering a Custom Collector
///
/// ```rust
/// use rootcause::{
///     hooks::report_creation::{AttachmentCollectorHook, register_attachment_collector_hook},
///     prelude::*,
/// };
///
/// struct MemoryUsageCollector;
/// #[derive(Debug)]
/// struct MemoryInfo {
///     used_mb: u64,
/// }
///
/// impl AttachmentCollectorHook<MemoryInfo> for MemoryUsageCollector {
///     type Handler = handlers::Debug;
///
///     fn collect(&self) -> MemoryInfo {
///         // This is a simplified example - in practice you'd use a proper method
///         // to get memory usage information
///         MemoryInfo { used_mb: 45 }
///     }
/// }
///
/// // Register the collector - it will now collect memory info for all reports
/// register_attachment_collector_hook(MemoryUsageCollector);
/// ```
///
/// ## Registering a Closure Collector
///
/// ```rust
/// use rootcause::hooks::report_creation::register_attachment_collector_hook;
///
/// // Register a simple closure that collects the current thread ID as a string
/// register_attachment_collector_hook(|| {
///     format!("Created on thread_id={:?}", std::thread::current().id())
/// });
/// ```
#[track_caller]
pub fn register_attachment_collector_hook<A, C>(collector: C)
where
    A: 'static + Send + Sync,
    C: AttachmentCollectorHook<A> + Send + Sync + 'static,
{
    HOOKS
        .write()
        .get()
        .get_or_insert_with(default_hooks)
        .push(attachment_hook_to_untyped::<A, C::Handler, C>(collector));
}

#[track_caller]
fn get_hooks() -> HookLockReadGuard<HookSet> {
    let read_guard = HOOKS.read();
    if read_guard.get().is_some() {
        read_guard
    } else {
        core::mem::drop(read_guard);
        HOOKS.write().get().get_or_insert_with(default_hooks);
        HOOKS.read()
    }
}

#[track_caller]
pub(crate) fn run_creation_hooks_local(mut report: ReportMut<'_, dyn Any, Local>) {
    if let Some(hooks) = get_hooks().get() {
        for hook in hooks {
            hook.on_local_creation(report.as_mut());
        }
    }
}

#[track_caller]
pub(crate) fn run_creation_hooks_sendsync(mut report: ReportMut<'_, dyn Any, SendSync>) {
    if let Some(hooks) = get_hooks().get() {
        for hook in hooks {
            hook.on_sendsync_creation(report.as_mut());
        }
    }
}
