//! Context formatting override system for customizing how error report contexts
//! are displayed.
//!
//! This module provides a hook system that allows customization of how contexts
//! (the main error types) are formatted in error reports. By installing hooks
//! for specific context types, you can override the default Display and Debug
//! formatting behavior to provide more detailed, context-aware, or
//! domain-specific formatting.
//!
//! # Key Components
//!
//! - [`ContextFormattingOverride`] - Trait for implementing custom context
//!   formatting
//!
//! # Example
//!
//! ```rust
//! use core::fmt;
//!
//! use rootcause::{
//!     ReportRef,
//!     hooks::{Hooks, formatting_overrides::context::ContextFormattingOverride},
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
//! // Install the custom formatter globally
//! Hooks::new()
//!     .with_context_override::<DatabaseError, _>(DatabaseErrorFormatter)
//!     .install()
//!     .expect("failed to install hooks");
//! ```

use alloc::{boxed::Box, fmt};
use core::{any::TypeId, marker::PhantomData};

use hashbrown::HashMap;
use rootcause_internals::handlers::{ContextFormattingStyle, FormattingFunction};

use crate::{
    ReportRef,
    hooks::HookData,
    markers::{Dynamic, Local, Uncloneable},
    preformatted::PreformattedContext,
};

#[derive(Default)]
pub(crate) struct HookMap {
    map: HashMap<TypeId, Box<dyn UntypedContextFormattingOverride>, rustc_hash::FxBuildHasher>,
}

impl core::fmt::Debug for HookMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.map.values().fmt(f)
    }
}

impl HookMap {
    /// Retrieves the formatting override hook for the specified attachment
    /// type.
    ///
    /// The returned hook is guaranteed to be an instance of type `Hook<C, H>`,
    /// where `TypeId::of::<C>() == type_id`.
    fn get(&self, type_id: TypeId) -> Option<&Box<dyn UntypedContextFormattingOverride>> {
        self.map.get(&type_id)
    }

    pub(crate) fn insert<C, H>(&mut self, hook: H)
    where
        C: 'static,
        H: ContextFormattingOverride<C>,
    {
        let hook: Hook<C, H> = Hook {
            hook,
            _hooked_type: PhantomData,
        };
        let hook: Box<Hook<C, H>> = Box::new(hook);
        // We must uphold the safety invariant of HookMap.
        //
        // The safety invariant requires that the hook stored under
        // `TypeId::of::<C>()` is always of type `Hook<C, H>`.
        //
        // However this is exactly what we are doing here,
        // so the invariant is upheld.
        self.map.insert(TypeId::of::<C>(), hook);
    }

    pub(crate) fn display_context(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if let Some(report) = report.downcast_report::<PreformattedContext>()
            && let Some(hook) = self.get(report.current_context().original_type_id())
        {
            hook.display_preformatted(report, formatter)
        } else if let Some(hook) = self.get(report.current_context_type_id()) {
            // SAFETY:
            // 1. The call to `get_hook` guarantees that the returned hook is of type
            //    `Hook<C, H>`, and `TypeId::of<C>() == report.current_context_type_id()`.
            //    Therefore the type `C` stored in the context matches the `C` from type
            //    `Hook<C, H>`.
            unsafe {
                // @add-unsafe-context: UntypedContextFormattingOverride
                hook.display(report, formatter)
            }
        } else {
            fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
        }
    }

    pub(crate) fn debug_context(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if let Some(report) = report.downcast_report::<PreformattedContext>()
            && let Some(hook) = self.get(report.current_context().original_type_id())
        {
            hook.debug_preformatted(report, formatter)
        } else if let Some(hook) = self.get(report.current_context_type_id()) {
            // SAFETY:
            // 1. The call to `get` guarantees that the returned hook is of type `Hook<C,
            //    H>`, and `TypeId::of<C>() == report.current_context_type_id()`. Therefore
            //    the type `C` stored in the context matches the `C` from type `Hook<C, H>`.
            unsafe {
                // @add-unsafe-context: UntypedContextFormattingOverride
                hook.debug(report, formatter)
            }
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
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        if let Some(current_context) = report.downcast_current_context::<PreformattedContext>()
            && let Some(hook) = self.get(current_context.original_type_id())
        {
            hook.preferred_context_formatting_style(report, report_formatting_function)
        } else if let Some(hook) = self.get(report.current_context_type_id()) {
            hook.preferred_context_formatting_style(report, report_formatting_function)
        } else {
            report.preferred_context_formatting_style_unhooked(report_formatting_function)
        }
    }
}

struct Hook<C, H>
where
    C: 'static,
{
    hook: H,
    _hooked_type: PhantomData<fn(C) -> C>,
}

impl<C, H> core::fmt::Debug for Hook<C, H>
where
    C: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ContextFormattingHook<{}, {}>",
            core::any::type_name::<C>(),
            core::any::type_name::<H>(),
        )
    }
}

/// Trait for untyped context formatting overrides.
///
/// This trait is guaranteed to only be implemented for [`Hook<C, H>`].
pub(crate) trait UntypedContextFormattingOverride:
    'static + Send + Sync + core::fmt::Debug
{
    /// Formats the context using Display formatting.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `C` stored in the context matches the `C` from type `Hook<C,
    ///    H>` this is implemented for.
    unsafe fn display(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// Formats the context using Debug formatting.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `C` stored in the context matches the `C` from type `Hook<C,
    ///    H>` this is implemented for.
    unsafe fn debug(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
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
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
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
pub trait ContextFormattingOverride<C>: 'static + Send + Sync {
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
    /// using [`ReportRef::preformat`] for performance or consistency reasons).
    /// The default implementation delegates to the context's unhooked
    /// Display formatting.
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
    /// * `report` - Reference to the report (as [`Dynamic`] as it can be either
    ///   a `C` or a [`PreformattedContext`])
    /// * `report_formatting_function` - Whether the overall report uses Display
    ///   or Debug formatting
    ///
    /// # Returns
    ///
    /// A `ContextFormattingStyle` that specifies the preferred formatting
    /// approach
    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
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
    unsafe fn display(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // SAFETY:
        // 1. Guaranteed by the caller
        let report = unsafe { report.downcast_report_unchecked::<C>() };
        self.hook.display(report, formatter)
    }

    unsafe fn debug(
        &self,
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // SAFETY:
        // 1. Guaranteed by the caller
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
        report: ReportRef<'_, Dynamic, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        self.hook
            .preferred_context_formatting_style(report, report_formatting_function)
    }
}

pub(crate) fn display_context(
    report: ReportRef<'_, Dynamic, Uncloneable, Local>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(hook_data) = HookData::fetch() {
        hook_data
            .context_formatting_overrides
            .display_context(report, formatter)
    } else {
        fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
    }
}

pub(crate) fn debug_context(
    report: ReportRef<'_, Dynamic, Uncloneable, Local>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(hook_data) = HookData::fetch() {
        hook_data
            .context_formatting_overrides
            .debug_context(report, formatter)
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
    report: ReportRef<'_, Dynamic, Uncloneable, Local>,
    report_formatting_function: FormattingFunction,
) -> ContextFormattingStyle {
    if let Some(hook_data) = HookData::fetch() {
        hook_data
            .context_formatting_overrides
            .get_preferred_context_formatting_style(report, report_formatting_function)
    } else {
        report.preferred_context_formatting_style_unhooked(report_formatting_function)
    }
}
