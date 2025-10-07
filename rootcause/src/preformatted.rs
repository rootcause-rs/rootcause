use alloc::{format, string::String};
use core::any::TypeId;

use rootcause_internals::handlers::{AttachmentHandler, ContextHandler};

use crate::{markers, report::ReportRef, report_attachment::ReportAttachmentRef};

pub struct Preformatted {
    original_type_id: TypeId,
    display: String,
    debug: String,
}

impl Preformatted {
    pub(crate) fn new_from_context<C, O, T>(report: ReportRef<'_, C, O, T>) -> Preformatted
    where
        C: markers::ObjectMarker + ?Sized,
        O: markers::ReportRefOwnershipMarker,
        T: markers::ThreadSafetyMarker,
    {
        Self {
            original_type_id: report.current_context_type_id(),
            display: format!("{}", report.format_current_context()),
            debug: format!("{:?}", report.format_current_context()),
        }
    }

    pub(crate) fn new_from_attachment<A>(report: ReportAttachmentRef<'_, A>) -> Preformatted
    where
        A: markers::ObjectMarker + ?Sized,
    {
        Self {
            original_type_id: report.inner_type_id(),
            display: format!("{}", report.format_inner()),
            debug: format!("{:?}", report.format_inner()),
        }
    }

    pub fn original_type_id(&self) -> TypeId {
        self.original_type_id
    }
}

pub(crate) struct PreformattedHandler;

impl ContextHandler<Preformatted> for PreformattedHandler {
    fn source(_value: &Preformatted) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(
        value: &Preformatted,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str(&value.display)
    }

    fn debug(value: &Preformatted, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&value.debug)
    }
}

impl AttachmentHandler<Preformatted> for PreformattedHandler {
    fn display(
        value: &Preformatted,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str(&value.display)
    }

    fn debug(value: &Preformatted, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&value.debug)
    }
}
