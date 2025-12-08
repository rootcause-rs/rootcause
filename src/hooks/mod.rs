//! Hooks system for customizing report creation and formatting behavior.
//!
//! # Quick Start
//!
//! ```rust
//! use rootcause::hooks::Hooks;
//!
//! // Automatically attach request IDs to all errors
//! Hooks::new()
//!     .attachment_collector(|| format!("Request: {}", get_request_id()))
//!     .install()
//!     .expect("failed to install hooks");
//!
//! fn get_request_id() -> u64 {
//!     42
//! } // Your implementation here
//! ```
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
//! # Hook Types
//!
//! ## Creation Hooks (add data to errors)
//!
//! - **[`report_creation`]**: Automatically attach data when errors are created
//!   (e.g., request IDs, timestamps, environment info)
//!
//! ## Formatting Hooks (control how things are displayed)
//!
//! - **[`attachment_formatter`]**: Format individual pieces of attached data
//!   (e.g., hide passwords, format timestamps, control where data appears)
//!
//! - **[`context_formatter`]**: Format the main error message itself (e.g.,
//!   custom error descriptions, add context, structured output)
//!
//! - **[`report_formatter`]**: Format the entire report structure and layout
//!   (e.g., ASCII vs Unicode, colors, JSON output, custom layouts)
//!
//! ## Built-in Components
//!
//! - **[`builtin_hooks`]**: Location tracking and default report formatter
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
pub mod report_formatter;

use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use self::{
    attachment_formatter::AttachmentFormatterHook,
    builtin_hooks::location::{Location, LocationHandler, LocationHook},
    context_formatter::ContextFormatterHook,
    report_creation::{
        AttachmentCollector, ReportCreationHook, StoredCreationHook, attachment_hook_to_untyped,
        creation_hook_to_untyped,
    },
    report_formatter::ReportFormatter,
};

/// Builder for configuring and installing hooks globally.
///
/// Hooks allow you to customize how reports are created and formatted across
/// your entire application. The builder pattern lets you chain multiple hook
/// configurations together before installing them globally.
///
/// # Hook Types
///
/// **Creation Hooks** (add data to errors):
/// - [`attachment_collector()`](Self::attachment_collector) - Automatically
///   attach data to all errors
/// - [`report_creation_hook()`](Self::report_creation_hook) - Conditional logic
///   during error creation
///
/// **Formatting Hooks** (control how things are displayed):
/// - [`attachment_formatter()`](Self::attachment_formatter) - Format individual
///   attached data
/// - [`context_formatter()`](Self::context_formatter) - Format main error
///   messages
/// - [`report_formatter()`](Self::report_formatter) - Customize entire report
///   layout
///
/// # Examples
///
/// Simple attachment collection:
/// ```rust
/// use rootcause::hooks::Hooks;
///
/// // Automatically attach process ID to all errors
/// Hooks::new()
///     .attachment_collector(|| format!("Process id: {}", std::process::id()))
///     .install()
///     .expect("failed to install hooks");
/// ```
///
/// Combining multiple hooks:
/// ```rust
/// use rootcause::hooks::{Hooks, builtin_hooks::report_formatter::DefaultReportFormatter};
///
/// Hooks::new()
///     .attachment_collector(|| "Running on production".to_string())
///     .report_formatter(DefaultReportFormatter::ASCII)
///     .install()
///     .expect("failed to install hooks");
/// ```
///
/// See also:
/// - [`report_creation`] - Add data automatically when reports are created
/// - [`attachment_formatter`] and [`context_formatter`] - Customize formatting
///   of specific types
/// - [`report_formatter`] - Change the entire report layout
#[derive(Debug)]
pub struct Hooks(Box<HookData>);

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(crate) struct HookData {
    pub(crate) report_creation: Vec<Box<dyn StoredCreationHook>>,
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
pub struct HooksAlreadyInstalledError(pub Hooks);

impl core::fmt::Debug for HooksAlreadyInstalledError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HooksAlreadyInstalledError").finish()
    }
}

impl core::fmt::Display for HooksAlreadyInstalledError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "hooks are already installed globally")
    }
}

impl core::error::Error for HooksAlreadyInstalledError {}

impl Hooks {
    /// Creates a new `Hooks` builder with location tracking.
    ///
    /// This is equivalent to using rootcause without installing any hooks - you
    /// get automatic location tracking. Use this as the base when you want to
    /// add additional hooks while keeping location tracking.
    ///
    /// See also [`new_without_locations`](Self::new_without_locations) to
    /// disable automatic location tracking.
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

    /// Creates a new `Hooks` builder without location tracking.
    ///
    /// By default, rootcause automatically tracks source locations where errors
    /// occur. Use this method when you don't want that automatic tracking.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new_without_locations().attachment_collector(|| "custom data".to_string());
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    pub fn new_without_locations() -> Self {
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
    /// [`HooksAlreadyInstalledError`], containing the hooks that were attempted
    /// to be installed.
    ///
    /// See also [`replace`](Self::replace) which will replace any existing
    /// hooks without erroring.
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
    /// let hooks = Hooks::new().attachment_collector(|| "custom data".to_string());
    ///
    /// // First installation succeeds
    /// hooks.install().expect("failed to install hooks");
    ///
    /// // Second installation would fail
    /// Hooks::new().install().unwrap_err();
    /// ```
    pub fn install(self) -> Result<(), HooksAlreadyInstalledError> {
        let boxed = Box::into_raw(self.0);

        // SAFETY:
        //
        // 1. The pointer `boxed` is valid and was obtained from `Box::into_raw`.
        // 2. On success, the pointer will not be used anymore.
        // 3. On failure, the pointer remains owned by us.
        let install_result = unsafe { HOOKS.install(boxed) };

        match install_result {
            Ok(()) => Ok(()),
            Err(()) => {
                // SAFETY:
                //
                // - This pointer was obtained from Box::into_raw above, so it is valid to
                //   convert it back into a Box.
                // - Since installation failed, we own the pointer, so it's safe to convert it
                //   back into a Box here.
                let hooks = unsafe { Box::from_raw(boxed) };

                Err(HooksAlreadyInstalledError(Hooks(hooks)))
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
    /// # unsafe { _previous.unwrap().reclaim() }; // Miri doesn't like leaking memory
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
        let ptr = NonNull::new(ptr).expect("Box::into_raw returned null pointer");
        LeakedHooks { hook_data: ptr }
    }
}

/// A handle to hooks that have been leaked into static memory.
///
/// This allows you to replace hooks multiple times without having to create
/// new hooks each time. The hooks remain in memory for the lifetime of the
/// program.
#[derive(Copy, Clone, Debug)]
pub struct LeakedHooks {
    /// # Safety
    ///
    /// 1. This pointer points to a valid HookData that has been created by
    ///    calling `Box::into_raw` on a `Box<HookData>`.
    /// 2. The HookData pointed to has been leaked and will remain valid for the
    ///    lifetime of the program, unless reclaimed using
    ///    [`LeakedHooks::reclaim`].
    hook_data: NonNull<HookData>,
}

impl LeakedHooks {
    /// Fetches the currently installed hooks, if any.
    pub fn fetch_current_hooks() -> Option<Self> {
        Some(Self {
            hook_data: HOOKS.fetch()?,
        })
    }

    /// Replaces the currently installed hooks with `self`.
    ///
    /// Returns the previously installed hooks, if any.
    pub fn replace(self) -> Option<LeakedHooks> {
        Some(Self {
            hook_data: HOOKS.replace(self.hook_data)?,
        })
    }

    /// Reclaims ownership of the leaked hooks, returning them as a `Hooks`
    /// instance.
    ///
    /// **⚠ WARNING: This function is almost impossible to use safely. Do not
    /// call it unless you are okay with undefined behavior, or unless you
    /// have global knowledge about the entire execution state of the
    /// program that justifies why it is safe.**
    ///
    /// # Safety
    ///
    /// To call this function safely, the caller must at the very least ensure
    /// that no other references to these hooks exist. These references can take
    /// many forms, including:
    ///
    /// 1. The hooks being currently installed globally.
    /// 2. This or other threads currently creating or formatting reports.
    /// 3. This or other threads holding onto a `LeakedHooks` instance that point
    ///    to the same hooks. These instances could for instance have been obtained
    ///    using [`LeakedHooks::fetch_current_hooks`].
    /// 4. This or other threads holding onto references obtained from the current
    ///    hooks. This might plausibly be done from inside custom hooks.
    pub unsafe fn reclaim(self) -> Hooks {
        // SAFETY:
        // - The caller has guaranteed that no other references to these hooks exist,
        //   so in principle it might be safe to reclaim ownership.
        // - While other parts of this file promise that the hooks remain for the lifetime
        //   of the program, the user has promised that no other references to
        //   the pointer exist, so while these guarantees are broken, there *should* be no way
        //   for it to lead to undefined behavior.
        // - In any case, the caller has explicitly opted into calling this function and promised
        //   that they have ensured safety.
        let boxed = unsafe { Box::from_raw(self.hook_data.as_ptr()) };
        Hooks(boxed)
    }
}

struct GlobalHooks {
    /// # Safety
    ///
    /// 1. This pointer will either be null, or point to a valid HookData that
    ///    has been created using `Box::into_raw`.
    /// 2. The pointer will remain valid for the lifetime of the program once set,
    ///    or until replaced with another valid pointer and then reclaimed using
    ///    `LeakedHooks::reclaim`.
    /// 3. All writing to the `AtomicPtr` is done using release semantics.
    /// 4. All reading from the `AtomicPtr` is done using acquire semantics when
    ///    the pointer will be dereferenced and with relaxed semantics
    ///    otherwise.
    ptr: AtomicPtr<HookData>,
}

impl GlobalHooks {
    const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Fetches the currently installed hooks, if any.
    ///
    /// The returned pointer is guaranteed to come from a call to
    /// `Box::into_raw` on a `Box<HookData>`. It is also guaranteed
    /// to remain valid for the lifetime of the program, or until replaced
    /// with another valid pointer and reclaimed using `LeakedHooks::reclaim`.
    fn fetch(&self) -> Option<NonNull<HookData>> {
        let ptr = self.ptr.load(Ordering::Acquire);
        NonNull::new(ptr)
    }

    /// Installs new hooks, returning an error if hooks are already installed.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The `new` pointer is valid and points to a `Box<HookData>` that has
    ///    been turned into a raw pointer using `Box::into_raw`.
    /// 2. On success the function claims ownership of the `new` pointer, and it
    ///    cannot be used by the caller anymore.
    /// 3. On failure, the `new` pointer remains owned by the caller and it is
    ///    their responsibility to manage its memory.
    unsafe fn install(&self, new: *mut HookData) -> Result<(), ()> {
        match self.ptr.compare_exchange(
            core::ptr::null_mut(),
            new,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    /// Replaces the currently installed hooks with `new`.
    ///
    /// The `new` pointer's ownership is claimed by this function.
    ///
    /// Returns the previously installed hooks, if any.
    fn replace(&self, new: NonNull<HookData>) -> Option<NonNull<HookData>> {
        let previous = self.ptr.swap(new.as_ptr(), Ordering::AcqRel);
        NonNull::new(previous)
    }
}

static HOOKS: GlobalHooks = GlobalHooks::new();

impl HookData {
    pub(crate) fn fetch() -> Option<&'static HookData> {
        let ptr = HOOKS.fetch()?;

        // SAFETY:
        //
        // - This pointer was obtained from Box::into_raw, so it is valid to
        //   convert it back into a reference.
        // - The pointer remains valid for the lifetime of the program or
        //   until replaced and then reclaimed using `LeakedHooks::reclaim`. However,
        //   for any such reclaiming to occur, they must ensure that no other
        //   references exist. This means we are free to assume that the pointer is valid
        //   here.
        let ptr = unsafe { ptr.as_ref() };

        Some(ptr)
    }
}
