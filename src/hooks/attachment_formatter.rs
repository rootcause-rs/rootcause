//! Attachment formatting hooks for customizing how attached data is displayed.
//!
//! This module lets you control how individual pieces of attached data appear
//! in error reports. Use this to:
//! - Format data nicely (timestamps, file sizes, structured data)
//! - Control where data appears (inline vs appendix)
//! - Set priority (which attachments show first)
//! - Hide noisy or sensitive information
//!
//! **Note:** Hooks format a type globally across ALL errors. To control
//! formatting for a single attachment, use [`attach_custom()`] with a handler
//! instead.
//!
//! [`attach_custom()`]: crate::Report::attach_custom
//!
//! By installing hooks for specific types, you can customize how attachments
//! are formatted and where they appear in reports.
//!
//! # Key Components
//!
//! - [`AttachmentFormatterHook`] - Trait for implementing custom attachment
//!   formatting
//! - [`AttachmentParent`] - Context about the report containing an attachment
//!
//! # Examples
//!
//! ## Custom Formatting
//!
//! ```
//! use core::fmt;
//!
//! use rootcause::{
//!     handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
//!     hooks::{
//!         Hooks,
//!         attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
//!     },
//!     markers::Dynamic,
//!     report_attachment::ReportAttachmentRef,
//! };
//!
//! struct ApiInformation {
//!     code: u32,
//!     message: String,
//! }
//!
//! struct ApiInformationFormatter;
//!
//! impl AttachmentFormatterHook<ApiInformation> for ApiInformationFormatter {
//!     fn display(
//!         &self,
//!         attachment: ReportAttachmentRef<'_, ApiInformation>,
//!         _parent: Option<AttachmentParent<'_>>,
//!         f: &mut fmt::Formatter<'_>,
//!     ) -> fmt::Result {
//!         let err = attachment.inner();
//!         write!(f, "API Error {}: {}", err.code, err.message)
//!     }
//!
//!     fn preferred_formatting_style(
//!         &self,
//!         _attachment: ReportAttachmentRef<'_, Dynamic>,
//!         _report_formatting_function: FormattingFunction,
//!     ) -> AttachmentFormattingStyle {
//!         AttachmentFormattingStyle {
//!             placement: AttachmentFormattingPlacement::InlineWithHeader {
//!                 header: "API Error Details",
//!             },
//!             function: FormattingFunction::Display,
//!             priority: 100, // High priority for API errors
//!         }
//!     }
//! }
//!
//! // Install the custom formatter globally
//! Hooks::new()
//!     .attachment_formatter::<ApiInformation, _>(ApiInformationFormatter)
//!     .install()
//!     .expect("failed to install hooks");
//! ```
//!
//! ## Controlling Placement and Priority
//!
//! Control where attachments appear and in what order:
//!
//! ```
//! use rootcause::{
//!     handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
//!     hooks::{Hooks, attachment_formatter::AttachmentFormatterHook},
//!     markers::Dynamic,
//!     report_attachment::ReportAttachmentRef,
//! };
//!
//! struct LogEntry(String);
//!
//! struct LogFormatter;
//!
//! impl AttachmentFormatterHook<LogEntry> for LogFormatter {
//!     fn preferred_formatting_style(
//!         &self,
//!         _attachment: ReportAttachmentRef<'_, Dynamic>,
//!         _report_formatting_function: FormattingFunction,
//!     ) -> AttachmentFormattingStyle {
//!         AttachmentFormattingStyle {
//!             // Put logs in appendix to reduce noise in main error
//!             placement: AttachmentFormattingPlacement::Appendix {
//!                 appendix_name: "Log Entries",
//!             },
//!             function: FormattingFunction::Display,
//!             priority: 10, // Lower priority than important data
//!         }
//!     }
//! }
//!
//! Hooks::new()
//!     .attachment_formatter::<LogEntry, _>(LogFormatter)
//!     .install()
//!     .expect("failed to install hooks");
//! ```
//!
//! ## Suppressing display of Attachments
//!
//! Omit noisy or unnecessary information by setting placement to `Opaque`
//! when formatting as display:
//!
//! ```
//! use rootcause::{
//!     handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
//!     hooks::{Hooks, attachment_formatter::AttachmentFormatterHook},
//!     markers::Dynamic,
//!     report_attachment::ReportAttachmentRef,
//! };
//!
//! struct DebugInfo(String);
//!
//! struct OmitDebugInfo;
//!
//! impl AttachmentFormatterHook<DebugInfo> for OmitDebugInfo {
//!     fn preferred_formatting_style(
//!         &self,
//!         _attachment: ReportAttachmentRef<'_, Dynamic>,
//!         report_formatting_function: FormattingFunction,
//!     ) -> AttachmentFormattingStyle {
//!         AttachmentFormattingStyle {
//!             placement: if report_formatting_function == FormattingFunction::Display {
//!                 // This will still show up as a count of omitted attachments,
//!                 // can be set it as Hidden instead to make it completely invisible.
//!                 AttachmentFormattingPlacement::Opaque,
//!             } else {
//!                 AttachmentFormattingPlacement::Inline
//!             },
//!             function: report_formatting_function,
//!             priority: 0,
//!         }
//!     }
//! }
//!
//! // Install hook to suppress debug info in production reports
//! Hooks::new()
//!     .attachment_formatter::<DebugInfo, _>(OmitDebugInfo)
//!     .install()
//!     .expect("failed to install hooks");
//! ```
//!
//! **Note:** Attachment formatter hooks provide explicit control over what
//! appears in reports. For sensitive data, you can also use custom handlers
//! (see [`attach_custom()`]) or [`crate::handlers::Debug`] which shows
//! "Context of type..." by default to avoid exposing debug data.
//!
//! [`attach_custom()`]: crate::Report::attach_custom

use alloc::{boxed::Box, fmt};
use core::{any::TypeId, marker::PhantomData};

use hashbrown::HashMap;
use rootcause_internals::handlers::{AttachmentFormattingStyle, FormattingFunction};

use crate::{
    ReportRef,
    hooks::{HookData, use_hooks},
    markers::{Dynamic, Local, Uncloneable},
    preformatted::PreformattedAttachment,
    report_attachment::ReportAttachmentRef,
};

#[derive(Default)]
pub(crate) struct HookMap {
    /// # Safety Invariant
    ///
    /// The hook stored under `TypeId::of::<A>()` is guaranteed to be an
    /// instance of the type `Hook<A, H>`.
    map: HashMap<TypeId, Box<dyn StoredHook>, rustc_hash::FxBuildHasher>,
}

impl core::fmt::Debug for HookMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.map.values().fmt(f)
    }
}

impl HookMap {
    /// Retrieves the formatter hook for the specified attachment
    /// type.
    ///
    /// The returned hook is guaranteed to be an instance of type `Hook<A, H>`,
    /// where `TypeId::of::<A>() == type_id`.
    fn get(&self, type_id: TypeId) -> Option<&dyn StoredHook> {
        Some(&**self.map.get(&type_id)?)
    }

    pub(crate) fn insert<A, H>(&mut self, hook: H)
    where
        A: Sized + 'static,
        H: AttachmentFormatterHook<A>,
    {
        let hook: Hook<A, H> = Hook {
            hook,
            _hooked_type: PhantomData,
        };
        let hook: Box<Hook<A, H>> = Box::new(hook);
        // We must uphold the safety invariant of HookMap.
        //
        // The safety invariant requires that the hook stored under
        // `TypeId::of::<A>()` is always of type `Hook<A, H>`.
        //
        // However this is exactly what we are doing here,
        // so the invariant is upheld.
        self.map.insert(TypeId::of::<A>(), hook);
    }
}

struct Hook<A, H>
where
    A: 'static,
{
    hook: H,
    _hooked_type: PhantomData<fn(A) -> A>,
}

impl<A, H> core::fmt::Debug for Hook<A, H>
where
    A: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttachmentFormattingHook<{}, {}>",
            core::any::type_name::<A>(),
            core::any::type_name::<H>(),
        )
    }
}

/// Information about the parent report that contains an attachment being
/// formatted.
///
/// This struct provides context about where an attachment exists within the
/// report hierarchy, which can be useful for custom formatting logic that needs
/// to understand the attachment's position or relationship to its containing
/// report.
///
/// # Examples
///
/// ```
/// use core::fmt;
///
/// use rootcause::{
///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
///     report_attachment::ReportAttachmentRef,
/// };
///
/// struct MyFormatter;
///
/// impl AttachmentFormatterHook<String> for MyFormatter {
///     fn display(
///         &self,
///         attachment: ReportAttachmentRef<'_, String>,
///         parent: Option<AttachmentParent<'_>>,
///         f: &mut fmt::Formatter<'_>,
///     ) -> fmt::Result {
///         if let Some(parent) = parent {
///             write!(
///                 f,
///                 "[Attachment {}] {}",
///                 parent.attachment_index,
///                 attachment.inner()
///             )
///         } else {
///             write!(f, "{}", attachment.inner())
///         }
///     }
/// }
/// ```
#[derive(Copy, Clone, Debug)]
pub struct AttachmentParent<'a> {
    /// Reference to the report that contains this attachment
    pub report: ReportRef<'a, Dynamic, Uncloneable, Local>,
    /// Index of this attachment within the parent report's attachment list
    pub attachment_index: usize,
}

/// Trait for untyped attachment formatter hooks.
///
/// This trait is guaranteed to only be implemented for [`Hook<A, H>`].
trait StoredHook: 'static + Send + Sync + core::fmt::Debug {
    /// Formats the attachment using Display formatting.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `A` stored in the attachment matches the `A` from type
    ///    `Hook<A, H>` this is implemented for.
    unsafe fn display(
        &self,
        attachment: ReportAttachmentRef<'_, Dynamic>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// Formats the attachment using Debug formatting.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The type `A` stored in the attachment matches the `A` from type
    ///    `Hook<A, H>` this is implemented for.
    unsafe fn debug(
        &self,
        attachment: ReportAttachmentRef<'_, Dynamic>,
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
        attachment: ReportAttachmentRef<'_, Dynamic>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle;
}

/// Trait for customizing how attachments of a specific type are formatted in
/// error reports.
///
/// This trait allows you to override the default formatting behavior for
/// attachments of type `A`. You can customize both Display and Debug
/// formatting, as well as handle preformatted attachments and specify preferred
/// formatting styles.
///
/// # Type Parameters
///
/// * `A` - The attachment type that this formatter handles
///
/// # Default Implementations
///
/// All methods have default implementations that delegate to the unhooked
/// formatting, so you only need to implement the methods for the formatting you
/// want to customize.
///
/// # Examples
///
/// Basic custom Display formatting:
/// ```
/// use core::fmt;
///
/// use rootcause::{
///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
///     report_attachment::ReportAttachmentRef,
/// };
///
/// struct MyData {
///     value: i32,
///     name: String,
/// }
///
/// struct MyDataFormatter;
///
/// impl AttachmentFormatterHook<MyData> for MyDataFormatter {
///     fn display(
///         &self,
///         attachment: ReportAttachmentRef<'_, MyData>,
///         _parent: Option<AttachmentParent<'_>>,
///         f: &mut fmt::Formatter<'_>,
///     ) -> fmt::Result {
///         let data = attachment.inner();
///         write!(f, "{}: {}", data.name, data.value)
///     }
/// }
/// ```
pub trait AttachmentFormatterHook<A>: 'static + Send + Sync {
    /// Formats the attachment using Display formatting.
    ///
    /// This method is called when the attachment needs to be displayed in a
    /// user-friendly format. The default implementation delegates to the
    /// attachment's unhooked Display formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the attachment being formatted
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    ///
    /// # Examples
    ///
    /// ```
    /// use core::fmt;
    ///
    /// use rootcause::{
    ///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
    ///     report_attachment::ReportAttachmentRef,
    /// };
    ///
    /// struct ErrorCode(u32);
    /// struct ErrorCodeFormatter;
    ///
    /// impl AttachmentFormatterHook<ErrorCode> for ErrorCodeFormatter {
    ///     fn display(
    ///         &self,
    ///         attachment: ReportAttachmentRef<'_, ErrorCode>,
    ///         _parent: Option<AttachmentParent<'_>>,
    ///         f: &mut fmt::Formatter<'_>,
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
    /// This method handles attachments that have been preformatted (typically
    /// done using [`ReportRef::preformat`]). The default implementation
    /// delegates to the attachment's unhooked Display formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the preformatted attachment
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
    ///     preformatted::PreformattedAttachment,
    ///     report_attachment::ReportAttachmentRef,
    /// };
    ///
    /// struct MyFormatter;
    /// impl AttachmentFormatterHook<String> for MyFormatter {
    ///     fn display_preformatted(
    ///         &self,
    ///         attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
    ///         _parent: Option<AttachmentParent<'_>>,
    ///         f: &mut core::fmt::Formatter<'_>,
    ///     ) -> core::fmt::Result {
    ///         write!(f, "[Preformatted] {}", attachment.format_inner_unhooked())
    ///     }
    /// }
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
    ///     report_attachment::ReportAttachmentRef,
    /// };
    ///
    /// struct MyFormatter;
    /// impl AttachmentFormatterHook<String> for MyFormatter {
    ///     fn debug(
    ///         &self,
    ///         attachment: ReportAttachmentRef<'_, String>,
    ///         _parent: Option<AttachmentParent<'_>>,
    ///         f: &mut core::fmt::Formatter<'_>,
    ///     ) -> core::fmt::Result {
    ///         write!(f, "Debug: {:?}", attachment.inner())
    ///     }
    /// }
    /// ```
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
    /// This method handles attachments that have been preformatted (typically
    /// done using [`ReportRef::preformat`]). The default implementation
    /// delegates to the attachment's unhooked Debug formatting.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the preformatted attachment
    /// * `attachment_parent` - Optional context about the parent report
    /// * `formatter` - The formatter to write output to
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
    ///     preformatted::PreformattedAttachment,
    ///     report_attachment::ReportAttachmentRef,
    /// };
    ///
    /// struct MyFormatter;
    /// impl AttachmentFormatterHook<String> for MyFormatter {
    ///     fn debug_preformatted(
    ///         &self,
    ///         attachment: ReportAttachmentRef<'_, PreformattedAttachment>,
    ///         _parent: Option<AttachmentParent<'_>>,
    ///         f: &mut core::fmt::Formatter<'_>,
    ///     ) -> core::fmt::Result {
    ///         write!(
    ///             f,
    ///             "[Preformatted Debug] {:?}",
    ///             attachment.format_inner_unhooked()
    ///         )
    ///     }
    /// }
    /// ```
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
    /// presented in the overall report structure (inline, with header, in
    /// appendix, etc.). The default implementation delegates to the
    /// attachment's unhooked preference.
    ///
    /// # Arguments
    ///
    /// * `attachment` - Reference to the attachment (as [`Dynamic`] as it can
    ///   be either `A` or a [`PreformattedAttachment`])
    /// * `report_formatting_function` - Whether the overall report uses Display
    ///   or Debug formatting
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{
    ///     handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
    ///     hooks::attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
    ///     markers::Dynamic,
    ///     report_attachment::ReportAttachmentRef,
    /// };
    ///
    /// struct MyFormatter;
    /// impl AttachmentFormatterHook<String> for MyFormatter {
    ///     fn preferred_formatting_style(
    ///         &self,
    ///         _attachment: ReportAttachmentRef<'_, Dynamic>,
    ///         formatting_function: FormattingFunction,
    ///     ) -> AttachmentFormattingStyle {
    ///         AttachmentFormattingStyle {
    ///             placement: AttachmentFormattingPlacement::InlineWithHeader { header: "Info" },
    ///             function: formatting_function,
    ///             priority: 100,
    ///         }
    ///     }
    /// }
    /// ```
    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, Dynamic>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        attachment.preferred_formatting_style_unhooked(report_formatting_function)
    }
}

impl<A, H> StoredHook for Hook<A, H>
where
    H: AttachmentFormatterHook<A>,
{
    unsafe fn display(
        &self,
        attachment: ReportAttachmentRef<'_, Dynamic>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // SAFETY:
        // 1. Guaranteed by the caller
        let attachment = unsafe { attachment.downcast_attachment_unchecked::<A>() };
        self.hook.display(attachment, attachment_parent, formatter)
    }

    unsafe fn debug(
        &self,
        attachment: ReportAttachmentRef<'_, Dynamic>,
        attachment_parent: Option<AttachmentParent<'_>>,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // SAFETY:
        // 1. Guaranteed by the caller
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
        attachment: ReportAttachmentRef<'_, Dynamic>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        self.hook
            .preferred_formatting_style(attachment, report_formatting_function)
    }
}

pub(crate) fn display_attachment(
    attachment: ReportAttachmentRef<'_, Dynamic>,
    attachment_parent: Option<AttachmentParent<'_>>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    use_hooks(|hook_data: Option<&HookData>| {
        if let Some(hook_data) = hook_data {
            let attachment_formatters: &HookMap = &hook_data.attachment_formatters;

            if let Some(attachment) = attachment.downcast_attachment::<PreformattedAttachment>()
                && let Some(hook) = attachment_formatters.get(attachment.inner().original_type_id())
            {
                return hook.display_preformatted(attachment, attachment_parent, formatter);
            }

            if let Some(hook) = attachment_formatters.get(attachment.inner_type_id()) {
                // SAFETY:
                // 1. The call to `get` guarantees that the returned hook is of type `Hook<A,
                //    H>`, and `TypeId::of<A>() == attachment.inner_type_id()`. Therefore the
                //    type `A` stored in the attachment matches the `A` from type `Hook<A, H>`.
                unsafe {
                    // @add-unsafe-context: StoredHook
                    return hook.display(attachment, attachment_parent, formatter);
                }
            }
        }
        fmt::Display::fmt(&attachment.format_inner_unhooked(), formatter)
    })
}

pub(crate) fn debug_attachment(
    attachment: ReportAttachmentRef<'_, Dynamic>,
    attachment_parent: Option<AttachmentParent<'_>>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    use_hooks(|hook_data: Option<&HookData>| {
        if let Some(hook_data) = hook_data {
            let attachment_formatters: &HookMap = &hook_data.attachment_formatters;

            if let Some(attachment) = attachment.downcast_attachment::<PreformattedAttachment>()
                && let Some(hook) = attachment_formatters.get(attachment.inner().original_type_id())
            {
                return hook.debug_preformatted(attachment, attachment_parent, formatter);
            }

            if let Some(hook) = attachment_formatters.get(attachment.inner_type_id()) {
                // SAFETY:
                // 1. The call to `get` guarantees that the returned hook is of type `Hook<A,
                //    H>`, and `TypeId::of<A>() == attachment.inner_type_id()`. Therefore the
                //    type `A` stored in the attachment matches the `A` from type `Hook<A, H>`.
                unsafe {
                    // @add-unsafe-context: StoredHook
                    return hook.debug(attachment, attachment_parent, formatter);
                }
            }
        }
        fmt::Debug::fmt(&attachment.format_inner_unhooked(), formatter)
    })
}

pub(crate) fn get_preferred_formatting_style(
    attachment: ReportAttachmentRef<'_, Dynamic>,
    report_formatting_function: FormattingFunction,
) -> AttachmentFormattingStyle {
    use_hooks(|hook_data: Option<&HookData>| {
        if let Some(hook_data) = hook_data {
            let attachment_formatters: &HookMap = &hook_data.attachment_formatters;
            if let Some(inner) = attachment.downcast_inner::<PreformattedAttachment>()
                && let Some(hook) = attachment_formatters.get(inner.original_type_id())
            {
                return hook.preferred_formatting_style(attachment, report_formatting_function);
            }

            if let Some(hook) = attachment_formatters.get(attachment.inner_type_id()) {
                return hook.preferred_formatting_style(attachment, report_formatting_function);
            }
        }
        attachment.preferred_formatting_style_unhooked(report_formatting_function)
    })
}
