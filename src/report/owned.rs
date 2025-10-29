use alloc::vec;
use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use rootcause_internals::{
    RawReport, RawReportRef,
    handlers::{ContextFormattingStyle, FormattingFunction},
};

use crate::{
    ReportIter, ReportMut, ReportRef,
    handlers::{self, ContextHandler},
    markers::{self, Cloneable, Local, Mutable, SendSync, Uncloneable},
    preformatted::PreformattedContext,
    report_attachment::ReportAttachment,
    report_attachments::{ReportAttachments, ReportAttachmentsMut, ReportAttachmentsRef},
    report_collection::{ReportCollection, ReportCollectionMut, ReportCollectionRef},
    util::format_helper,
};

/// An error report that contains a context, child reports, and attachments.
///
/// # Examples
/// ```
/// # use rootcause::prelude::*;
/// let report: Report = report!("file missing");
/// println!("{report}");
/// ```
#[repr(transparent)]
pub struct Report<Context = dyn Any, Ownership = Mutable, ThreadSafety = SendSync>
where
    Context: markers::ObjectMarker + ?Sized,
    Ownership: markers::ReportOwnershipMarker,
    ThreadSafety: markers::ThreadSafetyMarker,
{
    raw: RawReport,
    _context: PhantomData<Context>,
    _ownership: PhantomData<Ownership>,
    _thread_safety: PhantomData<ThreadSafety>,
}

impl<C, T> Report<C, Mutable, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new [`Report`] with the given context.
    ///
    /// This method is generic over the thread safety marker `T`. The context
    /// will use the [`handlers::Error`] handler for formatting.
    ///
    /// See also:
    ///
    /// - The [`report!()`] macro will also create a new report, but can
    ///   auto-detect the thread safety marker and handler.
    /// - The [`Report::new_sendsync`] and [`Report::new_local`] are more
    ///   restrictive variants of this function that might help avoid type
    ///   inference issues.
    /// - The [`Report::new_custom`] methods also allows you to manually specify
    ///   the handler.
    ///
    /// [`report!()`]: crate::report!
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::{SendSync, Mutable, Local}};
    /// # #[derive(Debug)]
    /// # struct MyError;
    /// # impl core::error::Error for MyError {}
    /// # impl core::fmt::Display for MyError { fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { unimplemented!() }}
    /// let report_sendsync: Report<MyError, Mutable, SendSync> = Report::new(MyError);
    /// let report_local: Report<MyError, Mutable, Local> = Report::new(MyError);
    /// ```
    #[track_caller]
    #[must_use]
    pub fn new(context: C) -> Self
    where
        C: core::error::Error + markers::ObjectMarkerFor<T> + Sized,
    {
        Self::new_custom::<handlers::Error>(context)
    }

    /// Creates a new [`Report`] with the given context and handler.
    ///
    /// This method is generic over the thread safety marker `T`.
    ///
    /// If you're having trouble with type inference for the thread safety
    /// parameter, consider using [`Report::new_sendsync_custom`] or
    /// [`Report::new_local_custom`] instead.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::{SendSync, Local}};
    /// # #[derive(Debug)]
    /// # struct MyError;
    /// # impl core::error::Error for MyError {}
    /// # impl core::fmt::Display for MyError { fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { unimplemented!() }}
    /// let report_sendsync: Report<MyError> = Report::new_custom::<handlers::Debug>(MyError);
    /// let report_local: Report<MyError> = Report::new_custom::<handlers::Display>(MyError);
    /// ```
    #[track_caller]
    #[must_use]
    pub fn new_custom<H>(context: C) -> Self
    where
        C: markers::ObjectMarkerFor<T> + Sized,
        H: ContextHandler<C>,
    {
        Self::from_parts::<H>(context, ReportCollection::new(), ReportAttachments::new())
    }

    /// Creates a new [`Report`] with the given context, children, and
    /// attachments.
    ///
    /// This method processes hooks during report creation. If you want to skip
    /// hook processing, use [`Report::from_parts_unhooked`] instead.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_collection::ReportCollection, report_attachments::ReportAttachments, markers::{SendSync}};
    /// let report: Report<String, _, SendSync> = Report::from_parts::<handlers::Display>(
    ///     "error".to_string(),
    ///     ReportCollection::new(),
    ///     ReportAttachments::new(),
    /// );
    /// ```
    #[track_caller]
    #[must_use]
    pub fn from_parts<H>(
        context: C,
        children: ReportCollection<dyn Any, T>,
        attachments: ReportAttachments<T>,
    ) -> Self
    where
        C: markers::ObjectMarkerFor<T> + Sized,
        H: ContextHandler<C>,
    {
        let mut report: Self = Self::from_parts_unhooked::<H>(context, children, attachments);
        crate::hooks::report_creation::__run_creation_hooks(report.as_mut().into_dyn_any());
        report
    }

    /// Creates a new [`Report`] with the given context, children, and
    /// attachments without hook processing.
    ///
    /// This method skips hook processing during report creation. If you want
    /// hooks to be processed, use [`Report::from_parts`] instead.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_collection::ReportCollection, report_attachments::ReportAttachments, markers::SendSync};
    /// let report: Report<String, _, SendSync> = Report::from_parts_unhooked::<handlers::Display>(
    ///     "error".to_string(),
    ///     ReportCollection::new(),
    ///     ReportAttachments::new(),
    /// );
    /// ```
    #[track_caller]
    #[must_use]
    pub fn from_parts_unhooked<H>(
        context: C,
        children: ReportCollection<dyn Any, T>,
        attachments: ReportAttachments<T>,
    ) -> Self
    where
        C: markers::ObjectMarkerFor<T> + Sized,
        H: ContextHandler<C>,
    {
        // SAFETY:
        // - The context matches the `C` in the output, because we just created the raw
        //   report
        // - The entire report has only a single reference, because we just created it
        // - The entire report only contains `Send+Sync` data, as the bounds on this
        //   method requires `C: Send+Sync` and our attachments have been marked as
        //   `SendSync` as well
        unsafe {
            Report::from_raw(RawReport::new::<C, H>(
                context,
                children.into_raw(),
                attachments.into_raw(),
            ))
        }
    }

    /// Decomposes the [`Report`] into its constituent parts.
    ///
    /// Returns a tuple containing the children collection, attachments
    /// collection, and context in that order. This is the inverse operation
    /// of [`Report::from_parts`] and [`Report::from_parts_unhooked`].
    ///
    /// This method can be useful when you need to:
    /// - Extract and modify individual components of a report
    /// - Rebuild a report with different components
    /// - Transfer components between different reports
    /// - Perform custom processing on specific parts
    ///
    /// Note that to exactly reconstruct the original report, you will also need
    /// to use the same handler as was used for the original report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_collection::ReportCollection, report_attachments::ReportAttachments};
    /// // Create a report with some children and attachments
    /// let mut report: Report<String> = Report::from_parts::<handlers::Display>(
    ///     "main error".to_string(),
    ///     ReportCollection::new(),
    ///     ReportAttachments::new(),
    /// );
    ///
    /// // Add some content
    /// let child_report = report!("child error").into_dyn_any().into_cloneable();
    /// report.children_mut().push(child_report);
    /// report.attachments_mut().push("debug info".into());
    ///
    /// // Decompose into parts
    /// let (context, children, attachments) = report.into_parts();
    ///
    /// assert_eq!(context, "main error");
    /// assert_eq!(children.len(), 1);
    /// assert!(attachments.len() >= 1); // "debug info" + potential automatic attachments
    ///
    /// // Can rebuild with the same or different parts
    /// let rebuilt: Report<String> = Report::from_parts::<handlers::Display>(
    ///     context,
    ///     children,
    ///     attachments,
    /// );
    /// ```
    pub fn into_parts(self) -> (C, ReportCollection<dyn Any, T>, ReportAttachments<T>)
    where
        C: Sized,
    {
        let (context, children, attachments) = unsafe { self.raw.into_parts() };
        let children = unsafe { ReportCollection::from_raw(children) };
        let attachments = unsafe { ReportAttachments::from_raw(attachments) };
        (context, children, attachments)
    }

    /// Extracts and returns the context value from the [`Report`].
    ///
    /// This is a convenience method that consumes the [`Report`] and returns
    /// only the context, discarding the children and attachments. It's
    /// equivalent to calling `report.into_parts().0`.
    ///
    /// This method can be useful when:
    /// - You only need the underlying error or context value
    /// - Converting from a [`Report`] back to the original error type
    /// - Extracting context for logging or forwarding to other systems
    /// - Implementing error conversion traits
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// #[derive(Debug, PartialEq, Clone)]
    /// struct MyError {
    ///     message: String,
    ///     code: u32,
    /// }
    ///
    /// impl std::fmt::Display for MyError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "Error {}: {}", self.code, self.message)
    ///     }
    /// }
    ///
    /// impl std::error::Error for MyError {}
    ///
    /// // Create a report with a custom error context
    /// let original_error = MyError {
    ///     message: "database connection failed".to_string(),
    ///     code: 500,
    /// };
    /// let report: Report<MyError> = report!(original_error.clone());
    ///
    /// // Extract just the context, discarding report structure
    /// let extracted_error = report.into_current_context();
    /// assert_eq!(extracted_error, original_error);
    /// assert_eq!(extracted_error.code, 500);
    /// assert_eq!(extracted_error.message, "database connection failed");
    /// ```
    pub fn into_current_context(self) -> C
    where
        C: Sized,
    {
        self.into_parts().0
    }

    /// Adds a new attachment to the [`Report`].
    ///
    /// This is a convenience method used for chaining method calls; it consumes
    /// the [`Report`] and returns it.
    ///
    /// If you want more direct control over the attachments, you can use the
    /// [`Report::attachments_mut`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("error message");
    /// let with_attachment = report.attach("additional info");
    /// ```
    #[must_use]
    pub fn attach<A>(mut self, attachment: A) -> Self
    where
        A: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
    {
        self.attachments_mut()
            .push(ReportAttachment::new(attachment).into_dyn_any());
        self
    }

    /// Adds a new attachment to the [`Report`].
    ///
    /// This is a convenience method used for chaining method calls; it consumes
    /// the [`Report`] and returns it.
    ///
    /// If you want more direct control over the attachments, you can use the
    /// [`Report::attachments_mut`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("error message");
    /// let with_attachment = report.attach_custom::<handlers::Display, _>("info");
    /// ```
    #[must_use]
    pub fn attach_custom<H, A>(mut self, attachment: A) -> Self
    where
        A: markers::ObjectMarkerFor<T>,
        H: handlers::AttachmentHandler<A>,
    {
        self.attachments_mut()
            .push(ReportAttachment::new_custom::<H>(attachment).into_dyn_any());
        self
    }

    /// Returns a mutable reference to the current context.
    ///
    /// # Examples
    /// ```
    /// use rootcause::prelude::*;
    /// let mut report: Report<String> = report!(String::from("An error occurred"));
    /// let context: &mut String = report.current_context_mut();
    /// context.push_str(" and that's bad");
    /// ```
    #[must_use]
    pub fn current_context_mut(&mut self) -> &mut C
    where
        C: Sized,
    {
        self.as_mut().into_current_context_mut()
    }

    /// Returns a mutable reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_collection::ReportCollectionMut};
    /// let mut report: Report = report!("error message");
    /// let children_mut: ReportCollectionMut<'_> = report.children_mut();
    /// ```
    pub fn children_mut(&mut self) -> ReportCollectionMut<'_, dyn Any, T> {
        let raw = unsafe { self.raw.as_mut().into_children_mut() };
        unsafe { ReportCollectionMut::from_raw(raw) }
    }

    /// Returns a mutable reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_attachments::ReportAttachmentsMut};
    /// let mut report: Report = report!("error message");
    /// let attachments_mut: ReportAttachmentsMut<'_> = report.attachments_mut();
    /// ```
    pub fn attachments_mut(&mut self) -> ReportAttachmentsMut<'_, T> {
        let raw = unsafe { self.raw.as_mut().into_attachments_mut() };
        unsafe { ReportAttachmentsMut::from_raw(raw) }
    }

    /// Returns a mutable reference to the report.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{ReportMut, prelude::*};
    /// let mut report: Report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// ```
    pub fn as_mut(&mut self) -> ReportMut<'_, C, T> {
        // SAFETY: We are guaranteed that the report is mutable, because we are in a
        // `Mutable` report and we have a mutable reference to it.
        unsafe { ReportMut::from_raw(self.raw.as_mut()) }
    }
}

impl<C, O, T> Report<C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new [`Report`] from a [`RawReport`]
    ///
    /// # Safety
    ///
    /// To call this method you must ensure the following:
    ///
    /// - The context embedded in the [`RawReport`] must either be the type `C`,
    ///   or `C` must be the type `dyn Any`
    /// - The ownership marker must match the actual ownership status of the
    ///   report. More specifically, if the ownership mode is [`Mutable`], then
    ///   no other references my exist to the report itself, but references to
    ///   sub-reports are allowed.
    /// - The thread safety marker must match the contents of the report. More
    ///   specifically if the marker is [`SendSync`], then all contexts and
    ///   attachments must be [`Send`]+[`Sync`]
    pub(crate) unsafe fn from_raw(raw: RawReport) -> Self {
        Self {
            raw,
            _context: PhantomData,
            _ownership: PhantomData,
            _thread_safety: PhantomData,
        }
    }

    /// Consumes the [`Report`] and returns the inner [`RawReport`].
    pub(crate) fn into_raw(self) -> RawReport {
        self.raw
    }

    /// Creates a lifetime-bound [`RawReportRef`] from the inner [`RawReport`].
    pub(crate) fn as_raw_ref(&self) -> RawReportRef<'_> {
        self.raw.as_ref()
    }

    /// Returns a reference to the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # struct MyError;
    /// # let my_error = MyError;
    /// let report: Report<MyError> = report!(my_error);
    /// let context: &MyError = report.current_context();
    /// ```
    #[must_use]
    pub fn current_context(&self) -> &C
    where
        C: Sized,
    {
        self.as_ref().current_context()
    }

    /// Creates a new [`Report`] with the given context and sets the current
    /// report as a child of the new report.
    ///
    /// The new context will use the [`handlers::Display`] handler to format the
    /// context.
    ///
    /// This is a convenience method used for chaining method calls; it consumes
    /// the [`Report`] and returns it.
    ///
    /// If you want a different context handler, you can use
    /// [`Report::context_custom`].
    ///
    /// If you want to more directly control the allocation of the new report,
    /// you can use [`Report::from_parts`], which is the underlying method
    /// used to implement this method.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("initial error");
    /// let contextual_report: Report<&str> = report.context("additional context");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context<D>(self, context: D) -> Report<D, Mutable, T>
    where
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
    {
        self.context_custom::<handlers::Display, _>(context)
    }

    /// Creates a new [`Report`] with the given context and sets the current
    /// report as a child of the new report.
    ///
    /// This is a convenience method used for chaining method calls; it consumes
    /// the [`Report`] and returns it.
    ///
    /// If you want to more directly control the allocation of the new report,
    /// you can use [`Report::from_parts`], which is the underlying method
    /// used to implement this method.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("initial error");
    /// let contextual_report: Report<&str> = report.context_custom::<handlers::Debug, _>("context");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context_custom<H, D>(self, context: D) -> Report<D, Mutable, T>
    where
        D: markers::ObjectMarkerFor<T>,
        H: ContextHandler<D>,
    {
        Report::from_parts::<H>(
            context,
            ReportCollection::from([self.into_dyn_any().into_cloneable()]),
            ReportAttachments::new(),
        )
    }

    /// Returns a reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_collection::ReportCollectionRef};
    /// let report: Report = report!("error message");
    /// let children: ReportCollectionRef<'_> = report.children();
    /// assert_eq!(children.len(), 0); // The report has just been created, so it has no children
    /// ```
    pub fn children(&self) -> ReportCollectionRef<'_, dyn Any, T> {
        let raw = self.as_raw_ref().children();
        unsafe { ReportCollectionRef::from_raw(raw) }
    }

    /// Returns a reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_attachments::ReportAttachmentsRef};
    /// let report: Report = report!("error message");
    /// let attachments: ReportAttachmentsRef<'_> = report.attachments();
    /// ```
    pub fn attachments(&self) -> ReportAttachmentsRef<'_, T> {
        let raw = self.as_raw_ref().attachments();
        unsafe { ReportAttachmentsRef::from_raw(raw) }
    }

    /// Changes the context type of the [`Report`] to [`dyn Any`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the context mode to
    /// `dyn Any`.
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that the context actually has the type
    /// `C`.
    ///
    /// To get back the report with a concrete `C` you can use the method
    /// [`Report::downcast_report`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::Any;
    /// # struct MyError;
    /// # let my_error = MyError;
    /// let report: Report<MyError> = report!(my_error);
    /// let dyn_report: Report<dyn Any> = report.into_dyn_any();
    /// ```
    #[must_use]
    pub fn into_dyn_any(self) -> Report<dyn Any, O, T> {
        unsafe { Report::from_raw(self.into_raw()) }
    }

    /// Changes the ownership of the [`Report`] to [`Cloneable`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the ownership mode to
    /// [`Cloneable`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that the [`Report`] only has a single
    /// owner.
    ///
    /// After calling this method, you can clone the [`Report`], but you can no
    /// longer add attachments to the [`Report`] or otherwise modify the
    /// root node.
    ///
    /// To get back a [`Mutable`] you need to either:
    /// - Allocate a new root node using e.g. [`Report::context`].
    /// - If there is a single unique owner of the report, you can use
    ///   [`Report::try_into_mutable`].
    /// - Preformat the root node using [`Report::preformat`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::{Mutable, Cloneable}};
    /// # let my_error = "error message";
    /// let report: Report<_, Mutable> = report!(my_error);
    /// let cloneable_report: Report<_, Cloneable> = report.into_cloneable();
    /// let cloned = cloneable_report.clone();
    /// ```
    #[must_use]
    pub fn into_cloneable(self) -> Report<C, Cloneable, T> {
        unsafe { Report::from_raw(self.into_raw()) }
    }

    /// Changes the thread safety mode of the [`Report`] to [`Local`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the thread safety
    /// mode to [`Local`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that all objects in the [`Report`] might
    /// actually be [`Send`] and [`Sync`].
    ///
    /// After calling this method, you can add objects to the [`Report`] that
    /// neither [`Send`] nor [`Sync`], but the report itself will no longer
    /// be [`Send`]+[`Sync`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::{Local, SendSync}};
    /// # let my_error = "error message";
    /// let report: Report<_, _, SendSync> = report!(my_error);
    /// let local_report: Report<_, _, Local> = report.into_local();
    /// ```
    #[must_use]
    pub fn into_local(self) -> Report<C, O, Local> {
        unsafe { Report::from_raw(self.into_raw()) }
    }

    /// Checks if there is only a single unique owner of the root node of the
    /// [`Report`].
    ///
    /// If there is only a single unique owner of the [`Report`], this method
    /// marks the current report as [`Mutable`] report and returns,
    /// otherwise it gives back the current report.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of checking for unique ownership and returns the same
    /// report (with different type parameters) no matter the outcome of the
    /// check.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::{Mutable, Cloneable}};
    /// # let some_report = report!("error message").into_cloneable();
    /// let cloneable: Report<_, Cloneable> = some_report;
    /// match cloneable.try_into_mutable() {
    ///     Ok(mutable) => println!("Converted to mutable"),
    ///     Err(cloneable) => println!("Still cloneable"),
    /// }
    /// ```
    pub fn try_into_mutable(self) -> Result<Report<C, Mutable, T>, Report<C, O, T>> {
        if self.as_raw_ref().strong_count() == 1 {
            unsafe { Ok(Report::from_raw(self.into_raw())) }
        } else {
            Err(self)
        }
    }

    /// Returns an immutable reference to the report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::{Cloneable, Mutable, Uncloneable}};
    /// let report: Report<_, Mutable> = report!("error message");
    /// let report_ref: ReportRef<'_, _, Uncloneable> = report.as_ref();
    ///
    /// let report: Report<_, Cloneable> = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_, _, Cloneable> = report.as_ref();
    /// ```
    pub fn as_ref(&self) -> ReportRef<'_, C, O::RefMarker, T> {
        unsafe { ReportRef::from_raw(self.as_raw_ref()) }
    }

    /// Returns an iterator over the complete report hierarchy including this
    /// report.
    ///
    /// The iterator visits reports in a depth-first order: it first visits the
    /// current report, then recursively visits each child report and all of
    /// their descendants before moving to the next sibling. Unlike
    /// [`Report::iter_sub_reports`], this method includes the report on
    /// which it was called as the first item in the iteration.
    ///
    /// The ownership marker of the returned iterator references matches the
    /// ownership of this report. For mutable reports, the references may
    /// not be cloneable, which can limit how you can use them. If you need
    /// cloneable references, consider using [`Report::iter_sub_reports`]
    /// instead, which only iterates over children but guarantees
    /// cloneable references.
    ///
    /// See also: [`Report::iter_sub_reports`] for iterating only over child
    /// reports with cloneable references.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// // Create base reports
    /// let error1: Report = report!("error 1");
    /// let error2: Report = report!("error 2");
    ///
    /// // Build hierarchy using .context() which creates new nodes
    /// let with_context1 = error1.context("context for error 1");  // Creates new node with error1 as child
    /// let with_context2 = error2.context("context for error 2");  // Creates new node with error2 as child
    ///
    /// // Create root that contains both context nodes as children
    /// let mut root = report!("root error").context("context for root error");
    /// root.children_mut().push(with_context1.into_dyn_any().into_cloneable());
    /// root.children_mut().push(with_context2.into_dyn_any().into_cloneable());
    ///
    /// // At this point our report tree looks like this:
    /// // - context for root error
    /// //   - root error
    /// //   - context for error 1
    /// //     - error 1
    /// //   - context for error 2
    /// //     - error 2
    ///
    /// let all_reports: Vec<String> = root
    ///     .iter_reports()
    ///     .map(|report| report.format_current_context().to_string())
    ///     .collect();
    ///
    /// assert_eq!(all_reports[0], "context for root error");  // Current report is included
    /// assert_eq!(all_reports[1], "root error");
    /// assert_eq!(all_reports.len(), 6);
    /// ```
    pub fn iter_reports(&self) -> ReportIter<'_, O::RefMarker, T> {
        let stack = vec![self.as_raw_ref()];
        unsafe { ReportIter::from_raw(stack) }
    }

    /// Returns an iterator over child reports in the report hierarchy
    /// (excluding this report).
    ///
    /// The iterator visits reports in a depth-first order: it first visits the
    /// current report's children, then recursively visits each child report
    /// and all of their descendants before moving to the next sibling.
    /// Unlike [`Report::iter_reports`], this method does NOT include the
    /// report on which it was called - only its descendants.
    ///
    /// This method always returns cloneable report references, making it
    /// suitable for scenarios where you need to store or pass around the
    /// report references. This is different from [`Report::iter_reports`],
    /// which returns references that match the ownership marker of the
    /// current report and may not be cloneable for mutable reports.
    ///
    /// See also: [`Report::iter_reports`] for iterating over all reports
    /// including the current one.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::Cloneable};
    /// # use core::any::Any;
    /// // Create base reports
    /// let error1: Report = report!("error 1");
    /// let error2: Report = report!("error 2");
    ///
    /// // Build hierarchy using .context() which creates new nodes
    /// let with_context1 = error1.context("context for error 1");  // Creates new node with error1 as child
    /// let with_context2 = error2.context("context for error 2");  // Creates new node with error2 as child
    ///
    /// // Create root that contains both context nodes as children
    /// let mut root = report!("root error").context("context for root error");
    /// root.children_mut().push(with_context1.into_dyn_any().into_cloneable());
    /// root.children_mut().push(with_context2.into_dyn_any().into_cloneable());
    ///
    /// let sub_reports: Vec<String> = root
    ///     .iter_sub_reports()  // Note: using iter_sub_reports, not iter_reports
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
    /// # use rootcause::{prelude::*, preformatted::PreformattedContext, ReportRef, markers::{Uncloneable, Mutable, SendSync, Local}};
    /// # use core::any::Any;
    /// # #[derive(Default)]
    /// # struct NonSendSyncError(core::cell::Cell<()>);
    /// # let non_send_sync_error = NonSendSyncError::default();
    /// let mut report: Report<NonSendSyncError, Mutable, Local> = report!(non_send_sync_error);
    /// let preformatted: Report<PreformattedContext, Mutable, SendSync> = report.preformat();
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
    /// # use rootcause::prelude::*;
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let type_id = report.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    ///
    /// let report: Report<dyn Any> = report.into_dyn_any();
    /// let type_id = report.current_context_type_id();
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
    /// # use rootcause::{prelude::*, markers::SendSync};
    /// # use core::any::TypeId;
    /// let report = Report::new_sendsync_custom::<handlers::Debug>("error message");
    /// let handler_type = report.current_context_handler_type_id();
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
    /// # use core::any::{Any, TypeId};
    /// let report: Report = report!("error message");
    /// let source: Option<&dyn core::error::Error> = report.current_context_error_source();
    /// assert!(source.is_none()); // The context does not implement Error, so no source
    ///
    /// #[derive(Debug, thiserror::Error)]
    /// enum MyError {
    ///     #[error("Io error: {0}")]
    ///     Io(#[from] std::io::Error),
    ///     // ...
    /// }
    ///
    /// let report: Report<MyError> = report!(MyError::Io(std::io::Error::other("My inner error")));
    /// let source: Option<&dyn std::error::Error> = report.current_context_error_source();
    /// assert_eq!(format!("{}", source.unwrap()), "My inner error");
    /// ```
    pub fn current_context_error_source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.as_raw_ref().context_source()
    }

    /// Formats the current context with hook processing.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("error message");
    /// let formatted = report.format_current_context();
    /// println!("{formatted}");
    /// ```
    pub fn format_current_context(&self) -> impl core::fmt::Display + core::fmt::Debug {
        let report: ReportRef<'_, dyn Any, Uncloneable, Local> =
            unsafe { ReportRef::from_raw(self.as_raw_ref()) };
        format_helper(
            report,
            |report, formatter| {
                crate::hooks::formatting_overrides::context::display_context(report, formatter)
            },
            |report, formatter| {
                crate::hooks::formatting_overrides::context::debug_context(report, formatter)
            },
        )
    }

    /// Formats the current context without hook processing.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("error message");
    /// let formatted = report.format_current_context_unhooked();
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
    /// See also [`Report::preferred_context_formatting_style_unhooked`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("error message");
    /// let style = report.preferred_context_formatting_style(handlers::FormattingFunction::Display);
    /// ```
    pub fn preferred_context_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let report: ReportRef<'_, dyn Any, Uncloneable, Local> =
            unsafe { ReportRef::from_raw(self.as_raw_ref()) };
        crate::hooks::formatting_overrides::context::get_preferred_context_formatting_style(
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
    /// let report: Report = report!("error message");
    /// let style =
    ///     report.preferred_context_formatting_style_unhooked(handlers::FormattingFunction::Display);
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
    /// let report: Report = report!("error message");
    /// assert_eq!(report.strong_count(), 1); // We just created the report so it has a single owner
    /// ```
    pub fn strong_count(&self) -> usize {
        self.as_raw_ref().strong_count()
    }
}

impl<O, T> Report<dyn Any, O, T>
where
    O: markers::ReportOwnershipMarker,
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
    /// # use core::any::Any;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report<dyn Any> = report.into_dyn_any();
    /// let context: Option<&MyError> = dyn_report.downcast_current_context();
    /// assert!(context.is_some());
    /// ```
    #[must_use]
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
    /// [`current_context_type_id()`]: Report::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report = report.into_dyn_any();
    ///
    /// // Verify the type first
    /// if dyn_report.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let context: &MyError = unsafe { dyn_report.downcast_current_context_unchecked() };
    /// }
    /// ```
    #[must_use]
    pub unsafe fn downcast_current_context_unchecked<C>(&self) -> &C
    where
        C: markers::ObjectMarker,
    {
        unsafe { self.as_ref().downcast_current_context_unchecked() }
    }

    /// Attempts to downcast the report to a specific context type.
    ///
    /// Returns `Ok(report)` if the current context is of type `C`,
    /// otherwise returns `Err(self)` with the original report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report = report.into_dyn_any();
    /// let downcasted: Result<Report<MyError>, _> = dyn_report.downcast_report();
    /// assert!(downcasted.is_ok());
    /// ```
    pub fn downcast_report<C>(self) -> Result<Report<C, O, T>, Self>
    where
        C: markers::ObjectMarker,
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

    /// Downcasts the report to a specific context type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current context is actually of type `C`.
    /// This can be verified by calling [`current_context_type_id()`] first.
    ///
    /// [`current_context_type_id()`]: Report::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::{Any, TypeId};
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report = report.into_dyn_any();
    ///
    /// // Verify the type first
    /// if dyn_report.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let downcasted: Report<MyError> = unsafe { dyn_report.downcast_report_unchecked() };
    /// }
    /// ```
    #[must_use]
    pub unsafe fn downcast_report_unchecked<C>(self) -> Report<C, O, T>
    where
        C: markers::ObjectMarker,
    {
        unsafe { Report::from_raw(self.into_raw()) }
    }
}

impl<C> Report<C, Mutable, SendSync>
where
    C: markers::ObjectMarker + Send + Sync,
{
    /// Creates a new [`Report`] with [`SendSync`] thread safety.
    ///
    /// This is a convenience method that calls [`Report::new`] with explicit
    /// [`SendSync`] thread safety. Use this method when you're having
    /// trouble with type inference for the thread safety parameter.
    ///
    /// The context will use the [`handlers::Error`] handler to format the
    /// context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # #[derive(Debug)]
    /// # struct MyError;
    /// # impl core::error::Error for MyError {}
    /// # impl core::fmt::Display for MyError { fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { unimplemented!() }}
    /// let report = Report::new_sendsync(MyError);
    /// ```
    #[track_caller]
    #[must_use]
    pub fn new_sendsync(context: C) -> Self
    where
        C: core::error::Error,
    {
        Self::new(context)
    }

    /// Creates a new [`Report`] with [`SendSync`] thread safety and the given
    /// handler.
    ///
    /// This is a convenience method that calls [`Report::new_custom`] with
    /// explicit [`SendSync`] thread safety. Use this method when you're
    /// having trouble with type inference for the thread safety parameter.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report = Report::new_sendsync_custom::<handlers::Display>("error");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn new_sendsync_custom<H>(context: C) -> Self
    where
        H: ContextHandler<C>,
    {
        Self::new_custom::<H>(context)
    }
}

impl<C> Report<C, Mutable, Local>
where
    C: markers::ObjectMarker,
{
    /// Creates a new [`Report`] with [`Local`] thread safety.
    ///
    /// This is a convenience method that calls [`Report::new`] with explicit
    /// [`Local`] thread safety. Use this method when you're having trouble
    /// with type inference for the thread safety parameter.
    ///
    /// The context will use the [`handlers::Error`] handler to format the
    /// context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # #[derive(Debug)]
    /// # struct MyError;
    /// # impl core::error::Error for MyError {}
    /// # impl core::fmt::Display for MyError { fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { unimplemented!() }}
    /// let report = Report::new_local(MyError);
    /// ```
    #[track_caller]
    #[must_use]
    pub fn new_local(context: C) -> Self
    where
        C: core::error::Error,
    {
        Self::new(context)
    }

    /// Creates a new [`Report`] with [`Local`] thread safety and the given
    /// handler.
    ///
    /// This is a convenience method that calls [`Report::new_custom`] with
    /// explicit [`Local`] thread safety. Use this method when you're having
    /// trouble with type inference for the thread safety parameter.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report = Report::new_local_custom::<handlers::Display>("error");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn new_local_custom<H>(context: C) -> Self
    where
        H: ContextHandler<C>,
    {
        Self::new_custom::<H>(context)
    }
}

unsafe impl<C, O> Send for Report<C, O, SendSync>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
{
}
unsafe impl<C, O> Sync for Report<C, O, SendSync>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
{
}

impl<C, T> From<C> for Report<C, Mutable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
    T: markers::ThreadSafetyMarker,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context)
    }
}

impl<C, T> From<C> for Report<C, Cloneable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
    T: markers::ThreadSafetyMarker,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context).into_cloneable()
    }
}

impl<C, T> From<C> for Report<dyn Any, Mutable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
    T: markers::ThreadSafetyMarker,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context).into_dyn_any()
    }
}

impl<C, T> From<C> for Report<dyn Any, Cloneable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
    T: markers::ThreadSafetyMarker,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context).into_dyn_any().into_cloneable()
    }
}

impl<C, T> Clone for Report<C, Cloneable, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        // SAFETY: We must guarantee that there are no external assumptions
        // that Arc is unique. However since we are a `Report<C, Cloneable, T>` there
        // are no such external assumptions.
        let raw = unsafe { self.as_raw_ref().clone_arc() };

        // SAFETY:
        // - The context is valid, because does not change
        // - The ownership of the cloned report is also set to Cloneable, which is still
        //   valid
        // - The thread marker is valid, because it does not change
        unsafe { Report::from_raw(raw) }
    }
}

impl<C, O, T> core::fmt::Display for Report<C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.as_ref(), f)
    }
}

impl<C, O, T> core::fmt::Debug for Report<C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), f)
    }
}

mod from_impls {
    use super::*;

    macro_rules! unsafe_report_to_report {
        ($(
            <
                $($param:ident),*
            >:
            $context1:ty => $context2:ty,
            $ownership1:ty => $ownership2:ty,
            $thread_safety1:ty => $thread_safety2:ty
        ),* $(,)?) => {
            $(
                impl<$($param),*> From<Report<$context1, $ownership1, $thread_safety1>> for Report<$context2, $ownership2, $thread_safety2>
                    where $(
                        $param: markers::ObjectMarker,
                    )*
                 {
                    #[track_caller]
                    fn from(report: Report<$context1, $ownership1, $thread_safety1>) -> Self {
                        // SAFETY:
                        // - The context is valid, because it either doesn't change or goes from a known `C` to `dyn Any`
                        // - The ownership marker is valid, because it either does not change or it goes from `Mutable` to `Cloneable`
                        // - The thread marker is valid, because it either does not change or it goes from `SendSync` to `Local`
                        unsafe { Report::from_raw(report.into_raw()) }
                    }
                }
            )*
        };
    }

    unsafe_report_to_report!(
        <C>: C => C, Mutable => Mutable, SendSync => Local,
        <C>: C => C, Mutable => Cloneable, SendSync => SendSync,
        <C>: C => C, Mutable => Cloneable, SendSync => Local,
        <C>: C => C, Mutable => Cloneable, Local => Local,
        <C>: C => C, Cloneable => Cloneable, SendSync => Local,
        <C>: C => dyn Any, Mutable => Mutable, SendSync => SendSync,
        <C>: C => dyn Any, Mutable => Mutable, SendSync => Local,
        <C>: C => dyn Any, Mutable => Mutable, Local => Local,
        <C>: C => dyn Any, Mutable => Cloneable, SendSync => SendSync,
        <C>: C => dyn Any, Mutable => Cloneable, SendSync => Local,
        <C>: C => dyn Any, Mutable => Cloneable, Local => Local,
        <C>: C => dyn Any, Cloneable => Cloneable, SendSync => SendSync,
        <C>: C => dyn Any, Cloneable => Cloneable, SendSync => Local,
        <C>: C => dyn Any, Cloneable => Cloneable, Local => Local,
        <>:  dyn Any => dyn Any, Mutable => Mutable, SendSync => Local,
        <>:  dyn Any => dyn Any, Mutable => Cloneable, SendSync => SendSync,
        <>:  dyn Any => dyn Any, Mutable => Cloneable, SendSync => Local,
        <>:  dyn Any => dyn Any, Mutable => Cloneable, Local => Local,
        <>:  dyn Any => dyn Any, Cloneable => Cloneable, SendSync => Local,
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
    fn test_report_send_sync() {
        static_assertions::assert_impl_all!(Report<(), Mutable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<(), Cloneable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<String, Mutable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<String, Cloneable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<NonSend, Mutable, SendSync>: Send, Sync); // This still makes sense, since you won't actually be able to construct this report
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<dyn Any, Mutable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<dyn Any, Cloneable, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(Report<(), Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<(), Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<String, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<NonSend, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Cloneable, Local>: Send, Sync);
    }

    #[test]
    fn test_report_copy_clone() {
        static_assertions::assert_not_impl_any!(Report<(), Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<(), Mutable, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Mutable, Local>: Copy, Clone);

        static_assertions::assert_impl_all!(Report<(), Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<(), Cloneable, Local>: Clone);
        static_assertions::assert_impl_all!(Report<String, Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<String, Cloneable, Local>: Clone);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, Local>: Clone);
        static_assertions::assert_impl_all!(Report<dyn Any, Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<dyn Any, Cloneable, Local>: Clone);

        static_assertions::assert_not_impl_any!(Report<(), Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<(), Cloneable, Local>: Copy);
        static_assertions::assert_not_impl_any!(Report<String, Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<String, Cloneable, Local>: Copy);
        static_assertions::assert_not_impl_any!(Report<NonSend, Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<NonSend, Cloneable, Local>: Copy);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<dyn Any, Cloneable, Local>: Copy);
    }
}
