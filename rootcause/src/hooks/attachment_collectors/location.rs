use alloc::borrow::Cow;
use rootcause_internals::handlers::{AttachmentFormattingStyle, AttachmentHandler};

use crate::hooks::AttachmentCollectorHook;

#[derive(Debug)]
pub struct Location {
    pub file: Cow<'static, str>,
    pub line: u32,
    pub column: u32,
}

pub struct LocationHandler;
impl AttachmentHandler<Location> for LocationHandler {
    fn display(value: &Location, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}:{}:{}", value.file, value.line, value.column)
    }

    fn debug(value: &Location, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(value, formatter)
    }

    fn preferred_formatting_style(
        _value: &Location,
        _report_formatting_function: rootcause_internals::handlers::FormattingFunction,
        _report_formatting_alternate: bool,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            priority: 20,
            ..Default::default()
        }
    }
}

pub struct LocationCollector;

impl AttachmentCollectorHook<Location> for LocationCollector {
    type Handler = LocationHandler;

    fn collect(&self) -> Location {
        let location = core::panic::Location::caller();
        Location {
            file: location.file().into(),
            line: location.line(),
            column: location.column(),
        }
    }
}
