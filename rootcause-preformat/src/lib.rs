#![no_std]

extern crate alloc;

use rootcause::{
    Report, ReportMut, ReportRef, handlers,
    markers::{self, Mutable, ReportOwnershipMarker, SendSync},
    report_attachment::{ReportAttachment, ReportAttachmentMut, ReportAttachmentRef},
};

mod preformatted;

pub use preformatted::{PreformattedAttachment, PreformattedContext};

pub trait PreformatReportExt {
    /// Creates a new report, which has the same structure as the current
    /// report, but has all the contexts and attachments preformatted.
    ///
    /// This can be useful, as the new report is mutable because it was just
    /// created, and additionally the new report is [`Send`]+[`Sync`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, preformatted::PreformattedContext};
    /// # #[derive(Default)]
    /// # struct NonSendSyncError(core::cell::Cell<()>);
    /// # let non_send_sync_error = NonSendSyncError::default();
    /// # let mut report = report!(non_send_sync_error);
    /// let report_mut: ReportMut<'_, NonSendSyncError, markers::Local> = report.as_mut();
    /// let preformatted: Report<PreformattedContext, markers::Mutable, markers::SendSync> =
    ///     report_mut.preformat();
    /// assert_eq!(format!("{report}"), format!("{preformatted}"));
    /// ```
    #[track_caller]
    #[must_use]
    fn preformat(&self) -> Report<PreformattedContext, Mutable, SendSync>;
}

pub trait PreformatAttachmentExt {
    /// Creates a new attachment, with the inner attachment data preformatted.
    ///
    /// This can be useful, as the preformatted attachment is a newly allocated
    /// object and additionally is [`Send`]+[`Sync`].
    ///
    /// See [`PreformattedAttachment`] for more information.
    ///
    /// [`PreformattedAttachment`](crate::preformatted::PreformattedAttachment)
    #[track_caller]
    #[must_use]
    fn preformat(&self) -> ReportAttachment<PreformattedAttachment, SendSync>;
}

impl<A: ?Sized, T> PreformatAttachmentExt for ReportAttachment<A, T> {
    fn preformat(&self) -> ReportAttachment<PreformattedAttachment, SendSync> {
        self.as_ref().preformat()
    }
}

impl<'a, A: ?Sized> PreformatAttachmentExt for ReportAttachmentMut<'a, A> {
    fn preformat(&self) -> ReportAttachment<PreformattedAttachment, SendSync> {
        self.as_ref().preformat()
    }
}

impl<'a, A: ?Sized> PreformatAttachmentExt for ReportAttachmentRef<'a, A> {
    fn preformat(&self) -> ReportAttachment<PreformattedAttachment, SendSync> {
        ReportAttachment::new_custom::<preformatted::PreformattedHandler>(
            PreformattedAttachment::new_from_attachment(*self),
        )
    }
}

pub trait PreformatRootExt<C, T>: Sized {
    /// Extracts the context and returns it with a preformatted version of the
    /// report.
    ///
    /// Returns a tuple: the original typed context and a new report with
    /// [`PreformattedContext`](crate::preformatted::PreformattedContext)
    /// containing the string representation. The preformatted report maintains
    /// the same structure (children and attachments). Useful when you need
    /// the typed value for processing and the formatted version for display.
    ///
    /// This is a lower-level method primarily for custom transformation logic.
    /// Most users should use
    /// [`context_transform`](Self::context_transform),
    /// [`context_transform_nested`](Self::context_transform_nested),
    /// or [`context_to`](Self::context_to) instead.
    ///
    /// See also: [`preformat`](Report::preformat) (formats entire hierarchy),
    /// [`into_parts`](Report::into_parts) (extracts without formatting),
    /// [`current_context`](crate::ReportRef::current_context) (reference
    /// without extraction).
    ///
    /// # Examples
    ///
    /// ```
    /// # use rootcause::{preformatted::PreformattedContext, prelude::*};
    /// # #[derive(Debug)]
    /// struct MyError {
    ///     code: u32
    /// }
    /// # impl std::fmt::Display for MyError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "error {}", self.code) }
    /// # }
    ///
    /// let report: Report<MyError> = report!(MyError { code: 500 });
    /// let (context, preformatted): (MyError, Report<PreformattedContext>) = report.preformat_root();
    /// ```
    #[track_caller]
    #[must_use]
    fn preformat_root(self) -> (C, Report<PreformattedContext, Mutable, T>)
    where
        PreformattedContext: markers::ObjectMarkerFor<T>;
}

pub trait ContextTransformNestedExt<C, T>: Sized {
    type Output<D: 'static>;

    /// Transforms the context and nests the original report as a preformatted
    /// child.
    ///
    /// Creates a new parent node with fresh hook data (location, backtrace),
    /// but the original context type is lost—the child becomes
    /// [`PreformattedContext`] and cannot be downcast.
    ///
    /// [`PreformattedContext`]: crate::preformatted::PreformattedContext
    ///
    /// # Examples
    ///
    /// ```
    /// # use rootcause::prelude::*;
    /// # #[derive(Debug)]
    /// # struct LibError;
    /// # impl std::fmt::Display for LibError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "lib error") }
    /// # }
    /// # #[derive(Debug)]
    /// enum AppError {
    ///     Lib(LibError)
    /// }
    /// # impl std::fmt::Display for AppError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "app error") }
    /// # }
    ///
    /// let lib_report: Report<LibError> = report!(LibError);
    /// let app_report: Report<AppError> = lib_report.context_transform_nested(AppError::Lib);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`context_transform()`](Report::context_transform) - Transforms
    ///   without nesting
    /// - [`context()`](Report::context) - Adds new parent, preserves child's
    ///   type
    /// - [`preformat_root()`](Report::preformat_root) - Lower-level operation
    ///   used internally
    /// - [`examples/context_methods.rs`] - Comparison guide
    ///
    /// [`examples/context_methods.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/context_methods.rs
    #[track_caller]
    #[must_use]
    fn context_transform_nested<F, D>(self, f: F) -> Self::Output<D>
    where
        F: FnOnce(C) -> D,
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
        PreformattedContext: markers::ObjectMarkerFor<T>;
}

impl<C: ?Sized, O, T> PreformatReportExt for Report<C, O, T>
where
    O: ReportOwnershipMarker,
{
    fn preformat(&self) -> Report<PreformattedContext, Mutable, SendSync> {
        self.as_ref().preformat()
    }
}

impl<'a, C: ?Sized, T> PreformatReportExt for ReportMut<'a, C, T> {
    fn preformat(&self) -> Report<PreformattedContext, Mutable, SendSync> {
        self.as_ref().preformat()
    }
}

impl<'a, C: ?Sized, O, T> PreformatReportExt for ReportRef<'a, C, O, T> {
    fn preformat(&self) -> Report<PreformattedContext, Mutable, SendSync> {
        let preformatted_context = PreformattedContext::new_from_context(*self);
        Report::from_parts_unhooked::<preformatted::PreformattedHandler>(
            preformatted_context,
            self.children()
                .iter()
                .map(|sub_report| sub_report.preformat())
                .collect(),
            self.attachments()
                .iter()
                .map(|attachment| attachment.preformat().into_dynamic())
                .collect(),
        )
    }
}

impl<C, T> PreformatRootExt<C, T> for Report<C, Mutable, T> {
    fn preformat_root(self) -> (C, Report<PreformattedContext, Mutable, T>)
    where
        PreformattedContext: markers::ObjectMarkerFor<T>,
    {
        let preformatted = PreformattedContext::new_from_context(self.as_ref());
        let (context, children, attachments) = self.into_parts();

        (
            context,
            Report::from_parts_unhooked::<preformatted::PreformattedHandler>(
                preformatted,
                children,
                attachments,
            ),
        )
    }
}

impl<C, T> ContextTransformNestedExt<C, T> for Report<C, Mutable, T> {
    type Output<D: 'static> = Report<D, Mutable, T>;

    fn context_transform_nested<F, D>(self, f: F) -> Report<D, Mutable, T>
    where
        F: FnOnce(C) -> D,
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
        PreformattedContext: markers::ObjectMarkerFor<T>,
    {
        let (context, report) = self.preformat_root();
        report.context_custom::<handlers::Display, _>(f(context))
    }
}

impl<V, C, T> ContextTransformNestedExt<C, T> for Result<V, Report<C, Mutable, T>> {
    type Output<D: 'static> = Result<V, Report<D, Mutable, T>>;

    fn context_transform_nested<F, D>(self, f: F) -> Result<V, Report<D, Mutable, T>>
    where
        F: FnOnce(C) -> D,
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
        PreformattedContext: markers::ObjectMarkerFor<T>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(report) => {
                let (context, report) = report.preformat_root();
                Err(report.context_custom::<handlers::Display, _>(f(context)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rootcause::{
        ReportRef,
        markers::{Local, Mutable, SendSync, Uncloneable},
        prelude::*,
    };

    use super::*;

    #[test]
    fn test_preformat() {
        #[derive(Default)]
        struct NonSendSyncError(core::cell::Cell<()>);
        let non_send_sync_error = NonSendSyncError::default();
        let report = report!(non_send_sync_error);
        let report_ref: ReportRef<'_, NonSendSyncError, Uncloneable, Local> = report.as_ref();
        let preformatted: Report<PreformattedContext, Mutable, SendSync> = report_ref.preformat();
        assert_eq!(alloc::format!("{report}"), alloc::format!("{preformatted}"));
    }
}
