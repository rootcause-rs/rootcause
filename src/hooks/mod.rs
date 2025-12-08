//! Hooks system for customizing report creation and formatting behavior.
//!
//! # When to Use Hooks
//!
//! **Most users don't need hooks** - the defaults work well. Use hooks when you
//! need to:
//! - Automatically attach data to ALL errors (request IDs, timestamps,
//!   environment info)
//! - Integrate with custom logging or observability systems
//! - Change how reports are formatted globally (different colors, layout,
//!   structure)
//! - Redact or transform sensitive data in error messages
//!
//! **If you just need to customize a single error**, use `.attach()` or
//! handlers (see [`examples/custom_handler.rs`]) instead of hooks.
//!
//! [`examples/custom_handler.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/custom_handler.rs
//!
//! # Modules
//!
//! - **[`report_creation`]**: Automatically add data to every report as it's
//!   created (e.g., request IDs, correlation IDs, environment variables)
//!
//! - **[`attachment_formatter`]** and **[`context_formatter`]**: Control how
//!   specific types appear in error messages (e.g., redact passwords, format
//!   timestamps, control attachment placement)
//!
//! - **[`report_formatting`]**: Change the entire report layout and structure
//!   (e.g., JSON output for logging, compact format, custom colors)
//!
//! - **[`builtin_hooks`]**: Default hooks that are automatically registered
//!   (location collectors, backtrace collectors, and the default formatter)
//!
//! See [`examples/report_creation_hook.rs`] and
//! [`examples/formatting_hooks.rs`] for complete examples.
//!
//! [`examples/report_creation_hook.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/report_creation_hook.rs
//! [`examples/formatting_hooks.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/formatting_hooks.rs

pub mod attachment_formatter;
pub mod builtin_hooks;
pub mod context_formatter;
pub mod report_creation;
pub mod report_formatting;

use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    marker::PhantomData,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use self::{
    attachment_formatter::AttachmentFormatterHook,
    builtin_hooks::location::{Location, LocationHandler, LocationHook},
    context_formatter::ContextFormatterHook,
    report_creation::{
        AttachmentCollector, ReportCreationHook, UntypedReportCreationHook,
        attachment_hook_to_untyped, creation_hook_to_untyped,
    },
    report_formatting::ReportFormatter,
};

/// Builder for configuring and installing hooks globally.
///
/// Hooks allow you to customize how reports are created and formatted across
/// your entire application. Unlike the older `register_*` functions, `Hooks`
/// uses a builder pattern without locks for better performance and flexibility.
///
/// # Examples
///
/// ```rust
/// use rootcause::{hooks::Hooks, prelude::*};
///
/// // Create a new Hooks builder with default hooks
/// let hooks = Hooks::new().attachment_collector(|| "Custom data".to_string());
///
/// // Install globally (can only be done once)
/// hooks.install().expect("hooks already installed");
/// ```
///
/// See also the individual hook modules for more details:
/// - [`report_creation`] - Add data automatically when reports are created
/// - [`attachment_formatter`] and [`context_formatter`] - Customize formatting of specific types
/// - [`report_formatting`] - Change the entire report layout
#[derive(Debug)]
pub struct Hooks(Box<HookData>);

#[derive(Debug)]
pub(crate) struct HookData {
    pub(crate) report_creation: Vec<Box<dyn UntypedReportCreationHook>>,
    pub(crate) attachment_formatters: attachment_formatter::HookMap,
    pub(crate) context_formatters: context_formatter::HookMap,
    pub(crate) report_formatter: Option<Box<dyn ReportFormatter>>,
    #[allow(dead_code, reason = "only used for debugging purposes")]
    pub(crate) created_at: Location,
}

/// Error returned when attempting to install hooks when they're already
/// installed.
///
/// Contains the hooks that were attempted to be installed, allowing you to
/// recover them if needed.
pub struct HooksAlreadyInstalled(pub Hooks);

impl core::fmt::Debug for HooksAlreadyInstalled {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HooksAlreadyInstalled").finish()
    }
}

impl core::fmt::Display for HooksAlreadyInstalled {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "hooks are already installed globally")
    }
}

impl core::error::Error for HooksAlreadyInstalled {}

impl Hooks {
    /// Creates a new `Hooks` builder with the default built-in hooks.
    ///
    /// By default, this includes:
    /// - Location tracking for capturing file/line information
    ///
    /// See also [`new_without_builtin_hooks`](Self::new_without_builtin_hooks)
    /// if you want full control over which hooks are registered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new().attachment_collector(|| std::process::id());
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    #[track_caller]
    pub fn new() -> Self {
        Self(Box::new(HookData {
            report_creation: vec![attachment_hook_to_untyped::<_, LocationHandler, _>(
                LocationHook,
            )],
            attachment_formatters: Default::default(),
            context_formatters: Default::default(),
            report_formatter: None,
            created_at: Location::caller(),
        }))
    }

    /// Creates a new `Hooks` builder without any built-in hooks.
    ///
    /// This gives you full control over which hooks are registered, but you'll
    /// need to manually add any hooks you want (including location tracking).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks =
    ///     Hooks::new_without_builtin_hooks().attachment_collector(|| "custom data".to_string());
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn new_without_builtin_hooks() -> Self {
        Self(Box::new(HookData {
            report_creation: Vec::new(),
            attachment_formatters: Default::default(),
            context_formatters: Default::default(),
            report_formatter: None,
            created_at: Location::caller(),
        }))
    }

    /// Registers an attachment collector hook that automatically collects and
    /// attaches data to every report.
    ///
    /// This is useful for adding consistent metadata like request IDs,
    /// timestamps, or environment information to all errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::{hooks::Hooks, prelude::*};
    ///
    /// let hooks = Hooks::new()
    ///     // Simple closure that returns Display + Debug types
    ///     .attachment_collector(|| std::process::id())
    ///     .attachment_collector(|| "Environment: production".to_string());
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn attachment_collector<A, C>(mut self, collector: C) -> Self
    where
        A: 'static + Send + Sync,
        C: AttachmentCollector<A> + Send + Sync + 'static,
    {
        self.0
            .report_creation
            .push(attachment_hook_to_untyped::<A, C::Handler, C>(collector));
        self
    }

    /// Registers a report creation hook for advanced customization.
    ///
    /// Use this when you need conditional logic or access to the full report
    /// during creation. For simple data collection, prefer
    /// [`attachment_collector`](Self::attachment_collector).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::{
    ///     ReportMut,
    ///     hooks::Hooks,
    ///     markers::{Dynamic, Local, SendSync},
    ///     prelude::*,
    /// };
    ///
    /// struct MyHook;
    ///
    /// impl rootcause::hooks::report_creation::ReportCreationHook for MyHook {
    ///     fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, Local>) {
    ///         // Add custom logic here
    ///     }
    ///
    ///     fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, SendSync>) {
    ///         // Add custom logic here
    ///     }
    /// }
    ///
    /// let hooks = Hooks::new().report_creation_hook(MyHook);
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn report_creation_hook<H>(mut self, hook: H) -> Self
    where
        H: ReportCreationHook + Send + Sync + 'static,
    {
        self.0.report_creation.push(creation_hook_to_untyped(hook));
        self
    }

    /// Registers a formatter for a specific attachment type.
    ///
    /// This controls how attachments of type `A` are displayed in error
    /// reports, including their placement, priority, and formatting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::{
    ///     handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
    ///     hooks::{Hooks, attachment_formatter::AttachmentFormatterHook},
    ///     markers::Dynamic,
    ///     report_attachment::ReportAttachmentRef,
    /// };
    ///
    /// struct MyData(String);
    /// struct MyFormatter;
    ///
    /// impl AttachmentFormatterHook<MyData> for MyFormatter {
    ///     fn preferred_formatting_style(
    ///         &self,
    ///         _: ReportAttachmentRef<'_, Dynamic>,
    ///         _: FormattingFunction,
    ///     ) -> AttachmentFormattingStyle {
    ///         AttachmentFormattingStyle {
    ///             placement: AttachmentFormattingPlacement::Inline,
    ///             function: FormattingFunction::Display,
    ///             priority: 100,
    ///         }
    ///     }
    /// }
    ///
    /// let hooks = Hooks::new().attachment_formatter::<MyData, _>(MyFormatter);
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn attachment_formatter<A, H>(mut self, hook: H) -> Self
    where
        A: Sized + 'static,
        H: AttachmentFormatterHook<A>,
    {
        self.0.attachment_formatters.insert::<A, H>(hook);
        self
    }

    /// Registers a formatter for a specific context (error) type.
    ///
    /// This controls how contexts of type `C` are displayed when they appear
    /// as the main error in a report.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::fmt;
    ///
    /// use rootcause::{
    ///     ReportRef,
    ///     hooks::{Hooks, context_formatter::ContextFormatterHook},
    ///     markers::{Local, Uncloneable},
    /// };
    ///
    /// struct MyError(String);
    /// struct MyFormatter;
    ///
    /// impl ContextFormatterHook<MyError> for MyFormatter {
    ///     fn display(
    ///         &self,
    ///         report: ReportRef<'_, MyError, Uncloneable, Local>,
    ///         f: &mut fmt::Formatter<'_>,
    ///     ) -> fmt::Result {
    ///         write!(f, "Custom: {}", report.current_context().0)
    ///     }
    /// }
    ///
    /// let hooks = Hooks::new().context_formatter::<MyError, _>(MyFormatter);
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn context_formatter<C, H>(mut self, hook: H) -> Self
    where
        C: Sized + 'static,
        H: ContextFormatterHook<C>,
    {
        self.0.context_formatters.insert::<C, H>(hook);
        self
    }

    /// Registers a hook for formatting entire reports.
    ///
    /// This controls the overall layout, structure, and appearance of error
    /// reports. Only one report formatting hook can be active at a time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::{Hooks, builtin_hooks::report_formatter::DefaultReportFormatter};
    ///
    /// // Use ASCII-only formatting
    /// let hooks = Hooks::new().report_formatter(DefaultReportFormatter::ASCII);
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn report_formatter<H>(mut self, hook: H) -> Self
    where
        H: ReportFormatter + 'static,
    {
        self.0.report_formatter = Some(Box::new(hook));
        self
    }

    /// Installs the hooks globally.
    ///
    /// If hooks are already installed, returns an error
    /// [`HooksAlreadyInstalled`], containing the hooks that were attempted
    /// to be installed.
    ///
    /// See also [`replace`](Self::replace) which will replace any existing
    /// hooks without erroring.
    ///
    /// # Memory Management
    ///
    /// After installing hooks globally, the memory for the hooks will be
    /// leaked and remain for the lifetime of the program. This happens even
    /// if the hooks are later replaced with other hooks. This is by design for
    /// thread-safety and performance - no locks are needed to access hooks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new().attachment_collector(|| "custom data".to_string());
    ///
    /// // First installation succeeds
    /// hooks.install().expect("failed to install hooks");
    ///
    /// // Second installation would fail
    /// Hooks::new().install().unwrap_err();
    /// ```
    pub fn install(self) -> Result<(), HooksAlreadyInstalled> {
        let boxed = Box::into_raw(self.0);
        match HOOKS.compare_exchange(
            core::ptr::null_mut(),
            boxed,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Restore ownership to avoid leak
                let hooks = unsafe { Box::from_raw(boxed) };
                Err(HooksAlreadyInstalled(Hooks(hooks)))
            }
        }
    }

    /// Replaces the currently installed hooks with `self`.
    ///
    /// Returns the previously installed hooks, if any, as a [`LeakedHooks`]
    /// handle.
    ///
    /// See also [`install`](Self::install) which will error if hooks are
    /// already installed.
    ///
    /// # Memory Management
    ///
    /// After installing hooks globally, the memory for the hooks will be
    /// leaked and remain for the lifetime of the program. This happens even
    /// if the hooks are later replaced with other hooks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new().attachment_collector(|| "first".to_string());
    /// hooks.install().expect("failed to install hooks");
    ///
    /// // Replace with different hooks
    /// let hooks2 = Hooks::new().attachment_collector(|| "second".to_string());
    /// let _previous = hooks2.replace();
    /// ```
    pub fn replace(self) -> Option<LeakedHooks> {
        self.leak().replace()
    }

    /// Leaks the hooks, returning a [`LeakedHooks`] handle.
    ///
    /// This is useful for installing the hooks later using
    /// [`LeakedHooks::replace`].
    pub fn leak(self) -> LeakedHooks {
        let ptr = Box::into_raw(self.0);
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        LeakedHooks {
            hook_data: ptr,
            _marker: PhantomData,
        }
    }
}

/// A handle to hooks that have been leaked into static memory.
///
/// This allows you to replace hooks multiple times without having to create
/// new hooks each time. The hooks remain in memory for the lifetime of the
/// program.
#[derive(Copy, Clone)]
pub struct LeakedHooks {
    hook_data: NonNull<HookData>,
    _marker: PhantomData<&'static HookData>,
}

impl core::fmt::Debug for LeakedHooks {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LeakedHooks")
            .field("hook_data", self.hook_data())
            .finish()
    }
}

impl LeakedHooks {
    /// Fetches the currently installed hooks, if any.
    pub fn fetch_current_hooks() -> Option<Self> {
        let current = HOOKS.load(Ordering::Acquire);
        let current = NonNull::new(current)?;
        Some(LeakedHooks {
            hook_data: current,
            _marker: PhantomData,
        })
    }

    /// Replaces the currently installed hooks with `self`.
    ///
    /// Returns the previously installed hooks, if any.
    pub fn replace(self) -> Option<LeakedHooks> {
        let previous = HOOKS.swap(self.hook_data.as_ptr(), Ordering::AcqRel);
        let previous = NonNull::new(previous)?;
        Some(LeakedHooks {
            hook_data: previous,
            _marker: PhantomData,
        })
    }

    fn hook_data(self) -> &'static HookData {
        unsafe { self.hook_data.as_ref() }
    }
}

static HOOKS: AtomicPtr<HookData> = AtomicPtr::new(core::ptr::null_mut());

impl HookData {
    pub(crate) fn fetch() -> Option<&'static HookData> {
        Some(LeakedHooks::fetch_current_hooks()?.hook_data())
    }
}
