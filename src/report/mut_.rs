use alloc::vec;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use rootcause_internals::{
    RawReportMut, RawReportRef,
    handlers::{ContextFormattingStyle, FormattingFunction},
};

use crate::{
    Report, ReportIter, ReportRef,
    markers::{self, Cloneable, Local, Mutable, SendSync, Uncloneable},
    preformatted::PreformattedContext,
    report_attachments::{ReportAttachmentsMut, ReportAttachmentsRef},
    report_collection::{ReportCollectionMut, ReportCollectionRef},
    util::format_helper,
};

/// A mutable reference to a [`Report`].
///
/// This type provides mutable access to a [`Report`], allowing modification of
/// children and attachments while maintaining safe borrowing semantics.
///
/// Note that unlike owned reports, mutable references cannot be consumed for
/// chaining operations like [`Report::context`] or [`Report::attach`].
///
/// [`Report`]: crate::Report
/// [`Report::context`]: crate::Report::context
/// [`Report::attach`]: crate::Report::attach
#[repr(transparent)]
pub struct ReportMut<'a, Context = dyn Any, ThreadSafety = SendSync>
where
    Context: markers::ObjectMarker + ?Sized,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: RawReportMut<'a>,
    _context: PhantomData<Context>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<'a, C, T> ReportMut<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new Report from a raw report
    ///
    /// # Safety
    ///
    /// To call this method you must ensure the following:
    ///
    /// - The context embedded in the RawReport must match the `C` of the output
    ///   type, or the `C` of the output type must be `dyn Any`
    /// - The thread safety marker must match the contents of the report. More
    ///   specifically if the marker is `SendSync`, then all contexts and
    ///   attachments must be `Send+Sync`
    pub(crate) unsafe fn from_raw(raw: RawReportMut<'a>) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> RawReportMut<'a> {
        self.raw
    }

    pub(crate) fn as_raw_ref(&self) -> RawReportRef<'_> {
        self.raw.as_ref()
    }

    /// Returns a reference to the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # struct MyError;
    /// # let my_error = MyError;
    /// # let mut report: Report<MyError> = report!(my_error);
    /// let report_mut: ReportMut<'_, MyError> = report.as_mut();
    /// let context: &MyError = report_mut.current_context();
    /// ```
    pub fn current_context(&self) -> &C
    where
        C: Sized,
    {
        self.as_ref().current_context()
    }

    /// Turns the [`ReportMut`] into a mutable reference to the current context
    /// with the same lifetime.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # let mut report: Report<String> = report!("An error occurred".to_string());
    /// let report_mut: ReportMut<'_, String> = report.as_mut();
    /// let context: &mut String = report_mut.into_current_context_mut();
    /// context.push_str(" and that's bad");
    /// ```
    pub fn into_current_context_mut(self) -> &'a mut C
    where
        C: Sized,
    {
        let raw = self.into_raw();
        unsafe { raw.into_context_downcast_unchecked() }
    }

    /// Returns a mutable reference to the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # let mut report: Report<String> = report!("An error occurred".to_string());
    /// let mut report_mut: ReportMut<'_, String> = report.as_mut();
    /// let context: &mut String = report_mut.current_context_mut();
    /// context.push_str(" and that's bad");
    /// ```
    pub fn current_context_mut(&mut self) -> &mut C
    where
        C: Sized,
    {
        self.reborrow().into_current_context_mut()
    }

    /// Returns an immutable reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_collection::ReportCollectionRef};
    /// let mut report: Report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// let children: ReportCollectionRef<'_> = report_mut.children();
    /// assert_eq!(children.len(), 0); // The report has just been created, so it has no children
    /// ```
    pub fn children(&self) -> ReportCollectionRef<'_, dyn Any, T> {
        let raw = self.as_raw_ref().children();
        unsafe { ReportCollectionRef::from_raw(raw) }
    }

    /// Returns a mutable reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_collection::ReportCollectionMut};
    /// # let mut report = report!("error message");
    /// let mut report_mut: ReportMut<'_> = report.as_mut();
    /// let children_mut: ReportCollectionMut<'_> = report_mut.children_mut();
    /// ```
    pub fn children_mut(&mut self) -> ReportCollectionMut<'_, dyn Any, T> {
        let raw = self.raw.reborrow().into_children_mut();
        unsafe { ReportCollectionMut::from_raw(raw) }
    }

    /// Returns an immutable reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_attachments::ReportAttachmentsRef};
    /// # let mut report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// let attachments: ReportAttachmentsRef<'_> = report_mut.attachments();
    /// ```
    pub fn attachments(&self) -> ReportAttachmentsRef<'_, T> {
        let raw = self.as_raw_ref().attachments();
        unsafe { ReportAttachmentsRef::from_raw(raw) }
    }

    /// Returns a mutable reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_attachments::ReportAttachmentsMut};
    /// # let mut report = report!("error message");
    /// let mut report_mut: ReportMut<'_> = report.as_mut();
    /// let attachments_mut: ReportAttachmentsMut<'_> = report_mut.attachments_mut();
    /// ```
    pub fn attachments_mut(&mut self) -> ReportAttachmentsMut<'_, T> {
        let raw = self.raw.reborrow().into_attachments_mut();
        unsafe { ReportAttachmentsMut::from_raw(raw) }
    }

    /// Changes the context type of the [`ReportMut`] to [`dyn Any`].
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
    /// [`ReportMut::downcast_report`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # use core::any::Any;
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let report: ReportMut<'_, MyError> = report.as_mut();
    /// let local_report: ReportMut<'_, dyn Any> = report.into_dyn_any();
    /// ```
    pub fn into_dyn_any(self) -> ReportMut<'a, dyn Any, T> {
        unsafe { ReportMut::from_raw(self.into_raw()) }
    }

    /// Returns an immutable reference to the report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, ReportRef};
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let report_mut: ReportMut<'_, MyError> = report.as_mut();
    /// let report_ref: ReportRef<'_, MyError, markers::Uncloneable> = report_mut.as_ref();
    /// ```
    pub fn as_ref(&self) -> ReportRef<'_, C, Uncloneable, T> {
        unsafe { ReportRef::from_raw(self.as_raw_ref()) }
    }

    /// Consumes the [`ReportMut`] and returns a [`ReportRef`] with same
    /// lifetime.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, ReportRef};
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let report_mut: ReportMut<'_, MyError> = report.as_mut();
    /// let report_ref: ReportRef<'_, MyError, markers::Uncloneable> = report_mut.into_ref();
    /// ```
    pub fn into_ref(self) -> ReportRef<'a, C, Uncloneable, T> {
        unsafe { ReportRef::from_raw(self.raw.into_ref()) }
    }

    /// Reborrows the [`ReportMut`] to return a new [`ReportMut`] with a shorter
    /// lifetime
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let mut report_mut: ReportMut<'_, MyError> = report.as_mut();
    /// {
    ///     // Create a new mutable report with a shorter
    ///     let mut borrowed_report_mut: ReportMut<'_, MyError> = report_mut.reborrow();
    /// }
    /// // After dropping the inner reference report, we can still use the outer one
    /// let _context: &MyError = report_mut.current_context();
    /// ```
    pub fn reborrow(&mut self) -> ReportMut<'_, C, T> {
        // SAFETY: The reborrow does not change the context or thread safety markers
        unsafe { ReportMut::from_raw(self.raw.reborrow()) }
    }

    /// Returns an iterator over the complete report hierarchy including this
    /// report.
    ///
    /// The iterator visits reports in a depth-first order: it first visits the
    /// current report, then recursively visits each child report and all of
    /// their descendants before moving to the next sibling. Unlike
    /// [`ReportMut::iter_sub_reports`], this method includes the report on
    /// which it was called as the first item in the iteration.
    ///
    /// Since this is a mutable reference, the returned iterator references are
    /// [`Uncloneable`] to ensure proper borrowing semantics.
    ///
    /// See also: [`ReportMut::iter_sub_reports`] for iterating only over child
    /// reports with cloneable references.
    ///
    /// [`Uncloneable`]: crate::markers::Uncloneable
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
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
    /// let root_mut: ReportMut<'_, &'static str> = root.as_mut();
    ///
    /// let all_reports: Vec<String> = root_mut
    ///     .iter_reports()
    ///     .map(|report| report.format_current_context().to_string())
    ///     .collect();
    ///
    /// assert_eq!(all_reports[0], "context for root error"); // Current report is included
    /// assert_eq!(all_reports[1], "root error");
    /// assert_eq!(all_reports[2], "context for error 1");
    /// assert_eq!(all_reports.len(), 6);
    /// ```
    pub fn iter_reports(&self) -> ReportIter<'_, Uncloneable, T> {
        let stack = vec![self.as_raw_ref()];
        unsafe { ReportIter::from_raw(stack) }
    }

    /// Returns an iterator over child reports in the report hierarchy
    /// (excluding this report).
    ///
    /// The iterator visits reports in a depth-first order: it first visits the
    /// current report's children, then recursively visits each child report
    /// and all of their descendants before moving to the next sibling.
    /// Unlike [`ReportMut::iter_reports`], this method does NOT include the
    /// report on which it was called - only its descendants.
    ///
    /// This method always returns cloneable report references, making it
    /// suitable for scenarios where you need to store or pass around the
    /// report references.
    ///
    /// See also: [`ReportMut::iter_reports`] for iterating over all reports
    /// including the current one.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # use core::any::Any;
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
    /// let root_mut: ReportMut<'_, &'static str> = root.as_mut();
    ///
    /// let sub_reports: Vec<String> = root_mut
    ///     .iter_sub_reports()
    ///     .map(|report| report.format_current_context().to_string())
    ///     .collect();
    ///
    /// // Current "root" report is NOT included in the results
    /// assert_eq!(sub_reports[0], "root error");
    /// assert_eq!(sub_reports[1], "context for error 1");
    /// assert_eq!(sub_reports.len(), 5);
    /// ```
    pub fn iter_sub_reports(&self) -> ReportIter<'_, Cloneable, T> {
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
    /// # use rootcause::{prelude::*, ReportMut, preformatted::PreformattedContext};
    /// # #[derive(Default)]
    /// # struct NonSendSyncError(core::cell::Cell<()>);
    /// # let non_send_sync_error = NonSendSyncError::default();
    /// # let mut report = report!(non_send_sync_error);
    /// let report_mut: ReportMut<'_, NonSendSyncError, markers::Local> = report.as_mut();
    /// let preformatted: Report<PreformattedContext, markers::Mutable, markers::SendSync> =
    ///     report.preformat();
    /// assert_eq!(format!("{report}"), format!("{preformatted}"));
    /// ```
    #[track_caller]
    #[must_use]
    pub fn preformat(&self) -> Report<PreformattedContext, Mutable, SendSync> {
        self.as_ref().preformat()
    }

    /// Returns the [`TypeId`] of the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut};
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// # let mut report = report!(MyError);
    /// let report_mut: ReportMut<'_, MyError> = report.as_mut();
    /// let type_id = report_mut.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    ///
    /// let report_mut: ReportMut<'_, dyn Any> = report_mut.into_dyn_any();
    /// let type_id = report_mut.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    /// ```
    pub fn current_context_type_id(&self) -> TypeId {
        self.as_raw_ref().context_type_id()
    }

    /// Returns the [`TypeId`] of the handler used for the current context.
    ///
    /// This can be useful for debugging or introspection to understand which
    /// handler was used to format the context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::TypeId;
    /// let mut report = Report::new_sendsync_custom::<handlers::Debug>("error message");
    /// let report_mut = report.as_mut();
    /// let handler_type = report_mut.current_context_handler_type_id();
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
    /// # use rootcause::prelude::*;
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let source = report_mut.current_context_error_source();
    /// assert!(source.is_none()); // The context does not implement Error, so no source
    /// ```
    pub fn current_context_error_source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.as_raw_ref().context_source()
    }

    /// Formats the current context with hook processing.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let formatted = report_mut.format_current_context();
    /// println!("{formatted}");
    /// ```
    pub fn format_current_context(&self) -> impl core::fmt::Display + core::fmt::Debug {
        let report = self.as_ref().into_dyn_any().into_uncloneable().into_local();
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
    /// # use rootcause::prelude::*;
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let formatted = report_mut.format_current_context_unhooked();
    /// println!("{formatted}");
    /// ```
    pub fn format_current_context_unhooked(&self) -> impl core::fmt::Display + core::fmt::Debug {
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
    /// # use rootcause::prelude::*;
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let style =
    ///     report_mut.preferred_context_formatting_style(handlers::FormattingFunction::Display);
    /// ```
    pub fn preferred_context_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let report = self.as_ref().into_dyn_any().into_uncloneable().into_local();
        crate::hooks::formatting_overrides::get_preferred_context_formatting_style(
            report,
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
    /// # use rootcause::prelude::*;
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let style = report_mut
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
    /// # use rootcause::prelude::*;
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// assert_eq!(report_mut.strong_count(), 1);
    /// ```
    pub fn strong_count(&self) -> usize {
        self.as_raw_ref().strong_count()
    }
}

impl<'a, T> ReportMut<'a, dyn Any, T>
where
    T: markers::ThreadSafetyMarker,
{
    /// Attempts to downcast the current context to a specific type.
    ///
    /// Returns `Some(&C)` if the current context is of type `C`, otherwise
    /// returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let mut dyn_report: Report = report.into_dyn_any();
    /// let mut_report = dyn_report.as_mut();
    /// let context: Option<&MyError> = mut_report.downcast_current_context();
    /// assert!(context.is_some());
    /// ```
    pub fn downcast_current_context<C>(&self) -> Option<&C>
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
    /// [`current_context_type_id()`]: ReportMut::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let mut dyn_report: Report = report.into_dyn_any();
    /// let mut_report = dyn_report.as_mut();
    ///
    /// // Verify the type first
    /// if mut_report.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let context: &MyError = unsafe { mut_report.downcast_current_context_unchecked() };
    /// }
    /// ```
    pub unsafe fn downcast_current_context_unchecked<C>(&self) -> &C
    where
        C: markers::ObjectMarker,
    {
        unsafe { self.as_raw_ref().context_downcast_unchecked::<C>() }
    }

    /// Attempts to downcast the entire report to a specific context type.
    ///
    /// Returns `Ok(ReportMut<C>)` if the current context is of type `C`,
    /// otherwise returns `Err(self)` with the original report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let mut dyn_report: Report = report.into_dyn_any();
    /// let mut_report = dyn_report.as_mut();
    /// let downcasted: Result<_, _> = mut_report.downcast_report::<MyError>();
    /// assert!(downcasted.is_ok());
    /// ```
    pub fn downcast_report<C>(self) -> Result<ReportMut<'a, C, T>, ReportMut<'a, dyn Any, T>>
    where
        C: markers::ObjectMarker + ?Sized,
    {
        if TypeId::of::<C>() == TypeId::of::<dyn Any>()
            || TypeId::of::<C>() == self.current_context_type_id()
        {
            // SAFETY:
            // - The context is valid because we just checked that it matches
            // - The thread marker is valid, because does not change
            Ok(unsafe { self.downcast_report_unchecked() })
        } else {
            Err(self)
        }
    }

    /// Downcasts the entire report to a specific context type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current context is actually of type `C`.
    /// This can be verified by calling [`current_context_type_id()`] first.
    ///
    /// [`current_context_type_id()`]: ReportMut::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let mut dyn_report: Report = report.into_dyn_any();
    /// let mut_report = dyn_report.as_mut();
    ///
    /// // Verify the type first
    /// if mut_report.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let downcasted = unsafe { mut_report.downcast_report_unchecked::<MyError>() };
    /// }
    /// ```
    pub unsafe fn downcast_report_unchecked<C>(self) -> ReportMut<'a, C, T>
    where
        C: markers::ObjectMarker + ?Sized,
    {
        unsafe { ReportMut::from_raw(self.into_raw()) }
    }
}

impl<'a, C, T> core::fmt::Display for ReportMut<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.as_ref(), f)
    }
}

impl<'a, C, T> core::fmt::Debug for ReportMut<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), f)
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
            $thread_safety:ty
        ),* $(,)?) => {
            $(
                impl<'a, $($param),*> From<ReportMut<'a, $context1, $thread_safety>> for ReportMut<'a, $context2, $thread_safety>
                    where $(
                        $param: markers::ObjectMarker,
                    )* {
                    fn from(report: ReportMut<'a, $context1, $thread_safety>) -> Self {
                        // SAFETY:
                        // - The context is valid, because it either doesn't change or goes from a known `C` to `dyn Any`.
                        // - The thread marker is valid, because it either does not change or it goes from `SendSync` to `Local`.
                        unsafe { ReportMut::from_raw(report.into_raw()) }
                    }
                }
            )*
        };
    }

    // NOTE: A mutable report reference is not variant over thread safety.
    //
    // * If you allow a SendSync => Local conversion, then you permit adding local
    //   attachments to the root.
    // * If you allow a Local => SendSync conversion, then you permit cloning a
    //   subreport that contains local data and sending it to another thread
    unsafe_reportref_to_reportref!(
        <C>: C => dyn Any, SendSync,
        <C>: C => dyn Any, Local,
    );
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[allow(dead_code)]
    struct NonSend(*const ());
    static_assertions::assert_not_impl_any!(NonSend: Send, Sync);

    #[test]
    fn test_report_mut_send_sync() {
        static_assertions::assert_not_impl_any!(ReportMut<'static, (), SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportMut<'static, String, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportMut<'static, NonSend, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportMut<'static, dyn Any, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportMut<'static, (), Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportMut<'static, String, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportMut<'static, NonSend, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportMut<'static, dyn Any, Local>: Send, Sync);
    }

    #[test]
    fn test_report_mut_copy_clone() {
        static_assertions::assert_not_impl_any!(ReportMut<'static, (), SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, (), Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, String, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, String, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, NonSend, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, NonSend, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, dyn Any, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(ReportMut<'static, dyn Any, Local>: Copy, Clone);
    }
}
