//! Context formatting override system for customizing how error report contexts
//! are displayed.
//!
//! This module provides a hook system that allows customization of how contexts
//! (the main error types) are formatted in error reports. By registering hooks
//! for specific context types, you can override the default Display and Debug
//! formatting behavior to provide more detailed, context-aware, or
//! domain-specific formatting.
//!
//! # Key Components
//!
//! - [`ContextFormattingOverride`] - Trait for implementing custom context
//!   formatting
//! - [`register_context_hook`] - Function to register formatting overrides for
//!   specific types
//! - [`debug_context_hooks`] - Utility to inspect registered hooks
//!
//! # Example
//!
//! ```rust
//! use core::fmt;
//!
//! use rootcause::{
//!     ReportRef,
//!     hooks::formatting_overrides::context::{ContextFormattingOverride, register_context_hook},
//!     markers::{Local, Uncloneable},
//! };
//!
//! struct DatabaseError {
//!     table: String,
//!     operation: String,
//!     details: String,
//! }
//!
//! struct DatabaseErrorFormatter;
//!
//! impl ContextFormattingOverride<DatabaseError> for DatabaseErrorFormatter {
//!     fn display(
//!         &self,
//!         report: ReportRef<'_, DatabaseError, Uncloneable, Local>,
//!         f: &mut fmt::Formatter<'_>,
//!     ) -> fmt::Result {
//!         let err = report.current_context();
//!         write!(
//!             f,
//!             "Database {} failed on table '{}': {}",
//!             err.operation, err.table, err.details
//!         )
//!     }
//! }
//!
//! // Register the custom formatter
//! register_context_hook::<DatabaseError, _>(DatabaseErrorFormatter);
//! ```

use alloc::fmt;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
    panic::Location,
};

use hashbrown::HashMap;
use rootcause_internals::handlers::{ContextFormattingStyle, FormattingFunction};
use triomphe::Arc;
use unsize::CoerceUnsize;

use crate::{
    ReportRef,
    hooks::hook_lock::HookLock,
    markers::{self, Local, Uncloneable},
    preformatted::PreformattedContext,
};

type HookMap =
    HashMap<TypeId, Arc<dyn UntypedContextFormattingOverride>, rustc_hash::FxBuildHasher>;

static HOOKS: HookLock<HookMap> = HookLock::new();

fn get_hook(type_id: TypeId) -> Option<Arc<dyn UntypedContextFormattingOverride>> {
    HOOKS.read().get()?.get(&type_id).cloned()
}

struct Hook<C, H>
where
    C: 'static,
{
    hook: H,
    added_at: &'static Location<'static>,
    _hooked_type: PhantomData<fn(C) -> C>,
}

impl<C, H> core::fmt::Display for Hook<C, H>
where
    C: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Context hook {} for context type {} registered at {}:{}",
            core::any::type_name::<H>(),
            core::any::type_name::<C>(),
            self.added_at.file(),
            self.added_at.line()
        )
    }
}

trait UntypedContextFormattingOverride: 'static + Send + Sync + core::fmt::Display {
    /// Formats the context using Display formatting.
    ///
    /// # Safety
    ///
    /// The implementation of this trait is free to make assumptions about the
    /// type of the context contained in the report and call
    /// [`ReportRef::downcast_report_unchecked`]. It is the responsibility
    /// of the caller to ensure that whatever those assumptions might be for
    /// the type in question, they hold for the report given as the argument.
    unsafe fn display(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// Formats the context using Debug formatting.
    ///
    /// # Safety
    ///
    /// The implementation of this trait is free to make assumptions about the
    /// type of the context contained in the report and call
    /// [`ReportRef::downcast_report_unchecked`]. It is the responsibility
    /// of the caller to ensure that whatever those assumptions might be for
    /// the type in question, they hold for the report given as the argument.
    unsafe fn debug(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn display_preformatted(
        &self,
        report: ReportRef<'_, PreformattedContext, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn debug_preformatted(
        &self,
        report: ReportRef<'_, PreformattedContext, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle;
}

/// Trait for customizing how contexts of a specific type are formatted in error
/// reports.
///
/// This trait allows you to override the default formatting behavior for
/// contexts (the main error types) of type `C`. You can customize both Display
/// and Debug formatting, handle preformatted contexts, and specify preferred
/// formatting styles.
///
/// # Type Parameters
///
/// * `C` - The context type that this formatter handles
///
/// # Default Implementations
///
/// All methods have default implementations that delegate to the unhooked
/// formatting, so you only need to override the methods for the formatting you
/// want to customize.
///
/// # Examples
///
/// Custom Display formatting for a business logic error:
/// ```rust
/// use core::fmt;
///
/// use rootcause::{
///     ReportRef,
///     hooks::formatting_overrides::context::ContextFormattingOverride,
///     markers::{Local, Uncloneable},
/// };
///
/// struct ValidationError {
///     field: String,
///     rule: String,
///     value: String,
/// }
///
/// struct ValidationErrorFormatter;
///
/// impl ContextFormattingOverride<ValidationError> for ValidationErrorFormatter {
///     fn display(
///         &self,
///         report: ReportRef<'_, ValidationError, Uncloneable, Local>,
///         f: &mut fmt::Formatter<'_>,
///     ) -> fmt::Result {
///         let err = report.current_context();
///         write!(
///             f,
///             "Validation failed for field '{}': value '{}' violates rule '{}'",
///             err.field, err.value, err.rule
///         )
///     }
/// }
/// ```
pub trait ContextFormattingOverride<C>: 'static + Send + Sync
where
    C: markers::ObjectMarker,
{
    /// Formats the context using Display formatting.
    ///
    /// This method is called when the context needs to be displayed in a
    /// user-friendly format. The default implementation delegates to the
    /// context's unhooked Display formatting.
    ///
    /// # Arguments
    ///
    /// * `report` - Reference to the report containing the context to format
    /// * `formatter` - The formatter to write output to
    ///
    /// # Examples
    ///
    /// ```rust
    /// use core::fmt;
    ///
    /// use rootcause::{
    ///     ReportRef,
    ///     hooks::formatting_overrides::context::ContextFormattingOverride,
    ///     markers::{Local, Uncloneable},
    /// };
    ///
    /// struct HttpError {
    ///     status: u16,
    ///     message: String,
    /// }
    ///
    /// struct HttpErrorFormatter;
    ///
    /// impl ContextFormattingOverride<HttpError> for HttpErrorFormatter {
    ///     fn display(
    ///         &self,
    ///         report: ReportRef<'_, HttpError, Uncloneable, Local>,
    ///         f: &mut fmt::Formatter<'_>,
    ///     ) -> fmt::Result {
    ///         let err = report.current_context();
    ///         write!(f, "HTTP {} - {}", err.status, err.message)
    ///     }
    /// }
    /// ```
    fn display(
        &self,
        report: ReportRef<'_, C, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
    }

    /// Formats a preformatted context using Display formatting.
    ///
    /// This method handles contexts that have been preformatted (typically done
    /// using [`ReportRef::preformat`]performance or consistency reasons). The
    /// default implementation delegates to the context's unhooked Display
    /// formatting.
    ///
    /// # Arguments
    ///
    /// * `report` - Reference to the report containing the preformatted context
    /// * `formatter` - The formatter to write output to
    fn display_preformatted(
        &self,
        report: ReportRef<'_, PreformattedContext, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
    }

    /// Formats the context using Debug formatting.
    ///
    /// This method is called when the context needs to be displayed in a
    /// debug-friendly format (typically more verbose and detailed). The default
    /// implementation delegates to the context's unhooked Debug formatting.
    ///
    /// # Arguments
    ///
    /// * `report` - Reference to the report containing the context to format
    /// * `formatter` - The formatter to write output to
    fn debug(
        &self,
        report: ReportRef<'_, C, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Debug::fmt(&report.format_current_context_unhooked(), formatter)
    }

    /// Formats a preformatted context using Debug formatting.
    ///
    /// This method handles preformatted contexts when debug formatting is
    /// requested. The default implementation delegates to the context's
    /// unhooked Debug formatting.
    ///
    /// # Arguments
    ///
    /// * `report` - Reference to the report containing the preformatted context
    /// * `formatter` - The formatter to write output to
    fn debug_preformatted(
        &self,
        report: ReportRef<'_, PreformattedContext, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Debug::fmt(&report.format_current_context_unhooked(), formatter)
    }

    /// Determines the preferred formatting style for this context.
    ///
    /// This method allows the formatter to specify how the context should be
    /// presented, including which formatting function should be used. The
    /// default implementation delegates to the context's unhooked
    /// preference.
    ///
    /// # Arguments
    ///
    /// * `report` - Reference to the report (as `dyn Any` as it can be either a
    ///   `C` or a [`PreformattedContext`])
    /// * `report_formatting_function` - Whether the overall report uses Display
    ///   or Debug formatting
    ///
    /// # Returns
    ///
    /// A `ContextFormattingStyle` that specifies the preferred formatting
    /// approach
    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        report.preferred_context_formatting_style_unhooked(report_formatting_function)
    }
}

impl<C, H> UntypedContextFormattingOverride for Hook<C, H>
where
    C: 'static,
    H: ContextFormattingOverride<C>,
{
    /// Formats the context using Display formatting.
    ///
    /// # Safety
    ///
    /// As specified in the trait, the implementer can make assumptions about
    /// the type of the context contained in the report.
    ///
    /// This implementation will downcast the report to the expected type `C`,
    /// so the caller must ensure that the report indeed contains a context
    /// of type `C`.
    unsafe fn display(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let report = unsafe { report.downcast_report_unchecked::<C>() };
        self.hook.display(report, formatter)
    }

    /// Formats the context using Debug formatting.
    ///
    /// # Safety
    ///
    /// As specified in the trait, the implementer can make assumptions about
    /// the type of the context contained in the report.
    ///
    /// This implementation will downcast the report to the expected type `C`,
    /// so the caller must ensure that the report indeed contains a context
    /// of type `C`.
    unsafe fn debug(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let report = unsafe { report.downcast_report_unchecked::<C>() };
        self.hook.debug(report, formatter)
    }

    fn display_preformatted(
        &self,
        report: ReportRef<'_, PreformattedContext, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.hook.display_preformatted(report, formatter)
    }

    fn debug_preformatted(
        &self,
        report: ReportRef<'_, PreformattedContext, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.hook.debug_preformatted(report, formatter)
    }

    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        self.hook
            .preferred_context_formatting_style(report, report_formatting_function)
    }
}

/// Registers a formatting override hook for contexts of type `C`.
///
/// This function allows you to customize how contexts (main error types) of a
/// specific type are formatted in error reports. Once registered, the hook will
/// be called whenever a context of type `C` needs to be formatted.
///
/// The registration includes location tracking for debugging purposes, so you
/// can identify where hooks were registered using [`debug_context_hooks`].
///
/// # Type Parameters
///
/// * `C` - The type of context this hook will handle
/// * `H` - The type of the formatting override hook
///
/// # Arguments
///
/// * `hook` - An implementation of [`ContextFormattingOverride<C>`]
///
/// # Examples
///
/// ```rust
/// use core::fmt;
///
/// use rootcause::{
///     ReportRef,
///     hooks::formatting_overrides::context::{ContextFormattingOverride, register_context_hook},
///     markers::{Local, Uncloneable},
/// };
///
/// struct FileSystemError {
///     path: String,
///     operation: String,
///     error_code: i32,
/// }
///
/// struct FileSystemErrorFormatter;
///
/// impl ContextFormattingOverride<FileSystemError> for FileSystemErrorFormatter {
///     fn display(
///         &self,
///         report: ReportRef<'_, FileSystemError, Uncloneable, Local>,
///         f: &mut fmt::Formatter<'_>,
///     ) -> fmt::Result {
///         let err = report.current_context();
///         write!(
///             f,
///             "File system error during {} on '{}' (code: {})",
///             err.operation, err.path, err.error_code
///         )
///     }
/// }
///
/// register_context_hook::<FileSystemError, _>(FileSystemErrorFormatter);
/// ```
#[track_caller]
pub fn register_context_hook<C, H>(hook: H)
where
    C: markers::ObjectMarker,
    H: ContextFormattingOverride<C> + Send + Sync + 'static,
{
    let added_location = Location::caller();
    let hook: Hook<C, H> = Hook {
        hook,
        added_at: added_location,
        _hooked_type: PhantomData,
    };
    let hook: Arc<Hook<C, H>> = Arc::new(hook);
    let hook = hook.unsize(unsize::Coercion!(to dyn UntypedContextFormattingOverride));

    HOOKS
        .write()
        .get()
        .get_or_insert_default()
        .insert(TypeId::of::<C>(), hook);
}

pub(crate) fn display_context(
    report: ReportRef<'_, dyn Any, Uncloneable, Local>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(report) = report.downcast_report::<PreformattedContext>()
        && let Some(hook) = get_hook(report.current_context().original_type_id())
    {
        hook.display_preformatted(report, formatter)
    } else if let Some(hook) = get_hook(report.current_context_type_id()) {
        unsafe { hook.display(report, formatter) }
    } else {
        fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
    }
}

pub(crate) fn debug_context(
    report: ReportRef<'_, dyn Any, Uncloneable, Local>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(report) = report.downcast_report::<PreformattedContext>()
        && let Some(hook) = get_hook(report.current_context().original_type_id())
    {
        hook.debug_preformatted(report, formatter)
    } else if let Some(hook) = get_hook(report.current_context_type_id()) {
        unsafe { hook.debug(report, formatter) }
    } else {
        fmt::Debug::fmt(&report.format_current_context_unhooked(), formatter)
    }
}

/// # Arguments
///
/// - `report_formatting_function`: Whether the report in which this context
///   will be embedded is being formatted using [`Display`] formatting or
///   [`Debug`]
///
/// [`Display`]: core::fmt::Display
/// [`Debug`]: core::fmt::Debug
pub(crate) fn get_preferred_context_formatting_style(
    report: ReportRef<'_, dyn Any, Uncloneable, Local>,
    report_formatting_function: FormattingFunction,
) -> ContextFormattingStyle {
    if let Some(current_context) = report.downcast_current_context::<PreformattedContext>()
        && let Some(hook) = get_hook(current_context.original_type_id())
    {
        hook.preferred_context_formatting_style(report, report_formatting_function)
    } else if let Some(hook) = get_hook(report.current_context_type_id()) {
        hook.preferred_context_formatting_style(report, report_formatting_function)
    } else {
        report.preferred_context_formatting_style_unhooked(report_formatting_function)
    }
}

/// Calls a function for each registered context formatting hook for debugging
/// purposes.
///
/// This utility function allows you to inspect all currently registered context
/// formatting hooks. Each hook provides information about the hook type, the
/// context type it handles, and where it was registered.
///
/// # Arguments
///
/// * `f` - A function that will be called once for each registered hook with a
///   displayable representation of the hook information
///
/// # Warning
///
/// This function will lock the internal hook registry for reading, so it can
/// potentially cause deadlocks if [`register_context_hook`] is called while the
/// function is executing.
///
/// # Examples
///
/// ```rust
/// use rootcause::hooks::formatting_overrides::context::debug_context_hooks;
///
/// // Print all registered context hooks
/// debug_context_hooks(|hook| {
///     println!("Registered hook: {}", hook);
/// });
/// ```
pub fn debug_context_hooks(mut f: impl FnMut(&dyn core::fmt::Display)) {
    if let Some(hooks) = HOOKS.read().get() {
        for hook in hooks.values() {
            f(hook);
        }
    }
}
