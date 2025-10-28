//! Prelude module re-exporting commonly used items.

pub use core::any::Any;

pub use crate::{
    IntoReport, Report, bail, handlers, iterator_ext::IteratorExt, markers, report,
    report_attachment, result_ext::ResultExt,
};
