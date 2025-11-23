//! Preformatted context and attachment types.
//!
//! # Overview
//!
//! This module provides types that store preformatted `String` representations
//! of report contexts and attachments. These types are primarily used through
//! the [`Report::preformat`] method, which converts any report into a version
//! where all contexts and attachments have been formatted into strings.
//!
//! # Why Preformat?
//!
//! Preformatting a report is useful in several scenarios:
//!
//! - **Regaining mutability**: After preformatting, you get back a [`Mutable`]
//!   report even if the original was [`Cloneable`], allowing you to add more
//!   context or attachments to the root node.
//! - **Thread safety**: Non-`Send`/`Sync` error types can be preformatted to
//!   create a `Send + Sync` report that can be transferred across thread
//!   boundaries.
//! - **Preserving formatting**: The preformatted version will always display
//!   the same way, even if the original types or handlers are no longer
//!   available.
//!
//! # Usage
//!
//! The typical usage is to call [`Report::preformat`] on an existing report:
//!
//! ```
//! use rootcause::{
//!     markers::{Mutable, SendSync},
//!     preformatted::PreformattedContext,
//!     prelude::*,
//! };
//!
//! let report: Report = report!("database connection failed");
//! let preformatted: Report<PreformattedContext, Mutable, SendSync> = report.preformat();
//!
//! // The preformatted report displays identically to the original
//! assert_eq!(format!("{}", report), format!("{}", preformatted));
//! ```
//!
//! # Non-Send/Sync Example
//!
//! ```
//! use core::cell::Cell;
//!
//! use rootcause::{
//!     markers::{Local, SendSync},
//!     preformatted::PreformattedContext,
//!     prelude::*,
//! };
//!
//! // Cell is !Send and !Sync
//! #[derive(Debug)]
//! struct LocalError {
//!     counter: Cell<u32>,
//! }
//!
//! impl core::fmt::Display for LocalError {
//!     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//!         write!(f, "Local error: {}", self.counter.get())
//!     }
//! }
//!
//! let local_report: Report<LocalError, _, Local> = report!(LocalError {
//!     counter: Cell::new(42)
//! });
//!
//! // Preformat to make it Send + Sync
//! let send_sync_report: Report<PreformattedContext, _, SendSync> = local_report.preformat();
//!
//! // Now it can be sent across threads
//! // std::thread::spawn(move || { ... send_sync_report ... });
//! ```
//!
//! [`Report::preformat`]: crate::Report::preformat
//! [`Mutable`]: crate::markers::Mutable
//! [`Cloneable`]: crate::markers::Cloneable

use alloc::{format, string::String};
use core::any::TypeId;

use rootcause_internals::handlers::{
    AttachmentFormattingStyle, AttachmentHandler, ContextFormattingStyle, ContextHandler,
};

use crate::{ReportRef, markers, report_attachment::ReportAttachmentRef};

/// A context that has been preformatted into `String`s for both
/// `Display` and `Debug`.
///
/// This type stores the formatted output of a context along with metadata about
/// the original type and preferred formatting styles. It's created
/// automatically by [`Report::preformat`] and should not typically be
/// constructed manually.
///
/// # Stored Information
///
/// - The original type's [`TypeId`] (accessible via [`original_type_id`])
/// - Preformatted `Display` output as a `String`
/// - Preformatted `Debug` output as a `String`
/// - Preferred formatting styles for both `Display` and `Debug`
///
/// # Examples
///
/// ```
/// use core::any::TypeId;
///
/// use rootcause::{preformatted::PreformattedContext, prelude::*};
///
/// #[derive(Debug)]
/// struct MyError;
/// impl core::fmt::Display for MyError {
///     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
///         write!(f, "my error")
///     }
/// }
///
/// let original: Report<MyError> = report!(MyError);
/// let original_type_id = TypeId::of::<MyError>();
///
/// let preformatted: Report<PreformattedContext> = original.preformat();
///
/// // The preformatted context remembers its original type
/// assert_eq!(
///     preformatted.current_context().original_type_id(),
///     original_type_id
/// );
/// ```
///
/// [`Report::preformat`]: crate::Report::preformat
/// [`original_type_id`]: PreformattedContext::original_type_id
/// [`TypeId`]: core::any::TypeId
pub struct PreformattedContext {
    original_type_id: TypeId,
    display: String,
    debug: String,
    display_preferred_formatting_style: ContextFormattingStyle,
    debug_preferred_formatting_style: ContextFormattingStyle,
}

impl PreformattedContext {
    pub(crate) fn new_from_context<C: ?Sized, O, T>(report: ReportRef<'_, C, O, T>) -> Self {
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

    /// Get the [`TypeId`] of the original context type before it was
    /// preformatted.
    ///
    /// This can be useful for debugging or for implementing custom logic based
    /// on the original type, even though the actual type has been erased.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::any::TypeId;
    ///
    /// use rootcause::{preformatted::PreformattedContext, prelude::*};
    ///
    /// #[derive(Debug)]
    /// struct DatabaseError {
    ///     code: i32,
    /// }
    ///
    /// impl core::fmt::Display for DatabaseError {
    ///     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    ///         write!(f, "Database error: {}", self.code)
    ///     }
    /// }
    ///
    /// let report: Report<DatabaseError> = report!(DatabaseError { code: 404 });
    /// let preformatted: Report<PreformattedContext> = report.preformat();
    ///
    /// // Even though the type is now PreformattedContext, we can still check
    /// // what the original type was
    /// assert_eq!(
    ///     preformatted.current_context().original_type_id(),
    ///     TypeId::of::<DatabaseError>()
    /// );
    /// ```
    ///
    /// [`TypeId`]: core::any::TypeId
    pub fn original_type_id(&self) -> TypeId {
        self.original_type_id
    }
}

/// An attachment that has been preformatted into `String`s for both
/// `Display` and `Debug`.
///
/// This type stores the formatted output of an attachment along with metadata
/// about the original type and preferred formatting styles. It's created
/// automatically by [`Report::preformat`] and should not typically be
/// constructed manually.
///
/// # Stored Information
///
/// - The original type's [`TypeId`] (accessible via [`original_type_id`])
/// - Preformatted `Display` output as a `String`
/// - Preformatted `Debug` output as a `String`
/// - Preferred formatting styles for both `Display` and `Debug`
///
/// # Examples
///
/// ```
/// use core::any::TypeId;
///
/// use rootcause::{preformatted::PreformattedAttachment, prelude::*};
///
/// // When a report is preformatted, all attachments become PreformattedAttachment
/// let report: Report = report!("error").attach("some data");
/// let preformatted = report.preformat();
///
/// // All attachments in a preformatted report are PreformattedAttachment
/// // They preserve information about their original types
/// for attachment in preformatted.attachments().iter() {
///     // Each attachment remembers its original type through original_type_id()
///     let _original_type = attachment.inner_type_id();
/// }
/// ```
///
/// [`Report::preformat`]: crate::Report::preformat
/// [`original_type_id`]: PreformattedAttachment::original_type_id
/// [`TypeId`]: core::any::TypeId
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

    /// Get the [`TypeId`] of the original attachment type before it was
    /// preformatted.
    ///
    /// This can be useful for debugging or for implementing custom logic based
    /// on the original attachment type, even though the actual type has
    /// been erased.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::any::TypeId;
    ///
    /// use rootcause::{preformatted::PreformattedAttachment, prelude::*};
    ///
    /// let report: Report = report!("error").attach(42u32);
    /// let preformatted = report.preformat();
    ///
    /// // After preformatting, all attachments are PreformattedAttachment
    /// // They preserve the original type information
    /// for attachment in preformatted.attachments().iter() {
    ///     // Each PreformattedAttachment can tell you what type it originally was
    ///     if let Some(preformatted) = attachment.downcast_inner::<PreformattedAttachment>() {
    ///         let original_type = preformatted.original_type_id();
    ///         // Can check if it was a specific type, e.g.:
    ///         // if original_type == TypeId::of::<u32>() { ... }
    ///     }
    /// }
    /// ```
    ///
    /// [`TypeId`]: core::any::TypeId
    pub fn original_type_id(&self) -> TypeId {
        self.original_type_id
    }
}

/// Internal handler for preformatted contexts and attachments.
///
/// This handler is automatically registered for [`PreformattedContext`] and
/// [`PreformattedAttachment`] types. It retrieves the pre-stored formatted
/// strings rather than performing any formatting at display time.
///
/// [`Report::preformat`]: crate::Report::preformat
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
