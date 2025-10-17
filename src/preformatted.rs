use alloc::{format, string::String};
use core::any::TypeId;

use rootcause_internals::handlers::{
    AttachmentFormattingStyle, AttachmentHandler, ContextFormattingStyle, ContextHandler,
};

use crate::{ReportRef, markers, report_attachment::ReportAttachmentRef};

/// A context that has been preformatted into `String`s for both
/// `Display` and `Debug`.
pub struct PreformattedContext {
    original_type_id: TypeId,
    display: String,
    debug: String,
    display_preferred_formatting_style: ContextFormattingStyle,
    debug_preferred_formatting_style: ContextFormattingStyle,
}

impl PreformattedContext {
    pub(crate) fn new_from_context<C, O, T>(report: ReportRef<'_, C, O, T>) -> Self
    where
        C: markers::ObjectMarker + ?Sized,
        O: markers::ReportRefOwnershipMarker,
        T: markers::ThreadSafetyMarker,
    {
        Self {
            original_type_id: report.current_context_type_id(),
            display: format!("{}", report.format_current_context()),
            debug: format!("{:?}", report.format_current_context()),
            display_preferred_formatting_style: report.preferred_context_formatting_style(
                rootcause_internals::handlers::FormattingFunction::Display,
            ),
            debug_preferred_formatting_style: report.preferred_context_formatting_style(
                rootcause_internals::handlers::FormattingFunction::Debug,
            ),
        }
    }

    pub fn original_type_id(&self) -> TypeId {
        self.original_type_id
    }
}

/// A context that has been preformatted into `String`s for both
/// `Display` and `Debug`.
pub struct PreformattedAttachment {
    original_type_id: TypeId,
    display: String,
    debug: String,
    display_preferred_formatting_style: AttachmentFormattingStyle,
    debug_preferred_formatting_style: AttachmentFormattingStyle,
}

impl PreformattedAttachment {
    pub(crate) fn new_from_attachment<A>(attachment: ReportAttachmentRef<'_, A>) -> Self
    where
        A: markers::ObjectMarker + ?Sized,
    {
        Self {
            original_type_id: attachment.inner_type_id(),
            display: format!("{}", attachment.format_inner()),
            debug: format!("{:?}", attachment.format_inner()),
            display_preferred_formatting_style: attachment.preferred_formatting_style(
                rootcause_internals::handlers::FormattingFunction::Display,
            ),
            debug_preferred_formatting_style: attachment.preferred_formatting_style(
                rootcause_internals::handlers::FormattingFunction::Debug,
            ),
        }
    }

    pub fn original_type_id(&self) -> TypeId {
        self.original_type_id
    }
}

pub(crate) struct PreformattedHandler;

impl ContextHandler<PreformattedContext> for PreformattedHandler {
    fn source(_value: &PreformattedContext) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(
        value: &PreformattedContext,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str(&value.display)
    }

    fn debug(
        value: &PreformattedContext,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str(&value.debug)
    }

    fn preferred_formatting_style(
        value: &PreformattedContext,
        report_formatting_function: rootcause_internals::handlers::FormattingFunction,
    ) -> ContextFormattingStyle {
        match report_formatting_function {
            rootcause_internals::handlers::FormattingFunction::Display => {
                value.display_preferred_formatting_style
            }
            rootcause_internals::handlers::FormattingFunction::Debug => {
                value.debug_preferred_formatting_style
            }
        }
    }
}

impl AttachmentHandler<PreformattedAttachment> for PreformattedHandler {
    fn display(
        value: &PreformattedAttachment,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str(&value.display)
    }

    fn debug(
        value: &PreformattedAttachment,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str(&value.debug)
    }

    fn preferred_formatting_style(
        value: &PreformattedAttachment,
        report_formatting_function: rootcause_internals::handlers::FormattingFunction,
    ) -> AttachmentFormattingStyle {
        match report_formatting_function {
            rootcause_internals::handlers::FormattingFunction::Display => {
                value.display_preferred_formatting_style
            }
            rootcause_internals::handlers::FormattingFunction::Debug => {
                value.debug_preferred_formatting_style
            }
        }
    }
}
