use alloc::vec;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use rootcause_internals::{
    RawReportRef,
    handlers::{ContextFormattingStyle, FormattingFunction},
};

use crate::{
    Report, ReportIter,
    markers::{self, Cloneable, Local, Mutable, SendSync, Uncloneable},
    preformatted::{self, PreformattedAttachment, PreformattedContext},
    report_attachment::ReportAttachment,
    report_attachments::ReportAttachmentsRef,
    report_collection::ReportCollectionRef,
    util::format_helper,
};

/// A reference to a [`Report`].
///
/// Note that if you create a [`ReportRef`] from a [`Report`] marked as
/// [`Mutable`], then it will become a [`ReportRef`] with the [`Uncloneable`]
/// marker instead.
///
/// [`Mutable`]: crate::markers::Mutable
#[repr(transparent)]
pub struct ReportRef<'a, Context = dyn Any, Ownership = Cloneable, ThreadSafety = SendSync>
where
    Context: markers::ObjectMarker + ?Sized,
    Ownership: markers::ReportRefOwnershipMarker,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: RawReportRef<'a>,
    _context: PhantomData<Context>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, C, O, T> ReportRef<'a, C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new Report from a raw report
    ///
    /// # Safety
    ///
    /// To call this method you must ensure the following:
    ///
    /// - The context embedded in the [`RawReportRef`] must either be of type
    ///   `C`, or the `C` must be `dyn Any`
    /// - The thread safety marker must match the contents of the report. More
    ///   specifically if the marker is [`SendSync`], then all contexts and
    ///   attachments must be `Send+Sync`
    pub(crate) unsafe fn from_raw(raw: RawReportRef<'a>) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    pub(crate) fn as_raw_ref(self) -> RawReportRef<'a> {
        self.raw
    }

    /// Returns a reference to the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let context: &MyError = report_ref.current_context();
    /// ```
    pub fn current_context(self) -> &'a C
    where
        C: Sized,
    {
        unsafe { self.as_raw_ref().context_downcast_unchecked() }
    }

    /// Returns a reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, report_collection::ReportCollectionRef};
    /// let report = report!("parent error").into_cloneable();
    /// let report_ref: ReportRef<'_, _, _> = report.as_ref();
    /// let children: ReportCollectionRef<'_> = report_ref.children();
    /// assert_eq!(children.len(), 0); // The report has just been created, so it has no children
    /// ```
    pub fn children(self) -> ReportCollectionRef<'a, dyn Any, T> {
        let raw = self.as_raw_ref().children();
        unsafe { ReportCollectionRef::from_raw(raw) }
    }

    /// Returns a reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, report_attachments::ReportAttachmentsRef};
    /// # let report = report!("error with attachment").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let attachments: ReportAttachmentsRef<'_> = report_ref.attachments();
    /// ```
    pub fn attachments(self) -> ReportAttachmentsRef<'a, T> {
        let raw = self.as_raw_ref().attachments();
        unsafe { ReportAttachmentsRef::from_raw(raw) }
    }

    /// Changes the context type of the [`ReportRef`] to `dyn Any`.
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the context mode to
    /// `dyn Any`.
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that that the context actually has the
    /// type `C`.
    ///
    /// To get back the report with a concrete `C` you can use the method
    /// [`ReportRef::downcast_report`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # use core::any::Any;
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, dyn Any> = report_ref.into_dyn_any();
    /// ```
    pub fn into_dyn_any(self) -> ReportRef<'a, dyn Any, O, T> {
        unsafe { ReportRef::from_raw(self.as_raw_ref()) }
    }

    /// Changes the ownership mode of the [`ReportRef`] to [`Uncloneable`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the ownership mode to
    /// [`Uncloneable`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that the [`ReportRef`] is cloneable.
    ///
    /// After calling this method, you can add objects to the [`ReportRef`] that
    /// neither [`Send`] nor [`Sync`], but the report itself will no longer
    /// be [`Send`]+[`Sync`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::{Uncloneable, Cloneable}};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError, Cloneable> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, MyError, Uncloneable> = report_ref.into_uncloneable();
    /// ```
    pub fn into_uncloneable(self) -> ReportRef<'a, C, Uncloneable, T> {
        unsafe { ReportRef::from_raw(self.as_raw_ref()) }
    }

    /// Changes the thread safety mode of the [`ReportRef`] to [`Local`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the thread safety
    /// mode to [`Local`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that all objects in the [`ReportRef`] are
    /// actually [`Send`] and [`Sync`].
    ///
    /// After calling this method, you can add objects to the [`ReportRef`] that
    /// neither [`Send`] nor [`Sync`], but the report itself will no longer
    /// be [`Send`]+[`Sync`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::{SendSync, Local}};
    /// # let report = report!("my error").into_cloneable();
    /// let report_ref: ReportRef<'_, _, _, SendSync> = report.as_ref();
    /// let local_report_ref: ReportRef<'_, _, _, Local> = report_ref.into_local();
    /// ```
    pub fn into_local(self) -> ReportRef<'a, C, O, Local> {
        unsafe { ReportRef::from_raw(self.as_raw_ref()) }
    }

    /// Returns an iterator over the complete report hierarchy including this
    /// report.
    ///
    /// The iterator visits reports in a depth-first order: it first visits the
    /// current report, then recursively visits each child report and all of
    /// their descendants before moving to the next sibling. Unlike
    /// [`ReportRef::iter_sub_reports`], this method includes the report on
    /// which it was called as the first item in the iteration.
    ///
    /// The ownership marker of the returned iterator references matches the
    /// ownership of this report. If you need cloneable references, consider
    /// using [`ReportRef::iter_sub_reports`] instead, which only iterates
    /// over children but guarantees cloneable references.
    ///
    /// See also: [`ReportRef::iter_sub_reports`] for iterating only over child
    /// reports with cloneable references.
    ///
    /// [`Uncloneable`]: crate::markers::Uncloneable
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Uncloneable};
    /// // Create base reports
    /// let error1: Report = report!("error 1");
    /// let error2: Report = report!("error 2");
    ///
    /// // Build hierarchy using .context() which creates new nodes
    /// let with_context1 = error1.context("context for error 1");
    /// let with_context2 = error2.context("context for error 2");
    ///
    /// // Create root that contains both context nodes as children
    /// let mut root = report!("root error").context("context for root error");
    /// root.children_mut()
    ///     .push(with_context1.into_dyn_any().into_cloneable());
    /// root.children_mut()
    ///     .push(with_context2.into_dyn_any().into_cloneable());
    ///
    /// let root_ref: ReportRef<'_, &'static str, Uncloneable> = root.as_ref();
    ///
    /// let all_reports: Vec<String> = root_ref
    ///     .iter_reports()
    ///     .map(|report| report.format_current_context().to_string())
    ///     .collect();
    ///
    /// assert_eq!(all_reports[0], "context for root error"); // Current report is included
    /// assert_eq!(all_reports[1], "root error");
    /// assert_eq!(all_reports[2], "context for error 1");
    /// assert_eq!(all_reports.len(), 6);
    /// ```
    pub fn iter_reports(self) -> ReportIter<'a, O, T> {
        let stack = vec![self.as_raw_ref()];
        unsafe { ReportIter::from_raw(stack) }
    }

    /// Returns an iterator over child reports in the report hierarchy
    /// (excluding this report).
    ///
    /// The iterator visits reports in a depth-first order: it first visits the
    /// current report's children, then recursively visits each child report
    /// and all of their descendants before moving to the next sibling.
    /// Unlike [`ReportRef::iter_reports`], this method does NOT include the
    /// report on which it was called - only its descendants.
    ///
    /// This method always returns cloneable report references, making it
    /// suitable for scenarios where you need to store or pass around the
    /// report references.
    ///
    /// See also: [`ReportRef::iter_reports`] for iterating over all reports
    /// including the current one.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Uncloneable};
    /// // Create base reports
    /// let error1: Report = report!("error 1");
    /// let error2: Report = report!("error 2");
    ///
    /// // Build hierarchy using .context() which creates new nodes
    /// let with_context1 = error1.context("context for error 1");
    /// let with_context2 = error2.context("context for error 2");
    ///
    /// // Create root that contains both context nodes as children
    /// let mut root = report!("root error").context("context for root error");
    /// root.children_mut()
    ///     .push(with_context1.into_dyn_any().into_cloneable());
    /// root.children_mut()
    ///     .push(with_context2.into_dyn_any().into_cloneable());
    ///
    /// let root_ref: ReportRef<'_, &'static str, Uncloneable> = root.as_ref();
    ///
    /// let sub_reports: Vec<String> = root_ref
    ///     .iter_sub_reports()
    ///     .map(|report| report.format_current_context().to_string())
    ///     .collect();
    ///
    /// // Current "root" report is NOT included in the results
    /// assert_eq!(sub_reports[0], "root error");
    /// assert_eq!(sub_reports[1], "context for error 1");
    /// assert_eq!(sub_reports.len(), 5);
    /// ```
    pub fn iter_sub_reports(self) -> ReportIter<'a, Cloneable, T> {
        let stack = self
            .children()
            .iter()
            .map(|r| r.as_raw_ref())
            .rev()
            .collect();
        unsafe { ReportIter::from_raw(stack) }
    }

    /// Creates a new report, which has the same structure as the current
    /// report, but has all the contexts and attachments preformatted.
    ///
    /// This can be useful, as the new report is mutable because it was just
    /// created, and additionally the new report is [`Send`]+[`Sync`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, preformatted::PreformattedContext, ReportRef, markers::{Uncloneable, Mutable, SendSync, Local}};
    /// # #[derive(Default)]
    /// # struct NonSendSyncError(core::cell::Cell<()>);
    /// # let non_send_sync_error = NonSendSyncError::default();
    /// # let report = report!(non_send_sync_error);
    /// let report_ref: ReportRef<'_, NonSendSyncError, Uncloneable, Local> = report.as_ref();
    /// let preformatted: Report<PreformattedContext, Mutable, SendSync> = report_ref.preformat();
    /// assert_eq!(format!("{report}"), format!("{preformatted}"));
    /// ```
    #[track_caller]
    #[must_use]
    pub fn preformat(self) -> Report<PreformattedContext, Mutable, SendSync> {
        let preformatted_context = PreformattedContext::new_from_context(self);
        Report::from_parts_unhooked::<preformatted::PreformattedHandler>(
            preformatted_context,
            self.children()
                .iter()
                .map(|sub_report| sub_report.preformat())
                .collect(),
            self.attachments()
                .iter()
                .map(|attachment| {
                    ReportAttachment::new_custom::<preformatted::PreformattedHandler>(
                        PreformattedAttachment::new_from_attachment(attachment),
                    )
                    .into_dyn_any()
                })
                .collect(),
        )
    }

    /// Returns the [`TypeId`] of the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let type_id = report_ref.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    ///
    /// let report_ref: ReportRef<'_, dyn Any> = report_ref.into_dyn_any();
    /// let type_id = report_ref.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    /// ```
    pub fn current_context_type_id(self) -> TypeId {
        self.as_raw_ref().context_type_id()
    }

    /// Returns the [`TypeId`] of the handler used for the current context.
    ///
    /// This can be useful for debugging or introspection to understand which
    /// handler was used to format the context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::{Uncloneable, SendSync}};
    /// # use core::any::TypeId;
    /// let report = Report::new_sendsync_custom::<handlers::Debug>("error message");
    /// let report_ref: ReportRef<'_, &'static str, Uncloneable> = report.as_ref();
    /// let handler_type = report_ref.current_context_handler_type_id();
    /// assert_eq!(handler_type, TypeId::of::<handlers::Debug>());
    /// ```
    pub fn current_context_handler_type_id(&self) -> TypeId {
        self.as_raw_ref().context_handler_type_id()
    }

    /// Returns the error source if the context implements [`Error`].
    ///
    /// [`Error`]: core::error::Error
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # let report = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let source: Option<&dyn core::error::Error> = report_ref.current_context_error_source();
    /// ```
    pub fn current_context_error_source(self) -> Option<&'a (dyn core::error::Error + 'static)> {
        self.as_raw_ref().context_source()
    }

    /// Formats the current context with hook processing.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # let report = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let formatted = report_ref.format_current_context();
    /// println!("{formatted}");
    /// ```
    pub fn format_current_context(self) -> impl core::fmt::Display + core::fmt::Debug {
        let report = self.into_dyn_any().into_uncloneable().into_local();
        format_helper(
            report,
            |report, formatter| {
                crate::hooks::formatting_overrides::display_context(report, formatter)
            },
            |report, formatter| {
                crate::hooks::formatting_overrides::debug_context(report, formatter)
            },
        )
    }

    /// Formats the current context without hook processing.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # let report = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let formatted = report_ref.format_current_context_unhooked();
    /// println!("{formatted}");
    /// ```
    pub fn format_current_context_unhooked(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.as_raw_ref(),
            |report, formatter| report.context_display(formatter),
            |report, formatter| report.context_debug(formatter),
        )
    }

    /// Gets the preferred formatting style for the context with hook
    /// processing.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this context
    ///   will be embedded is being formatted using [`Display`] formatting or
    ///   [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # let report = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let style =
    ///     report_ref.preferred_context_formatting_style(handlers::FormattingFunction::Display);
    /// ```
    pub fn preferred_context_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        crate::hooks::formatting_overrides::get_preferred_context_formatting_style(
            self.into_dyn_any().into_uncloneable().into_local(),
            report_formatting_function,
        )
    }

    /// Gets the preferred formatting style for the context without hook
    /// processing.
    ///
    /// # Arguments
    ///
    /// - `report_formatting_function`: Whether the report in which this context
    ///   will be embedded is being formatted using [`Display`] formatting or
    ///   [`Debug`]
    ///
    /// [`Display`]: core::fmt::Display
    /// [`Debug`]: core::fmt::Debug
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # let report = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let style = report_ref
    ///     .preferred_context_formatting_style_unhooked(handlers::FormattingFunction::Display);
    /// ```
    pub fn preferred_context_formatting_style_unhooked(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        self.as_raw_ref()
            .preferred_context_formatting_style(report_formatting_function)
    }

    /// Returns the number of references to this report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// let mut report: Report = report!("error message");
    /// let report_ref: ReportRef<'_, _, _> = report.as_ref();
    /// assert_eq!(report_ref.strong_count(), 1); // The report has just been created, so it has a single owner
    /// ```
    pub fn strong_count(&self) -> usize {
        self.as_raw_ref().strong_count()
    }
}

impl<'a, O, T> ReportRef<'a, dyn Any, O, T>
where
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    /// Attempts to downcast the current context to a specific type.
    ///
    /// Returns `Some(&C)` if the current context is of type `C`, otherwise
    /// returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # use core::any::Any;
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, dyn Any> = report_ref.into_dyn_any();
    /// let context: Option<&MyError> = dyn_report_ref.downcast_current_context();
    /// assert!(context.is_some());
    /// ```
    pub fn downcast_current_context<C>(self) -> Option<&'a C>
    where
        C: markers::ObjectMarker,
    {
        if TypeId::of::<C>() == self.current_context_type_id() {
            // SAFETY:
            // - The context is valid because we just checked that it matches
            // - The thread marker is valid, because does not change
            Some(unsafe { self.downcast_current_context_unchecked() })
        } else {
            None
        }
    }

    /// Downcasts the current context to a specific type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current context is actually of type `C`.
    /// This can be verified by calling [`current_context_type_id()`] first.
    ///
    /// [`current_context_type_id()`]: ReportRef::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// # let report = report!(MyError).into_dyn_any().into_cloneable();
    /// let report_ref: ReportRef<'_, dyn Any> = report.as_ref();
    ///
    /// // Verify the type first
    /// if report_ref.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let context: &MyError = unsafe { report_ref.downcast_current_context_unchecked() };
    /// }
    /// ```
    pub unsafe fn downcast_current_context_unchecked<C>(self) -> &'a C
    where
        C: markers::ObjectMarker,
    {
        unsafe { self.as_raw_ref().context_downcast_unchecked::<C>() }
    }

    /// Attempts to downcast the report to a specific context type.
    ///
    /// Returns `Some(report_ref)` if the current context is of type `C`,
    /// otherwise returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # use core::any::Any;
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, dyn Any> = report_ref.into_dyn_any();
    /// let downcasted: Option<ReportRef<'_, MyError>> = dyn_report_ref.downcast_report::<MyError>();
    /// assert!(downcasted.is_some());
    /// ```
    pub fn downcast_report<C>(self) -> Option<ReportRef<'a, C, O, T>>
    where
        C: markers::ObjectMarker + ?Sized,
    {
        if TypeId::of::<C>() == TypeId::of::<dyn Any>()
            || TypeId::of::<C>() == self.current_context_type_id()
        {
            // SAFETY:
            // - The context is valid because we just checked that it matches
            // - The thread marker is valid, because does not change
            Some(unsafe { self.downcast_report_unchecked() })
        } else {
            None
        }
    }

    /// Downcasts the report to a specific context type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current context is actually of type `C`.
    /// This can be verified by calling [`current_context_type_id()`] first.
    ///
    /// [`current_context_type_id()`]: ReportRef::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef};
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// # let report = report!(MyError).into_dyn_any().into_cloneable();
    /// let report_ref: ReportRef<'_, dyn Any> = report.as_ref();
    ///
    /// // Verify the type first
    /// if report_ref.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let downcasted = unsafe { report_ref.downcast_report_unchecked::<MyError>() };
    /// }
    /// ```
    pub unsafe fn downcast_report_unchecked<C>(self) -> ReportRef<'a, C, O, T>
    where
        C: markers::ObjectMarker + ?Sized,
    {
        unsafe { ReportRef::from_raw(self.as_raw_ref()) }
    }
}

impl<'a, C, T> ReportRef<'a, C, Cloneable, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Clones the underlying [`triomphe::Arc`] of the report returning
    /// you a new [`Report`] the references the same root node.
    pub fn clone_arc(self) -> Report<C, Cloneable, T> {
        let raw = unsafe { self.as_raw_ref().clone_arc() };
        unsafe { Report::from_raw(raw) }
    }
}

impl<'a, C, O, T> Copy for ReportRef<'a, C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
}
impl<'a, C, O, T> Clone for ReportRef<'a, C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C, T> From<ReportRef<'a, C, Cloneable, T>> for Report<C, Cloneable, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(report: ReportRef<'a, C, Cloneable, T>) -> Self {
        report.clone_arc()
    }
}

impl<'a, C, O, T> core::fmt::Display for ReportRef<'a, C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let report = self.into_dyn_any().into_uncloneable().into_local();
        crate::hooks::report_formatting::format_report(report, f, FormattingFunction::Display)
    }
}

impl<'a, C, O, T> core::fmt::Debug for ReportRef<'a, C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportRefOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let report = self.into_dyn_any().into_uncloneable().into_local();
        crate::hooks::report_formatting::format_report(report, f, FormattingFunction::Debug)
    }
}

mod from_impls {
    use super::*;

    macro_rules! unsafe_reportref_to_reportref {
        ($(
            <
                $($param:ident),*
            >:
            $context1:ty => $context2:ty,
            $ownership1:ty => $ownership2:ty,
            $thread_safety1:ty => $thread_safety2:ty
        ),* $(,)?) => {
            $(
                impl<'a, $($param),*> From<ReportRef<'a, $context1, $ownership1, $thread_safety1>> for ReportRef<'a, $context2, $ownership2, $thread_safety2>
                    where $(
                        $param: markers::ObjectMarker,
                    )*
                {
                    fn from(report: ReportRef<'a, $context1, $ownership1, $thread_safety1>) -> Self {
                        // SAFETY:
                        // - The context is valid, because it either doesn't change or goes from a known `C` to `dyn Any`.
                        // - The ownership marker is valid, because it either does not change or it goes from `Cloneable` to `Uncloneable`.
                        // - The thread marker is valid, because it either does not change or it goes from `SendSync` to `Local`.
                        unsafe { ReportRef::from_raw(report.as_raw_ref()) }
                    }
                }
            )*
        };
    }

    unsafe_reportref_to_reportref!(
        <C>: C => C, Cloneable => Cloneable, SendSync => Local,
        <C>: C => C, Cloneable => Uncloneable, SendSync => SendSync,
        <C>: C => C, Cloneable => Uncloneable, SendSync => Local,
        <C>: C => C, Cloneable => Uncloneable, Local => Local,
        <C>: C => C, Uncloneable => Uncloneable, SendSync => Local,
        <C>: C => dyn Any, Cloneable => Cloneable, SendSync => SendSync,
        <C>: C => dyn Any, Cloneable => Cloneable, SendSync => Local,
        <C>: C => dyn Any, Cloneable => Cloneable, Local => Local,
        <C>: C => dyn Any, Cloneable => Uncloneable, SendSync => SendSync,
        <C>: C => dyn Any, Cloneable => Uncloneable, SendSync => Local,
        <C>: C => dyn Any, Cloneable => Uncloneable, Local => Local,
        <C>: C => dyn Any, Uncloneable => Uncloneable, SendSync => SendSync,
        <C>: C => dyn Any, Uncloneable => Uncloneable, SendSync => Local,
        <C>: C => dyn Any, Uncloneable => Uncloneable, Local => Local,
        <>:  dyn Any => dyn Any, Cloneable => Cloneable, SendSync => Local,
        <>:  dyn Any => dyn Any, Cloneable => Uncloneable, SendSync => SendSync,
        <>:  dyn Any => dyn Any, Cloneable => Uncloneable, SendSync => Local,
        <>:  dyn Any => dyn Any, Cloneable => Uncloneable, Local => Local,
        <>:  dyn Any => dyn Any, Uncloneable => Uncloneable, SendSync => Local,
    );
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;
    use crate::markers::{Mutable, Uncloneable};

    #[allow(dead_code)]
    struct NonSend(*const ());
    static_assertions::assert_not_impl_any!(NonSend: Send, Sync);

    #[test]
    fn test_report_ref_send_sync() {
        static_assertions::assert_not_impl_any!(ReportRef<'static, (), Uncloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, (), Cloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, String, Uncloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, String, Cloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, NonSend, Uncloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, NonSend, Cloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, dyn Any, Uncloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, dyn Any, Cloneable, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportRef<'static, (), Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, (), Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, String, Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, String, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, NonSend, Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, NonSend, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, dyn Any, Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, dyn Any, Cloneable, Local>: Send, Sync);
    }

    #[test]
    fn test_report_ref_copy_clone() {
        static_assertions::assert_impl_all!(ReportRef<'static, (), Uncloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, (), Cloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, (), Uncloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, (), Cloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Uncloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Cloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Uncloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Cloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Uncloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Cloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Uncloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Cloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, dyn Any, Uncloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, dyn Any, Cloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, dyn Any, Uncloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, dyn Any, Cloneable, Local>: Copy, Clone);
    }

    #[test]
    fn test_report_ref_into_report() {
        static_assertions::assert_impl_all!(Report<(), Cloneable, Local>: From<ReportRef<'static, (), Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<(), Cloneable, SendSync>: From<ReportRef<'static, (), Cloneable, SendSync>>);
        static_assertions::assert_impl_all!(Report<String, Cloneable, Local>: From<ReportRef<'static, String, Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<String, Cloneable, SendSync>: From<ReportRef<'static, String, Cloneable, SendSync>>);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, Local>: From<ReportRef<'static, NonSend, Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, SendSync>: From<ReportRef<'static, NonSend, Cloneable, SendSync>>);
        static_assertions::assert_impl_all!(Report<dyn Any, Cloneable, Local>: From<ReportRef<'static, dyn Any, Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<dyn Any, Cloneable, SendSync>: From<ReportRef<'static, dyn Any, Cloneable, SendSync>>);

        static_assertions::assert_not_impl_any!(Report<(), Mutable, Local>: From<ReportRef<'static, (), Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<(), Mutable, SendSync>: From<ReportRef<'static, (), Uncloneable, SendSync>>);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, Local>: From<ReportRef<'static, String, Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, SendSync>: From<ReportRef<'static, String, Uncloneable, SendSync>>);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, Local>: From<ReportRef<'static, NonSend, Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, SendSync>: From<ReportRef<'static, NonSend, Uncloneable, SendSync>>);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Mutable, Local>: From<ReportRef<'static, dyn Any, Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Mutable, SendSync>: From<ReportRef<'static, dyn Any, Uncloneable, SendSync>>);
    }

    #[test]
    fn test_preformat() {
        use crate::{
            ReportRef,
            markers::{Local, Mutable, SendSync, Uncloneable},
            preformatted::PreformattedContext,
            prelude::*,
        };
        #[derive(Default)]
        struct NonSendSyncError(core::cell::Cell<()>);
        let non_send_sync_error = NonSendSyncError::default();
        let report = report!(non_send_sync_error);
        let report_ref: ReportRef<'_, NonSendSyncError, Uncloneable, Local> = report.as_ref();
        let preformatted: Report<PreformattedContext, Mutable, SendSync> = report_ref.preformat();
        assert_eq!(alloc::format!("{report}"), alloc::format!("{preformatted}"));
    }
}
