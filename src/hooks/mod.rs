//! Hooks system for customizing report creation and formatting behavior.
//!
//! Hooks are global callback functions that intercept error creation and
//! formatting events, allowing you to automatically add data or customize
//! display across your entire application.
//!
//! # Quick Start
//!
//! ```
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
        AttachmentCollector, ReportCreationHook, StoredReportCreationHook,
        attachment_hook_to_stored_hook, creation_hook_to_stored_hook,
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
/// ```
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
/// ```
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

// The lifetime of an instance of the `HookData` struct is as follows:
//
// ### `Hooks`
//
// - Creation: A `HookData` is allocated using `Box::new()` when `Hooks::new()`
//   or `Hooks::new_without_locations()` is called.
// - `Hooks->GlobalHooks`: When the `install()` method is called on a `Hooks`
//   instance, the allocated `HookData` is either transferred to the global
//   hooks storage or returned back to the caller in case of an error.
// - `Hooks->HooksHandle`: When the `leak()` method is called on a `Hooks`
//   instance, the allocated `HookData` is transferred to a `HooksHandle`
//   instance.
// - Deallocation: If the `Hooks` object is dropped without calling `install()`
//   or `leak()`, then the `HookData` is deallocated and its memory is freed.
//
// ### `GlobalHooks`
//
// - Initialization: The `GlobalHooks` is initialized using a null pointer,
//   indicating that no hooks are installed initially.
// - `GlobalHooks`->`use_hooks`: The global hooks storage may be accessed by
//   multiple threads concurrently via the `use_hooks` function, which provides
//   temporary access to the `HookData`. The `HookData` must remain valid for
//   the duration of these accesses.
// - `GlobalHooks`->`HooksHandle`: When the `replace()` method is called on a
//   `HooksHandle` instance, the allocated `HookData` is transferred to the
//   global hooks storage, replacing any existing hooks. The previous hooks, if
//   any, are returned as a new `HooksHandle` instance.
// - If the `replace()` method is not called, then the `HookData` remains in
//   memory for the lifetime of the program.
//
// ### `HooksHandle`
//
// - `HooksHandle->GlobalHooks`: When the `replace()` method is called on a
//   `HooksHandle` instance, the allocated `HookData` is transferred to the
//   global hooks storage, replacing any existing hooks.
// - `HooksHandle->Hooks`: When the `reclaim()` method is called on a
//   `HooksHandle` instance, the allocated `HookData` is transferred back to a
//   `Hooks` instance, allowing further modifications or deallocation. Since the
//   `HooksHandle` instance might previously have been installed globally, the
//   `use_hooks` function might currently be accessing the same `HookData`.
//   Therefore, care must be taken to ensure that no concurrent accesses are
//   happening when reclaiming the hooks.
// - Deallocation: If the `HooksHandle` object is dropped without calling
//   `replace()` or `reclaim()`, the `HookData` is leaked.
#[derive(Debug)]
pub(crate) struct HookData {
    pub(crate) report_creation: Vec<Box<dyn StoredReportCreationHook>>,
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
///
/// # Examples
///
/// ```should_panic
/// use rootcause::hooks::Hooks;
///
/// Hooks::new().install().unwrap();
/// // Second install fails
/// Hooks::new().install().unwrap();
/// ```
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
    /// ```
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new().attachment_collector(|| std::process::id());
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    #[track_caller]
    pub fn new() -> Self {
        Self(Box::new(HookData {
            report_creation: vec![attachment_hook_to_stored_hook::<_, LocationHandler, _>(
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
    /// ```
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new_without_locations().attachment_collector(|| "custom data".to_string());
    ///
    /// hooks.install().expect("failed to install hooks");
    /// ```
    #[track_caller]
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
    /// This accepts any type implementing [`AttachmentCollector`], including
    /// closures (which have a blanket implementation). Use this for the common
    /// case of simply adding data to all errors. For more control, such as
    /// conditional logic based on the error type, use
    /// [`report_creation_hook`](Self::report_creation_hook) instead.
    ///
    /// [`AttachmentCollector`]: report_creation::AttachmentCollector
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{hooks::Hooks, prelude::*};
    ///
    /// let hooks = Hooks::new()
    ///     // Simple closure that returns Display + Debug types
    ///     .attachment_collector(|| format!("Thread ID: {:?}", std::thread::current().id()))
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
            .push(attachment_hook_to_stored_hook::<A, C::Handler, C>(
                collector,
            ));
        self
    }

    /// Registers a report creation hook for advanced customization.
    ///
    /// Use this when you need conditional logic or access to the full report
    /// during creation. For the common case of simply attaching data to all
    /// errors, use [`attachment_collector`](Self::attachment_collector)
    /// instead, which is easier to use.
    ///
    /// # Examples
    ///
    /// ```
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
        self.0
            .report_creation
            .push(creation_hook_to_stored_hook(hook));
        self
    }

    /// Registers a formatter for a specific attachment type.
    ///
    /// This controls how attachments of type `A` are displayed in error
    /// reports, including their placement, priority, and formatting.
    ///
    /// # Examples
    ///
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
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
        // 3. On failure, the pointer remains owned by us and the install function will
        //    not create any additional references to it.
        let install_result = unsafe {
            // @add-unsafe-context: GlobalHooks
            // @add-unsafe-context: HooksHandle
            // @add-unsafe-context: Hooks
            // @add-unsafe-context: HookData
            // @add-unsafe-context: use_hooks
            // @add-unsafe-context: HOOKS
            HOOKS.install(boxed)
        };

        match install_result {
            Ok(()) => Ok(()),
            Err(()) => {
                // SAFETY:
                //
                // - This pointer was obtained from Box::into_raw above, so it is valid to
                //   convert it back into a Box.
                // - Since installation failed, we own the pointer and have the only reference
                //   to it.
                let hooks = unsafe { Box::from_raw(boxed) };

                Err(HooksAlreadyInstalledError(Hooks(hooks)))
            }
        }
    }

    /// Replaces the currently installed hooks with `self`.
    ///
    /// Returns the previously installed hooks, if any, as a [`HooksHandle`]
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
    /// ```
    /// use rootcause::hooks::Hooks;
    ///
    /// let hooks = Hooks::new().attachment_collector(|| "first".to_string());
    /// hooks.install().expect("failed to install hooks");
    ///
    /// // Replace with different hooks
    /// let hooks2 = Hooks::new().attachment_collector(|| "second".to_string());
    /// let previous = hooks2.replace();
    /// # unsafe { previous.unwrap().reclaim(); } // Clean up as Miri does not like memory leaks
    /// ```
    pub fn replace(self) -> Option<HooksHandle> {
        self.leak().replace()
    }

    /// Leaks the hooks, returning a [`HooksHandle`] handle.
    ///
    /// This is useful for installing the hooks later using
    /// [`HooksHandle::replace`].
    fn leak(self) -> HooksHandle {
        let ptr = Box::into_raw(self.0);
        let ptr = NonNull::new(ptr).expect("Box::into_raw returned null pointer");
        HooksHandle { hook_data: ptr }
    }
}

/// A handle to hooks that have been leaked into static memory.
///
/// You get a `HooksHandle` when calling [`Hooks::replace()`], which returns
/// the previously installed hooks. The hooks remain in memory for the lifetime
/// of the program unless you call the unsafe [`reclaim()`](Self::reclaim)
/// method.
///
/// # What to do with a HooksHandle
///
/// - **Call [`replace()`](Self::replace)** to install these hooks again,
///   swapping them with the currently active hooks
/// - **Drop it** to intentionally leak the memory (safe, but permanent)
/// - **Call [`reclaim()`](Self::reclaim)** (unsafe) to recover the memory -
///   only if you can guarantee no code is using these hooks anymore
///
/// # Memory Leak Warning
///
/// Dropping a `HooksHandle` **will leak memory**. This is by design - the hooks
/// might still be referenced by other threads, so we can't safely deallocate
/// them. In most applications this is fine since you typically install hooks
/// once at startup. If you're replacing hooks frequently in tests or hot-reload
/// scenarios, be aware of this behavior.
///
/// # Examples
///
/// ```
/// use rootcause::hooks::Hooks;
///
/// // Install initial hooks
/// Hooks::new()
///     .attachment_collector(|| "v1")
///     .install()
///     .unwrap();
///
/// // Replace with new hooks, getting back the old ones
/// let old_hooks = Hooks::new().attachment_collector(|| "v2").replace();
///
/// // Option 1: Drop and leak (typical case)
/// # // We don't want to leak in the doctest, as Miri will complain about a memory leak
/// # let saved_old_hooks = old_hooks;
/// # let old_hooks = ();
/// let _ = old_hooks;
///
/// // Option 2: Replace again to swap back
/// # // Restore the saved hooks, and save the ones we replace with
/// # // so we can free them in option 3
/// # let old_hooks = saved_old_hooks;
/// # let old_hooks =
/// old_hooks.unwrap().replace();
///
/// // Option 3: Unsafe reclaim (only if you know it's safe!)
/// if let Some(old_hooks) = old_hooks {
///     unsafe {
///         old_hooks.reclaim();
///     }
/// }
/// ```
#[derive(Debug)]
#[allow(
    missing_copy_implementations,
    reason = "ownership semantics require move-only"
)]
pub struct HooksHandle {
    /// # Safety
    ///
    /// 1. This pointer points to a valid HookData that has been created by
    ///    calling `Box::into_raw` on a `Box<HookData>`.
    /// 2. This struct has exclusive ownership of the pointer. No other
    ///    `HooksHandle` instances exist pointing to the same data.
    /// 3. There might exist shared references to the HookData created by
    ///    `use_hooks`. These references are always temporary (scoped to the
    ///    `use_hooks` call) and read-only. Deallocation through `reclaim()` is
    ///    only safe when the caller can guarantee no such references exist or
    ///    will be created in the future.
    /// 4. No mutation or deallocation of the pointed-to data will occur until
    ///    `reclaim()` is called.
    hook_data: NonNull<HookData>,
}

impl HooksHandle {
    /// Replaces the currently installed hooks with `self`.
    ///
    /// Returns the previously installed hooks, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::hooks::Hooks;
    ///
    /// // Install first set of hooks, get back None (nothing was installed before)
    /// let old_hooks = Hooks::new().replace();
    /// assert!(old_hooks.is_none());
    ///
    /// // Install second set, get back the first set we installed
    /// let old_hooks = Hooks::new().replace();
    /// assert!(old_hooks.is_some());
    /// # unsafe { old_hooks.unwrap().reclaim(); }
    /// ```
    pub fn replace(self) -> Option<HooksHandle> {
        let Self { hook_data } = self;

        // SAFETY:
        // 1. The `hook_data` pointer is valid and points to a `Box<HookData>` that has
        //    been turned into a raw pointer using `Box::into_raw` as guaranteed by the
        //    struct invariant.
        // 2. The `self` we have just deconstructed had ownership of the `new` pointer,
        //    which we now transfer to the called function.
        // 3. If the function returns `Some(ptr)`, then that pointer will have been
        //    created using `Box::into_raw`, and ownership of it is transferred to us.
        // 4. The returned pointer might still be referenced by this or other threads in
        //    the process of executing `use_hooks`, so it must not be deallocated or
        //    mutated until it is certain that all such function calls have completed.
        //    The struct invariant ensures we will not attempt to do so - the only way
        //    to deallocate is through `reclaim()`, which requires the caller to uphold
        //    the safety contract that no concurrent `use_hooks` calls exist. The
        //    returned `HooksHandle` can be safely stored because it provides exclusive
        //    ownership without allowing mutation until `reclaim()` is called.
        let hook_data = unsafe {
            // @add-unsafe-context: GlobalHooks
            // @add-unsafe-context: HooksHandle
            // @add-unsafe-context: Hooks
            // @add-unsafe-context: HookData
            // @add-unsafe-context: use_hooks
            // @add-unsafe-context: HOOKS
            HOOKS.replace(hook_data)
        };

        let hook_data = hook_data?;
        Some(Self { hook_data })
    }

    /// Reclaims ownership of the leaked hooks, returning them as a `Hooks`
    /// instance.
    ///
    /// **⚠ WARNING: This function is almost impossible to use safely. Do not
    /// call it unless you have global knowledge about the entire execution
    /// state of the program that justifies why it is safe.**
    ///
    /// # Safety
    ///
    /// 1. The caller must guarantee that, if this pointer came from being
    ///    installed globally, then all calls to `use_hooks` that might have
    ///    used this pointer have completed, and that no future calls to
    ///    `use_hooks` will use this pointer.
    pub unsafe fn reclaim(self) -> Hooks {
        // SAFETY:
        // - We know that the pointer is valid and was obtained from `Box::into_raw`
        //   because of the struct invariant.
        // - We know that we have exclusive ownership of the pointer because of the
        //   struct invariant.
        // - The caller has guaranteed that all calls to `use_hooks` that might have
        //   used this pointer have completed. Since this is the only way to access the
        //   pointer, we can safely convert it back into a Box here.
        let boxed = unsafe {
            // @add-unsafe-context: GlobalHooks
            // @add-unsafe-context: HooksHandle
            // @add-unsafe-context: Hooks
            // @add-unsafe-context: HookData
            // @add-unsafe-context: use_hooks
            // @add-unsafe-context: HOOKS
            Box::from_raw(self.hook_data.as_ptr())
        };
        Hooks(boxed)
    }
}

struct GlobalHooks {
    /// # Safety
    ///
    /// 1. This pointer will either be null, or point to a valid HookData that
    ///    has been created using `Box::into_raw`.
    /// 2. If the pointer is non-null, then it is owned by this struct.
    /// 3. All writing to the `AtomicPtr` is done using release semantics.
    /// 4. All reading from the `AtomicPtr` is done using acquire semantics when
    ///    the pointer will be dereferenced and with relaxed semantics
    ///    otherwise.
    /// 5. If the pointer is replaced, then the previous pointer might still be
    ///    referenced by this or other threads in the process of executing
    ///    `use_hooks`, so it must not be deallocated or mutated until it is
    ///    certain that all such function calls have completed.
    ptr: AtomicPtr<HookData>,
}

impl GlobalHooks {
    const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Installs new hooks, returning an error if hooks are already installed.
    ///
    /// # Safety
    ///
    /// 1. The `new` pointer is valid and points to a `Box<HookData>` that has
    ///    been turned into a raw pointer using `Box::into_raw`.
    /// 2. On success the function claims ownership of the `new` pointer, and it
    ///    cannot be used by the caller anymore.
    /// 3. On failure, the `new` pointer remains owned by the caller, and no
    ///    additional references to it are created by this function.
    unsafe fn install(&self, new: *mut HookData) -> Result<(), ()> {
        // Use Release on success to synchronize with Acquire loads in `use_hooks`.
        // Use Relaxed on failure since we don't need synchronization when the
        // operation fails - the caller retains ownership and no sharing occurs.
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
    /// # Safety
    ///
    /// 1. The `new` pointer is valid and points to a `Box<HookData>` that has
    ///    been turned into a raw pointer using `Box::into_raw`.
    /// 2. The function claims ownership of the `new` pointer.
    /// 3. If the function returns `Some(ptr)`, then ownership of that pointer
    ///    is transferred to the caller. The returned pointer is similarly
    ///    guaranteed to have been created using `Box::into_raw`.
    /// 4. The returned pointer might still be referenced by this or other
    ///    threads in the process of executing `use_hooks`, so it must not be
    ///    deallocated or mutated until it is certain that all such function
    ///    calls have completed.
    unsafe fn replace(&self, new: NonNull<HookData>) -> Option<NonNull<HookData>> {
        // Use AcqRel: Release ensures the new HookData is visible to future Acquire
        // loads in `use_hooks`; Acquire ensures we properly observe any previous
        // HookData before returning it (though we don't actually access it here).
        let previous = self.ptr.swap(new.as_ptr(), Ordering::AcqRel);
        NonNull::new(previous)
    }
}

static HOOKS: GlobalHooks = GlobalHooks::new();

/// A trait for calling hook functions with optional `HookData`.
///
/// This is functionally similar to `FnOnce(Option<&HookData>) -> R`, but
/// adds a `#[track_caller]` to allow `report_creation` hooks to track the
/// caller location.
pub(crate) trait HookCallback<R> {
    #[track_caller]
    fn call(self, hook_data: Option<&HookData>) -> R;
}

impl<R, F> HookCallback<R> for F
where
    for<'a> F: FnOnce(Option<&'a HookData>) -> R,
{
    fn call(self, hook_data: Option<&HookData>) -> R {
        self(hook_data)
    }
}

#[track_caller]
pub(crate) fn use_hooks<F, R>(f: F) -> R
where
    F: HookCallback<R>,
{
    let ptr = HOOKS.ptr.load(Ordering::Acquire);

    // SAFETY:
    // 1. The pointer was obtained from `Box::into_raw`, so it is either null or
    //    points to a valid `HookData`.
    // 2. The Acquire load synchronizes with the Release store in `install` and the
    //    AcqRel swap in `replace`, ensuring we see a properly initialized
    //    `HookData`.
    // 3. Even if the pointer is replaced by another thread immediately after we
    //    load it, the old `HookData` remains valid and will not be deallocated. The
    //    only way to deallocate is through `HooksHandle::reclaim()`, which is
    //    `unsafe` and requires the caller to guarantee that all `use_hooks` calls
    //    have completed.
    // 4. We only create a shared reference with a lifetime limited to this
    //    function. We do not mutate or deallocate the data, satisfying GlobalHooks
    //    invariant #5.
    let ptr = unsafe {
        // @add-unsafe-context: GlobalHooks
        // @add-unsafe-context: HooksHandle
        // @add-unsafe-context: Hooks
        // @add-unsafe-context: HookData
        // @add-unsafe-context: use_hooks
        // @add-unsafe-context: HOOKS
        ptr.as_ref()
    };

    f.call(ptr)
}
