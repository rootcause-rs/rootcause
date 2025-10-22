use alloc::fmt;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
    panic::Location,
};

use hashbrown::HashMap;
use rootcause_internals::handlers::{AttachmentFormattingStyle, FormattingFunction};
use triomphe::Arc;
use unsize::CoerceUnsize;

use crate::{
    ReportRef,
    hooks::hook_lock::HookLock,
    markers::{self, Local, Uncloneable},
    preformatted::PreformattedAttachment,
    report_attachment::ReportAttachmentRef,
};

type HookMap = HashMap<TypeId, Arc<dyn UntypedAttachmentHook>, rustc_hash::FxBuildHasher>;

static HOOKS: HookLock<HookMap> = HookLock::new();

fn get_hook(type_id: TypeId) -> Option<Arc<dyn UntypedAttachmentHook>> {
    HOOKS.read().get()?.get(&type_id).cloned()
}

struct Hook<A, H>
where
    A: markers::ObjectMarker + ?Sized,
{
    hook: H,
    added_at: &'static Location<'static>,
    _hooked_type: PhantomData<A>,
}

impl<A, H> core::fmt::Display for Hook<A, H>
where
    A: markers::ObjectMarker + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Attachment hook {} for attachment type {} registered at {}:{}",
            core::any::type_name::<H>(),
            core::any::type_name::<A>(),
            self.added_at.file(),
            self.added_at.line()
        )
    }
}

unsafe impl<A, H> Send for Hook<A, H>
where
    A: markers::ObjectMarker + ?Sized,
    H: Send + Sync,
{
}
unsafe impl<A, H> Sync for Hook<A, H>
where
    A: markers::ObjectMarker + ?Sized,
    H: Send + Sync,
{
}

#[derive(Copy, Clone, Debug)]
pub struct AttachmentParent<'a> {
    pub report: ReportRef<'a, dyn Any, Uncloneable, Local>,
    pub attachment_index: usize,
}

pub(crate) trait UntypedAttachmentHook: 'static + Send + Sync + core::fmt::Display {
    unsafe fn display(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    unsafe fn debug(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn display_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn debug_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this attachment will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle;
}

pub trait AttachmentHook<A>: 'static + Send + Sync
where
    A: markers::ObjectMarker + ?Sized,
{
    fn display(
        &self,
        attachment: ReportAttachmentRef<'_, A>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Display::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    fn display_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Display::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    fn debug(
        &self,
        attachment: ReportAttachmentRef<'_, A>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Debug::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    fn debug_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Debug::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this attachment will be embedded is being formatted using [`Display`] formatting or [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        attachment.preferred_formatting_style_unhooked(report_formatting_function)
    }
}

impl<A, H> UntypedAttachmentHook for Hook<A, H>
where
    A: markers::ObjectMarker + ?Sized,
    H: AttachmentHook<A>,
{
    unsafe fn display(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let attachment = unsafe { attachment.downcast_attachment_unchecked::<A>() };
        self.hook.display(attachment, attachment_parent, formatter)
    }

    unsafe fn debug(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let attachment = unsafe { attachment.downcast_attachment_unchecked::<A>() };
        self.hook.debug(attachment, attachment_parent, formatter)
    }

    fn display_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.hook
            .display_preformatted(attachment, attachment_parent, formatter)
    }

    fn debug_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.hook
            .debug_preformatted(attachment, attachment_parent, formatter)
    }

    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.hook
            .preferred_formatting_style(attachment, report_formatting_function)
    }
}

#[track_caller]
pub fn register_attachment_hook<A, H>(hook: H)
where
    A: 'static,
    H: AttachmentHook<A> + Send + Sync + 'static,
{
    let added_location = Location::caller();
    let hook: Hook<A, H> = Hook {
        hook,
        added_at: added_location,
        _hooked_type: PhantomData,
    };
    let hook: Arc<Hook<A, H>> = Arc::new(hook);
    let hook = hook.unsize(unsize::Coercion!(to dyn UntypedAttachmentHook));

    HOOKS
        .write()
        .get()
        .get_or_insert_default()
        .insert(TypeId::of::<A>(), hook);
}

pub(crate) fn display_attachment(
    attachment: ReportAttachmentRef<'_, dyn Any>,
    attachment_parent: Option<AttachmentParent<'_>>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(attachment) = attachment.downcast_attachment::<PreformattedAttachment>()
        && let Some(hook) = get_hook(attachment.inner().original_type_id())
    {
        hook.display_preformatted(attachment, attachment_parent, formatter)
    } else if let Some(hook) = get_hook(attachment.inner_type_id()) {
        unsafe { hook.display(attachment, attachment_parent, formatter) }
    } else {
        fmt::Display::fmt(&attachment.format_inner_unhooked(), formatter)
    }
}

pub(crate) fn debug_attachment(
    attachment: ReportAttachmentRef<'_, dyn Any>,
    attachment_parent: Option<AttachmentParent<'_>>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(attachment) = attachment.downcast_attachment::<PreformattedAttachment>()
        && let Some(hook) = get_hook(attachment.inner().original_type_id())
    {
        hook.debug_preformatted(attachment, attachment_parent, formatter)
    } else if let Some(hook) = get_hook(attachment.inner_type_id()) {
        unsafe { hook.debug(attachment, attachment_parent, formatter) }
    } else {
        fmt::Debug::fmt(&attachment.format_inner_unhooked(), formatter)
    }
}

/// # Arguments
///
/// - `report_formatting_function`: Whether the report in which this attachment will be embedded is being formatted using [`Display`] formatting or [`Debug`]
///
/// [`Display`]: core::fmt::Display
/// [`Debug`]: core::fmt::Debug
pub(crate) fn get_preferred_formatting_style(
    attachment: ReportAttachmentRef<'_, dyn Any>,
    report_formatting_function: FormattingFunction,
) -> AttachmentFormattingStyle {
    if let Some(inner) = attachment.downcast_inner::<PreformattedAttachment>()
        && let Some(hook) = get_hook(inner.original_type_id())
    {
        hook.preferred_formatting_style(attachment, report_formatting_function)
    } else if let Some(hook) = get_hook(attachment.inner_type_id()) {
        hook.preferred_formatting_style(attachment, report_formatting_function)
    } else {
        attachment.preferred_formatting_style_unhooked(report_formatting_function)
    }
}

pub fn debug_attachment_hooks(mut f: impl FnMut(&dyn core::fmt::Display)) {
    if let Some(hooks) = HOOKS.read().get() {
        for hook in hooks.values() {
            f(hook);
        }
    }
}
