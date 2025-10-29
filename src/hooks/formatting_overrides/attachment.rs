//! Attachment formatting override system for customizing how error report attachments are displayed.
//!
//! This module provides a hook system that allows customization of how attachments are formatted
//! in error reports. By registering hooks for specific attachment types, you can override the
//! default Display and Debug formatting behavior to provide more context-aware or specialized
//! formatting.
//!
//! # Key Components
//!
//! - [`AttachmentFormattingOverride`] - Trait for implementing custom attachment formatting
//! - [`AttachmentParent`] - Context about the report containing an attachment
//! - [`register_attachment_hook`] - Function to register formatting overrides for specific types
//! - [`debug_attachment_hooks`] - Utility to inspect registered hooks
//!
//! # Example
//!
//! ```rust
//! use rootcause::hooks::formatting_overrides::{
//!     AttachmentFormattingOverride, AttachmentParent, register_attachment_hook
//! };
//! use rootcause::report_attachment::ReportAttachmentRef;
//! use core::fmt;
//!
//! struct MyError(String);
//!
//! struct MyErrorFormatter;
//!
//! impl AttachmentFormattingOverride<MyError> for MyErrorFormatter {
//!     fn display(
//!         &self,
//!         attachment: ReportAttachmentRef<'_, MyError>,
//!         _parent: Option<AttachmentParent<'_>>,
//!         f: &mut fmt::Formatter<'_>
//!     ) -> fmt::Result {
//!         write!(f, "Custom format: {}", attachment.inner().0)
//!     }
//! }
//!
//! // Register the custom formatter
//! register_attachment_hook(MyErrorFormatter);
//! ```

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

type HookMap =
    HashMap<TypeId, Arc<dyn UntypedAttachmentFormattingOverride>, rustc_hash::FxBuildHasher>;

static HOOKS: HookLock<HookMap> = HookLock::new();

fn get_hook(type_id: TypeId) -> Option<Arc<dyn UntypedAttachmentFormattingOverride>> {
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

/// Information about the parent report that contains an attachment being formatted.
///
/// This struct provides context about where an attachment exists within the report hierarchy,
/// which can be useful for custom formatting logic that needs to understand the attachment's
/// position or relationship to its containing report.
///
/// # Examples
///
/// ```rust
/// use rootcause::hooks::formatting_overrides::{
///     AttachmentFormattingOverride, AttachmentParent
/// };
/// use rootcause::report_attachment::ReportAttachmentRef;
/// use core::fmt;
///
/// struct MyFormatter;
///
/// impl AttachmentFormattingOverride<String> for MyFormatter {
///     fn display(
///         &self,
///         attachment: ReportAttachmentRef<'_, String>,
///         parent: Option<AttachmentParent<'_>>,
///         f: &mut fmt::Formatter<'_>
///     ) -> fmt::Result {
///         if let Some(parent) = parent {
///             write!(f, "[Attachment {}] {}", parent.attachment_index, attachment.inner())
///         } else {
///             write!(f, "{}", attachment.inner())
///         }
///     }
/// }
/// ```
#[derive(Copy, Clone, Debug)]
pub struct AttachmentParent<'a> {
    /// Reference to the report that contains this attachment
    pub report: ReportRef<'a, dyn Any, Uncloneable, Local>,
    /// Index of this attachment within the parent report's attachment list
    pub attachment_index: usize,
}

pub(crate) trait UntypedAttachmentFormattingOverride:
    'static + Send + Sync + core::fmt::Display
{
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

    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle;
}

/// Trait for customizing how attachments of a specific type are formatted in error reports.
///
/// This trait allows you to override the default formatting behavior for attachments
/// of type `A`. You can customize both Display and Debug formatting, as well as handle
/// preformatted attachments and specify preferred formatting styles.
///
/// # Type Parameters
///
/// * `A` - The attachment type that this formatter handles
///
/// # Default Implementations
///
/// All methods have default implementations that delegate to the unhooked formatting,
/// so you only need to override the methods for the formatting you want to customize.
///
/// # Examples
///
/// Basic custom Display formatting:
/// ```rust
/// use rootcause::hooks::formatting_overrides::{
///     AttachmentFormattingOverride, AttachmentParent
/// };
/// use rootcause::report_attachment::ReportAttachmentRef;
/// use core::fmt;
///
/// struct MyData {
///     value: i32,
///     name: String,
/// }
///
/// struct MyDataFormatter;
///
/// impl AttachmentFormattingOverride<MyData> for MyDataFormatter {
///     fn display(
///         &self,
///         attachment: ReportAttachmentRef<'_, MyData>,
///         _parent: Option<AttachmentParent<'_>>,
///         f: &mut fmt::Formatter<'_>
///     ) -> fmt::Result {
///         let data = attachment.inner();
///         write!(f, "{}: {}", data.name, data.value)
///     }
/// }
/// ```
pub trait AttachmentFormattingOverride<A>: 'static + Send + Sync
where
    A: markers::ObjectMarker + ?Sized,
{
    /// Formats the attachment using Display formatting.
    ///
    /// This method is called when the attachment needs to be displayed in a user-friendly
    /// format. The default implementation delegates to the attachment's unhooked Display
    /// formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the attachment being formatted
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::formatting_overrides::{
    ///     AttachmentFormattingOverride, AttachmentParent
    /// };
    /// use rootcause::report_attachment::ReportAttachmentRef;
    /// use core::fmt;
    ///
    /// struct ErrorCode(u32);
    /// struct ErrorCodeFormatter;
    ///
    /// impl AttachmentFormattingOverride<ErrorCode> for ErrorCodeFormatter {
    ///     fn display(
    ///         &self,
    ///         attachment: ReportAttachmentRef<'_, ErrorCode>,
    ///         _parent: Option<AttachmentParent<'_>>,
    ///         f: &mut fmt::Formatter<'_>
    ///     ) -> fmt::Result {
    ///         write!(f, "Error Code: 0x{:04X}", attachment.inner().0)
    ///     }
    /// }
    /// ```
    fn display(
        &self,
        attachment: ReportAttachmentRef<'_, A>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Display::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    /// Formats a preformatted attachment using Display formatting.
    ///
    /// This method handles attachments that have been preformatted (typically done
    /// using [`ReportRef::preformat`]). The default implementation delegates
    /// to the attachment's unhooked Display formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the preformatted attachment
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    fn display_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Display::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    /// Formats the attachment using Debug formatting.
    ///
    /// This method is called when the attachment needs to be displayed in a
    /// debug-friendly format. The default
    /// implementation delegates to the attachment's unhooked Debug formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the attachment being formatted
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    fn debug(
        &self,
        attachment: ReportAttachmentRef<'_, A>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Debug::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    /// Formats a preformatted attachment using Debug formatting.
    ///
    /// This method handles attachments that have been preformatted (typically done
    /// using [`ReportRef::preformat`]). The default implementation delegates
    /// to the attachment's unhooked Debug formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the preformatted attachment
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    fn debug_preformatted(
        &self,
        attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let _ = attachment_parent;
        fmt::Debug::fmt(&attachment.format_inner_unhooked(), formatter)
    }

    /// Determines the preferred formatting style for this attachment.
    ///
    /// This method allows the formatter to specify how the attachment should be
    /// presented in the overall report structure (inline, with header, in appendix, etc.).
    /// The default implementation delegates to the attachment's unhooked preference.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the attachment (as `dyn Any` as it can be either `A` or a [`PreformattedAttachment`])
    /// * `report_formatting_function` - Whether the overall report uses Display or Debug formatting
    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, dyn Any>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        attachment.preferred_formatting_style_unhooked(report_formatting_function)
    }
}

impl<A, H> UntypedAttachmentFormattingOverride for Hook<A, H>
where
    A: markers::ObjectMarker + ?Sized,
    H: AttachmentFormattingOverride<A>,
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

/// Registers a formatting override hook for attachments of type `A`.
///
/// This function allows you to customize how attachments of a specific type are
/// formatted in error reports. Once registered, the hook will be called whenever
/// an attachment of type `A` needs to be formatted.
///
/// The registration includes location tracking for debugging purposes, so you can
/// identify where hooks were registered using [`debug_attachment_hooks`].
///
/// # Type Parameters
///
/// * `A` - The type of attachment this hook will handle
/// * `H` - The type of the formatting override hook
///
/// # Arguments
///
/// * `hook` - An implementation of [`AttachmentFormattingOverride<A>`]
///
/// # Examples
///
/// ```rust
/// use rootcause::hooks::formatting_overrides::{
///     AttachmentFormattingOverride, AttachmentParent, register_attachment_hook
/// };
/// use rootcause::report_attachment::ReportAttachmentRef;
/// use core::fmt;
///
/// struct ApiError {
///     code: u32,
///     message: String,
/// }
///
/// struct ApiErrorFormatter;
///
/// impl AttachmentFormattingOverride<ApiError> for ApiErrorFormatter {
///     fn display(
///         &self,
///         attachment: ReportAttachmentRef<'_, ApiError>,
///         _parent: Option<AttachmentParent<'_>>,
///         f: &mut fmt::Formatter<'_>
///     ) -> fmt::Result {
///         let err = attachment.inner();
///         write!(f, "API Error {}: {}", err.code, err.message)
///     }
/// }
///
/// register_attachment_hook::<ApiError, _>(ApiErrorFormatter);
/// ```
#[track_caller]
pub fn register_attachment_hook<A, H>(hook: H)
where
    A: 'static,
    H: AttachmentFormattingOverride<A> + Send + Sync + 'static,
{
    let added_location = Location::caller();
    let hook: Hook<A, H> = Hook {
        hook,
        added_at: added_location,
        _hooked_type: PhantomData,
    };
    let hook: Arc<Hook<A, H>> = Arc::new(hook);
    let hook = hook.unsize(unsize::Coercion!(to dyn UntypedAttachmentFormattingOverride));

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

/// Calls a function for each registered attachment formatting hook for debugging purposes.
///
/// This utility function allows you to inspect all currently registered attachment
/// formatting hooks. Each hook provides information about the hook type, the attachment
/// type it handles, and where it was registered.
///
/// # Arguments
///
/// * `f` - A function that will be called once for each registered hook with a
///         displayable representation of the hook information
///
/// # Warning
///
/// This function will lock the internal hook registry for reading, so it can potentially
/// cause deadlocks if [`register_attachment_hook`] is called while the function
/// is executing.
///
/// # Examples
///
/// ```rust
/// use rootcause::hooks::formatting_overrides::debug_attachment_hooks;
///
/// // Print all registered attachment hooks
/// debug_attachment_hooks(|hook| {
///     println!("Registered hook: {}", hook);
/// });
/// ```
pub fn debug_attachment_hooks(mut f: impl FnMut(&dyn core::fmt::Display)) {
    if let Some(hooks) = HOOKS.read().get() {
        for hook in hooks.values() {
            f(hook);
        }
    }
}
