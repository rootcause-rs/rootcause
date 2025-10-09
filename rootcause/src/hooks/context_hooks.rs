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
    hooks::hook_lock::HookLock,
    markers::{self, Local, Uncloneable},
    preformatted::Preformatted,
    report::ReportRef,
};

type HookMap = HashMap<TypeId, Arc<dyn UntypedContextHook>>;

static HOOKS: HookLock<HookMap> = HookLock::new();

fn get_hook(type_id: TypeId) -> Option<Arc<dyn UntypedContextHook>> {
    HOOKS.read().get()?.get(&type_id).cloned()
}

struct Hook<C, H>
where
    C: markers::ObjectMarker + ?Sized,
{
    hook: H,
    added_at: &'static Location<'static>,
    _hooked_type: PhantomData<C>,
}

impl<C, H> core::fmt::Display for Hook<C, H>
where
    C: markers::ObjectMarker + ?Sized,
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

unsafe impl<C, H> Send for Hook<C, H>
where
    C: markers::ObjectMarker + ?Sized,
    H: Send + Sync,
{
}
unsafe impl<C, H> Sync for Hook<C, H>
where
    C: markers::ObjectMarker + ?Sized,
    H: Send + Sync,
{
}

pub(crate) trait UntypedContextHook: 'static + Send + Sync + core::fmt::Display {
    unsafe fn display(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    unsafe fn debug(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn display_preformatted(
        &self,
        report: ReportRef<'_, Preformatted, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn debug_preformatted(
        &self,
        report: ReportRef<'_, Preformatted, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this context will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    /// - `report_formatting_alternate`: Whether the report in which this context will be embedded is being formatted using the [`alternate`] mode
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`alternate`]: core::fmt::Formatter::alternate
    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
        report_formatting_alternate: bool,
    ) -> ContextFormattingStyle;
}

pub trait ContextHook<C>: 'static + Send + Sync
where
    C: markers::ObjectMarker + ?Sized,
{
    fn display(
        &self,
        report: ReportRef<'_, C, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
    }

    fn display_preformatted(
        &self,
        report: ReportRef<'_, Preformatted, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Display::fmt(&report.format_current_context_unhooked(), formatter)
    }

    fn debug(
        &self,
        report: ReportRef<'_, C, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Debug::fmt(&report.format_current_context_unhooked(), formatter)
    }

    fn debug_preformatted(
        &self,
        report: ReportRef<'_, Preformatted, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        fmt::Debug::fmt(&report.format_current_context_unhooked(), formatter)
    }

    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this context will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    /// - `report_formatting_alternate`: Whether the report in which this context will be embedded is being formatted using the [`alternate`] mode
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    /// [`alternate`]: core::fmt::Formatter::alternate
    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
        report_formatting_alternate: bool,
    ) -> ContextFormattingStyle {
        report.preferred_context_formatting_style_unhooked(
            report_formatting_function,
            report_formatting_alternate,
        )
    }
}

impl<C, H> UntypedContextHook for Hook<C, H>
where
    C: markers::ObjectMarker + ?Sized,
    H: ContextHook<C>,
{
    unsafe fn display(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let report = unsafe { report.downcast_report_unchecked::<C>() };
        self.hook.display(report, formatter)
    }

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
        report: ReportRef<'_, Preformatted, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.hook.display_preformatted(report, formatter)
    }

    fn debug_preformatted(
        &self,
        report: ReportRef<'_, Preformatted, Uncloneable, Local>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.hook.debug_preformatted(report, formatter)
    }

    fn preferred_context_formatting_style(
        &self,
        report: ReportRef<'_, dyn Any, Uncloneable, Local>,
        report_formatting_function: FormattingFunction,
        report_formatting_alternate: bool,
    ) -> ContextFormattingStyle {
        self.hook.preferred_context_formatting_style(
            report,
            report_formatting_function,
            report_formatting_alternate,
        )
    }
}

#[track_caller]
pub fn register_context_hook<C, H>(hook: H)
where
    C: markers::ObjectMarker + ?Sized,
    H: ContextHook<C> + Send + Sync + 'static,
{
    let added_location = Location::caller();
    let hook: Hook<C, H> = Hook {
        hook,
        added_at: added_location,
        _hooked_type: PhantomData,
    };
    let hook: Arc<Hook<C, H>> = Arc::new(hook);
    let hook = hook.unsize(unsize::Coercion!(to dyn UntypedContextHook));

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
    if let Some(report) = report.downcast_report::<Preformatted>()
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
    if let Some(report) = report.downcast_report::<Preformatted>()
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
/// - `report_formatting_function`: Whether the report in which this context will be embedded is being formatted using [`Display`] formatting or [`Debug`]
/// - `report_formatting_alternate`: Whether the report in which this context will be embedded is being formatted using the [`alternate`] mode
///
/// [`Display`]: core::fmt::Display
/// [`Debug`]: core::fmt::Debug
/// [`alternate`]: core::fmt::Formatter::alternate
pub(crate) fn get_preferred_context_formatting_style(
    report: ReportRef<'_, dyn Any, Uncloneable, Local>,
    report_formatting_function: FormattingFunction,
    report_formatting_alternate: bool,
) -> ContextFormattingStyle {
    if let Some(current_context) = report.downcast_current_context::<Preformatted>()
        && let Some(hook) = get_hook(current_context.original_type_id())
    {
        hook.preferred_context_formatting_style(
            report,
            report_formatting_function,
            report_formatting_alternate,
        )
    } else if let Some(hook) = get_hook(report.current_context_type_id()) {
        hook.preferred_context_formatting_style(
            report,
            report_formatting_function,
            report_formatting_alternate,
        )
    } else {
        report.preferred_context_formatting_style_unhooked(
            report_formatting_function,
            report_formatting_alternate,
        )
    }
}

pub fn debug_context_hooks(mut f: impl FnMut(&dyn core::fmt::Display)) {
    if let Some(hooks) = HOOKS.read().get() {
        for hook in hooks.values() {
            f(hook);
        }
    }
}
