#![cfg_attr(not(doc), no_std)]
#![deny(
    missing_docs,
    elided_lifetimes_in_paths,
    unsafe_code,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::broken_intra_doc_links,
    missing_copy_implementations,
    unused_doc_comments
)]
// Extra checks on nightly
#![cfg_attr(nightly_extra_checks, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(nightly_extra_checks, forbid(rustdoc::missing_doc_code_examples))]

//! Preformatting extensions for [`rootcause`] error reports.
//!
//! This crate adds the ability to turn a [`Report`] into a version where every
//! context and attachment has been formatted into a `String` and the original
//! types have been erased. The two storage types ([`PreformattedContext`] and
//! [`PreformattedAttachment`]) plus a handful of extension traits are exposed:
//!
//! - [`PreformatReportExt::preformat`] — preformat an entire report tree.
//! - [`PreformatAttachmentExt::preformat`] — preformat a single attachment.
//! - [`PreformatRootExt::preformat_root`] — extract the typed root context and
//!   return a preformatted report alongside it.
//! - [`ContextTransformNestedExt::context_transform_nested`] — transform the
//!   root context while nesting the original report as a preformatted child.
//!
//! # Why preformat?
//!
//! - **Regaining mutability**: After preformatting, you get back a [`Mutable`]
//!   report even if the original was [`Cloneable`].
//! - **Thread safety**: Non-`Send`/`Sync` error types can be preformatted to
//!   produce a `Send + Sync` report that can cross thread boundaries.
//! - **Preserving formatting**: The preformatted version will always display
//!   the same way, even if the original types or hooks are no longer
//!   available.
//!
//! # Quick Start
//!
//! ```
//! use rootcause::{
//!     markers::{Mutable, SendSync},
//!     prelude::*,
//! };
//! use rootcause_preformat::{PreformatReportExt, PreformattedContext};
//!
//! let report: Report = report!("database connection failed");
//! let preformatted: Report<PreformattedContext, Mutable, SendSync> = report.preformat();
//!
//! // The preformatted report displays identically to the original
//! assert_eq!(format!("{}", report), format!("{}", preformatted));
//! ```
//!
//! [`Mutable`]: rootcause::markers::Mutable
//! [`Cloneable`]: rootcause::markers::Cloneable

extern crate alloc;

use rootcause::{
    Report, ReportMut, ReportRef, handlers,
    markers::{self, Mutable, ReportOwnershipMarker, SendSync},
    report_attachment::{ReportAttachment, ReportAttachmentMut, ReportAttachmentRef},
};

mod preformatted;

pub use preformatted::{PreformattedAttachment, PreformattedContext};

/// Extension trait providing [`preformat`](Self::preformat) on [`Report`],
/// [`ReportRef`], and [`ReportMut`].
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
/// use rootcause_preformat::{PreformatReportExt, PreformattedContext};
///
/// let report: Report = report!("boom");
/// let preformatted: Report<PreformattedContext, _, _> = report.preformat();
/// ```
pub trait PreformatReportExt {
    /// Creates a new report, which has the same structure as the current
    /// report, but has all the contexts and attachments preformatted.
    ///
    /// This can be useful, as the new report is mutable because it was just
    /// created, and additionally the new report is [`Send`]+[`Sync`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # use rootcause_preformat::{PreformatReportExt, PreformattedContext};
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

/// Extension trait providing [`preformat`](Self::preformat) on
/// [`ReportAttachment`], [`ReportAttachmentRef`], and [`ReportAttachmentMut`].
///
/// # Examples
///
/// ```
/// use rootcause::report_attachment::ReportAttachment;
/// use rootcause_preformat::PreformatAttachmentExt;
///
/// let attachment = ReportAttachment::new_sendsync(42i32);
/// let preformatted = attachment.preformat();
/// assert_eq!(
///     attachment.format_inner().to_string(),
///     preformatted.format_inner().to_string()
/// );
/// ```
pub trait PreformatAttachmentExt {
    /// Creates a new attachment, with the inner attachment data preformatted.
    ///
    /// This can be useful, as the preformatted attachment is a newly allocated
    /// object and additionally is [`Send`]+[`Sync`].
    ///
    /// See [`PreformattedAttachment`] for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::report_attachment::ReportAttachment;
    /// use rootcause_preformat::PreformatAttachmentExt;
    ///
    /// let attachment = ReportAttachment::new_sendsync(42i32);
    /// let preformatted = attachment.preformat();
    /// ```
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

/// Extension trait providing [`preformat_root`](Self::preformat_root) on
/// [`Report`] with a [`Mutable`] root.
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
/// use rootcause_preformat::{PreformatRootExt, PreformattedContext};
///
/// #[derive(Debug)]
/// struct Boom;
/// # impl std::fmt::Display for Boom {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "boom") }
/// # }
///
/// let report: Report<Boom> = report!(Boom);
/// let (_context, _preformatted): (Boom, Report<PreformattedContext>) = report.preformat_root();
/// ```
pub trait PreformatRootExt<C, T>: Sized {
    /// Extracts the context and returns it with a preformatted version of the
    /// report.
    ///
    /// Returns a tuple: the original typed context and a new report with
    /// [`PreformattedContext`] containing the string representation. The
    /// preformatted report maintains the same structure (children and
    /// attachments). Useful when you need the typed value for processing and
    /// the formatted version for display.
    ///
    /// This is a lower-level method primarily for custom transformation logic.
    /// Most users should use
    /// [`context_transform_nested`](ContextTransformNestedExt::context_transform_nested)
    /// instead.
    ///
    /// See also: [`preformat`](PreformatReportExt::preformat) (formats entire
    /// hierarchy), [`into_parts`](rootcause::Report::into_parts) (extracts
    /// without formatting),
    /// [`current_context`](rootcause::ReportRef::current_context) (reference
    /// without extraction).
    ///
    /// # Examples
    ///
    /// ```
    /// # use rootcause::prelude::*;
    /// # use rootcause_preformat::{PreformatRootExt, PreformattedContext};
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

/// Extension trait providing
/// [`context_transform_nested`](Self::context_transform_nested) on [`Report`]
/// with a [`Mutable`] root, and on `Result<_, Report<_, Mutable, _>>`.
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
/// use rootcause_preformat::ContextTransformNestedExt;
///
/// #[derive(Debug)]
/// struct Inner;
/// # impl std::fmt::Display for Inner {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "inner") }
/// # }
/// #[derive(Debug)]
/// struct Outer(Inner);
/// # impl std::fmt::Display for Outer {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "outer") }
/// # }
///
/// let inner: Report<Inner> = report!(Inner);
/// let wrapped: Report<Outer> = inner.context_transform_nested(Outer);
/// ```
pub trait ContextTransformNestedExt<C, T>: Sized {
    /// The output type after transforming the context to `D`. Either
    /// `Report<D, Mutable, T>` or `Result<V, Report<D, Mutable, T>>`. See
    /// [`context_transform_nested`](Self::context_transform_nested) for an
    /// example.
    type Output<D: 'static>;

    /// Transforms the context and nests the original report as a preformatted
    /// child.
    ///
    /// Creates a new parent node with fresh hook data (location, backtrace),
    /// but the original context type is lost—the child becomes
    /// [`PreformattedContext`] and cannot be downcast.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rootcause::prelude::*;
    /// # use rootcause_preformat::ContextTransformNestedExt;
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
    /// - [`context()`](rootcause::Report::context) - Adds new parent, preserves
    ///   child's type
    /// - [`preformat_root()`](PreformatRootExt::preformat_root) - Lower-level
    ///   operation used internally
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
    use alloc::format;
    use core::any::TypeId;

    use rootcause::{
        ReportRef,
        markers::{Local, Mutable, SendSync, Uncloneable},
        prelude::*,
        report_attachment::ReportAttachment,
    };

    use super::*;

    #[derive(Debug)]
    struct DemoError(u32);

    impl core::fmt::Display for DemoError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "demo {}", self.0)
        }
    }

    #[derive(Debug)]
    struct Wrapper(#[allow(dead_code)] DemoError);

    impl core::fmt::Display for Wrapper {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "wrapper")
        }
    }

    #[test]
    fn test_preformat() {
        #[derive(Default)]
        struct NonSendSyncError(core::cell::Cell<()>);
        let non_send_sync_error = NonSendSyncError::default();
        let report = report!(non_send_sync_error);
        let report_ref: ReportRef<'_, NonSendSyncError, Uncloneable, Local> = report.as_ref();
        let preformatted: Report<PreformattedContext, Mutable, SendSync> = report_ref.preformat();
        assert_eq!(format!("{report}"), format!("{preformatted}"));
    }

    #[test]
    fn test_preformat_root_extracts_typed_context() {
        let report: Report<DemoError> = report!(DemoError(7)).attach("ctx-detail");
        let display_before = format!("{report}");
        let attachments_before = report.attachments().len();

        let (context, preformatted) = report.preformat_root();

        assert_eq!(context.0, 7);
        assert_eq!(format!("{preformatted}"), display_before);
        assert_eq!(
            preformatted.current_context().original_type_id(),
            TypeId::of::<DemoError>(),
        );
        assert_eq!(preformatted.attachments().len(), attachments_before);
    }

    #[test]
    fn test_context_transform_nested_on_report() {
        let inner: Report<DemoError> = report!(DemoError(3));

        let outer: Report<Wrapper> = inner.context_transform_nested(Wrapper);

        assert_eq!(outer.current_context_type_id(), TypeId::of::<Wrapper>());
        assert_eq!(outer.iter_sub_reports().count(), 1);
        let child = outer.children().get(0).unwrap();
        assert_eq!(
            child.current_context_type_id(),
            TypeId::of::<PreformattedContext>(),
        );
        let child_typed = child
            .downcast_report::<PreformattedContext>()
            .expect("child should be PreformattedContext");
        assert_eq!(
            child_typed.current_context().original_type_id(),
            TypeId::of::<DemoError>(),
        );
    }

    #[test]
    fn test_context_transform_nested_on_result_ok_passes_through() {
        let ok: Result<i32, Report<DemoError>> = Ok(42);
        let mapped: Result<i32, Report<Wrapper>> = ok.context_transform_nested(Wrapper);
        assert_eq!(mapped.unwrap(), 42);
    }

    #[test]
    fn test_context_transform_nested_on_result_err_wraps() {
        let err: Result<i32, Report<DemoError>> = Err(report!(DemoError(9)));
        let mapped: Result<i32, Report<Wrapper>> = err.context_transform_nested(Wrapper);

        let outer = mapped.unwrap_err();
        assert_eq!(outer.current_context_type_id(), TypeId::of::<Wrapper>());
        assert_eq!(outer.iter_sub_reports().count(), 1);
        let child = outer.children().get(0).unwrap();
        let child_typed = child
            .downcast_report::<PreformattedContext>()
            .expect("child should be PreformattedContext");
        assert_eq!(
            child_typed.current_context().original_type_id(),
            TypeId::of::<DemoError>(),
        );
    }

    #[test]
    fn test_preformat_attachment_owned_ref_mut() {
        let mut attachment = ReportAttachment::new_sendsync(42u32);
        let display = format!("{}", attachment.format_inner());
        let debug = format!("{:?}", attachment.format_inner());

        let from_owned = attachment.preformat();
        let from_ref = attachment.as_ref().preformat();
        let from_mut = attachment.as_mut().preformat();

        for preformatted in [&from_owned, &from_ref, &from_mut] {
            assert_eq!(format!("{}", preformatted.format_inner()), display);
            assert_eq!(format!("{:?}", preformatted.format_inner()), debug);
            assert_eq!(
                preformatted.inner().original_type_id(),
                TypeId::of::<u32>(),
            );
        }
    }
}
