use core::any::{Any, TypeId};

use rootcause_internals::{
    RawReport,
    handlers::{ContextFormattingStyle, FormattingFunction},
};

use crate::{
    ReportIter, ReportMut, ReportRef,
    handlers::{self, ContextHandler},
    markers::{self, Cloneable, Local, Mutable, SendSync, Uncloneable},
    preformatted::PreformattedContext,
    report_attachment::ReportAttachment,
    report_attachments::ReportAttachments,
    report_collection::ReportCollection,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::{any::Any, marker::PhantomData};

    use rootcause_internals::{RawReport, RawReportMut, RawReportRef};

    use crate::markers::{self, Mutable, SendSync};

    /// An error report that contains a context, child reports, and attachments.
    ///
    /// [`Report`] is the main type for creating and working with error reports
    /// in this library. It can contain a root context (typically an error),
    /// zero or more child reports, and zero or more attachments.
    ///
    /// # Type Parameters
    ///
    /// [`Report`] has three type parameters that control its behavior:
    ///
    /// - **Context (`C`)**: The type of the root error or context (defaults to
    ///   `dyn Any`)
    /// - **Ownership (`O`)**: Controls whether the report can be cloned
    ///   - [`Mutable`]: Unique ownership, can modify but cannot clone (default)
    ///   - [`Cloneable`]: Shared ownership via [`Arc`], can clone but cannot
    ///     modify root
    /// - **Thread Safety (`T`)**: Controls whether the report can be sent
    ///   across threads
    ///   - [`SendSync`]: Can be sent across threads (default, requires all data
    ///     is [`Send`]+[`Sync`])
    ///   - [`Local`]: Cannot be sent across threads (allows non-thread-safe
    ///     data)
    ///
    /// # Common Usage
    ///
    /// The easiest way to create a [`Report`] is with the [`report!()`] macro:
    ///
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("file missing");
    /// println!("{report}");
    /// ```
    ///
    /// You can add context and attachments using method chaining:
    ///
    /// ```
    /// # use rootcause::prelude::*;
    /// let report = report!("database query failed")
    ///     .context("failed to fetch user data")
    ///     .attach("user_id: 12345");
    /// println!("{report}");
    /// ```
    ///
    /// [`Arc`]: triomphe::Arc
    /// [`Local`]: crate::markers::Local
    /// [`Mutable`]: crate::markers::Mutable
    /// [`Cloneable`]: crate::markers::Cloneable
    /// [`SendSync`]: crate::markers::SendSync
    /// [`report!()`]: crate::report!
    #[repr(transparent)]
    pub struct Report<Context = dyn Any, Ownership = Mutable, ThreadSafety = SendSync>
    where
        Context: markers::ObjectMarker + ?Sized,
        Ownership: markers::ReportOwnershipMarker,
        ThreadSafety: markers::ThreadSafetyMarker,
    {
        /// # Safety
        ///
        /// The following safety invariants must be upheld as long as this
        /// struct exists:
        ///
        /// 1. If `C` is a concrete type: The context embedded in the report
        ///    must be of type `C`
        /// 2. If `O = Mutable`: This is the unique owner of the report. More
        ///    specifically this means that the strong count of the underlying
        ///    `triomphe::Arc` is exactly 1.
        /// 3. If `O = Cloneable`: All other references to this report are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 4. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 5. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`.
        raw: RawReport,
        _context: PhantomData<Context>,
        _ownership: PhantomData<Ownership>,
        _thread_safety: PhantomData<ThreadSafety>,
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
        /// The caller must ensure:
        ///
        /// 1. If `C` is a concrete type: The context embedded in the report
        ///    must be of type `C`
        /// 2. If `O = Mutable`: This is the unique owner of the report. More
        ///    specifically this means that the strong count of the underlying
        ///    `triomphe::Arc` is exactly 1.
        /// 3. If `O = Cloneable`: All other references to this report are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 4. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 5. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`.
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawReport) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            // 3. Guaranteed by the caller
            // 4. Guaranteed by the caller
            // 5. Guaranteed by the caller
            Self {
                raw,
                _context: PhantomData,
                _ownership: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Consumes the [`Report`] and returns the inner [`RawReport`].
        #[must_use]
        pub(crate) fn into_raw(self) -> RawReport {
            // SAFETY: We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }

        /// Creates a lifetime-bound [`RawReportRef`] from the inner
        /// [`RawReport`].
        #[must_use]
        pub(crate) fn as_raw_ref(&self) -> RawReportRef<'_> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Trivially upheld, as no mutation occurs
            // 2. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    incompatible with shared ownership when `O=Mutable`, this cannot happen.
            // 3. Trivially upheld, as no mutation occurs
            // 4. Upheld, as this does not create any such references.
            // 5. Trivially upheld, as no mutation occurs
            let raw = &self.raw;

            raw.as_ref()
        }
    }

    impl<C, T> Report<C, markers::Mutable, T>
    where
        C: markers::ObjectMarker + ?Sized,
        T: markers::ThreadSafetyMarker,
    {
        /// Creates a lifetime-bound [`RawReportMut`] from the inner
        /// [`RawReport`].
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `T = SendSync`, no objects are added to the report through
        ///    this that are not `Send+Sync`
        #[must_use]
        pub(crate) unsafe fn as_raw_mut(&mut self) -> RawReportMut<'_> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. While mutation of the context is possible through this reference, it is
            //    not possible to change the type of the context. Therefore this invariant
            //    is upheld.
            // 2. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    the unique owner, this cannot happen.
            // 3. `O = Mutable`, so this is trivially upheld.
            // 4. Upheld, as this does not create any such references.
            // 5. The caller guarantees that the current report is not modified in an
            //    incompatible way and it is not possible to mutate the sub-reports.
            let raw = &mut self.raw;

            // SAFETY:
            // 1. This method is in an impl for `Report<C, Mutable, T>`, which means that we
            //    can invoke our own safety invariant to show that this is indeed the unique
            //    owner of the report.
            unsafe { raw.as_mut() }
        }
    }
}

pub use limit_field_access::Report;

impl<C, T> Report<C, Mutable, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Returns a mutable reference to the report.
    ///
    /// # Examples
    /// ```
    /// use rootcause::{ReportMut, prelude::*};
    /// let mut report: Report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// ```
    #[must_use]
    pub fn as_mut(&mut self) -> ReportMut<'_, C, T> {
        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T=SendSync`, then
        //    we are not allowed to mutate the returned raw report in a way that
        //    adds non-`Send+Sync` objects. However the invariants of the created
        //    `ReportMut` guarantee that no such mutation can occur.
        let raw = unsafe { self.as_raw_mut() };

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. Have have a `&mut self`. This means that there are no other borrows active
        //    to `self`. We also know that this is the unique owner of the report, so
        //    there are no other references to it through different ownership. This
        //    means that there are no other references to this report at all, so there
        //    cannot be any incompatible references.
        unsafe { ReportMut::from_raw(raw) }
    }

    /// Creates a new [`Report`] with the given context.
    ///
    /// This method is generic over the thread safety marker `T`. The context
    /// will use the [`handlers::Error`] handler for formatting.
    ///
    /// See also:
    ///
    /// - The [`report!()`] macro will also create a new [`Report`], but can
    ///   auto-detect the thread safety marker and handler.
    /// - [`Report::new_sendsync`] and [`Report::new_local`] are more
    ///   restrictive variants of this function that might help avoid type
    ///   inference issues.
    /// - [`Report::new_custom`] also allows you to manually specify the
    ///   handler.
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
        T::run_creation_hooks(report.as_mut().into_dyn_any());
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
        let raw = RawReport::new::<C, H>(context, children.into_raw(), attachments.into_raw());

        // SAFETY:
        // 1. We just created the report and the `C` does indeed match.
        // 2. We just created the report and we are the unique owner.
        // 3. `O=Mutable`, so this is trivially upheld.
        // 4. This is guaranteed by the invariants of the `ReportCollection` we used to
        //    create the raw report.
        // 5. If `T=Local`, then this is trivially true. If `T=SendSync`, then we have
        //    `C: ObjectMarkerFor<SendSync>`, so the context is `Send+Sync`.
        //    Additionally the invariants of the `ReportCollection` and
        //    `ReportAttachments` guarantee that the children and attachments are
        //    `Send+Sync` as well.
        unsafe {
            // @add-unsafe-context: ReportCollection
            // @add-unsafe-context: ReportAttachments
            // @add-unsafe-context: markers::ObjectMarkerFor
            Report::<C, Mutable, T>::from_raw(raw)
        }
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

    /// Returns a mutable reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_collection::ReportCollection};
    /// let mut report: Report = report!("error message");
    /// let children_mut: &mut ReportCollection = report.children_mut();
    /// ```
    #[must_use]
    pub fn children_mut(&mut self) -> &mut ReportCollection<dyn Any, T> {
        self.as_mut().into_children_mut()
    }

    /// Returns a mutable reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_attachments::ReportAttachments};
    /// let mut report: Report = report!("error message");
    /// let attachments_mut: &mut ReportAttachments = report.attachments_mut();
    /// ```
    #[must_use]
    pub fn attachments_mut(&mut self) -> &mut ReportAttachments<T> {
        self.as_mut().into_attachments_mut()
    }
}

impl<C, T> Report<C, Mutable, T>
where
    C: markers::ObjectMarker,
    T: markers::ThreadSafetyMarker,
{
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
    #[must_use]
    pub fn into_parts(self) -> (C, ReportCollection<dyn Any, T>, ReportAttachments<T>) {
        let raw = self.into_raw();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. Since `O=Mutable` and we are consuming `self`, then this is guaranteed to
        //    be the unique owner of the report.
        let (context, children, attachments) = unsafe { raw.into_parts() };

        // SAFETY:
        // 1. `C=dyn Any` for the created collection, so this is trivially true.
        // 2. This is guaranteed by the invariants of this type.
        // 3. If `T=Local`, then this is trivially true. If `T=SendSync`, then this is
        //    guaranteed by the invariants of this type.
        let children = unsafe { ReportCollection::<dyn Any, T>::from_raw(children) };

        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T=SendSync`, then this is
        //    guaranteed by the invariants of this type.
        let attachments = unsafe { ReportAttachments::<T>::from_raw(attachments) };

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
    #[must_use]
    pub fn into_current_context(self) -> C {
        self.into_parts().0
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
    pub fn current_context_mut(&mut self) -> &mut C {
        self.as_mut().into_current_context_mut()
    }
}

impl<C, O, T> Report<C, O, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
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
    /// # use rootcause::{prelude::*, report_collection::ReportCollection};
    /// let report: Report = report!("error message");
    /// let children: &ReportCollection = report.children();
    /// assert_eq!(children.len(), 0); // The report has just been created, so it has no children
    /// ```
    #[must_use]
    pub fn children(&self) -> &ReportCollection<dyn Any, T> {
        self.as_ref().children()
    }

    /// Returns a reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, report_attachments::ReportAttachments};
    /// let report: Report = report!("error message");
    /// let attachments: &ReportAttachments = report.attachments();
    /// ```
    #[must_use]
    pub fn attachments(&self) -> &ReportAttachments<T> {
        self.as_ref().attachments()
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
        let raw = self.into_raw();

        // SAFETY:
        // 1. `C=dyn Any`, so this is trivially true.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        unsafe { Report::<dyn Any, O, T>::from_raw(raw) }
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
        let raw = self.into_raw();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        unsafe { Report::<C, Cloneable, T>::from_raw(raw) }
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
    /// are neither [`Send`] nor [`Sync`], but the report itself will no
    /// longer be [`Send`]+[`Sync`].
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
        let raw = self.into_raw();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. `T=Local`, so this is trivially true.
        unsafe { Report::<C, O, Local>::from_raw(raw) }
    }

    /// Checks if there is only a single unique owner of the root node of the
    /// [`Report`].
    ///
    /// If there is only a single unique owner, this method
    /// marks the current [`Report`] as [`Mutable`] and returns `Ok`,
    /// otherwise it returns `Err` with the original [`Report`].
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
        if self.strong_count() == 1 {
            let raw = self.into_raw();

            // SAFETY:
            // 1. This is guaranteed by the invariants of this type.
            // 2. We just checked that the strong count is `1`, and we are taking ownership
            //    of `self`, so we are the unique owner.
            // 3. `O=Mutable`, so this is trivially upheld.
            // 4. This is guaranteed by the invariants of this type.
            // 5. This is guaranteed by the invariants of this type.
            let report = unsafe { Report::from_raw(raw) };
            Ok(report)
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
    #[must_use]
    pub fn as_ref(&self) -> ReportRef<'_, C, O::RefMarker, T> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. If the ownership of `self` is `Mutable`, then the `O=Uncloneable`, which
        //    means that this is trivially true. On the other hand, if the ownership of
        //    `self` is `Cloneable`, then the `O=Cloneable`. However in that case, the
        //    invariants of this type guarantee that all references are compatible.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        unsafe {
            // @add-unsafe-context: markers::ReportOwnershipMarker
            ReportRef::<C, O::RefMarker, T>::from_raw(raw)
        }
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
        self.as_ref().iter_reports()
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
        self.as_ref().iter_sub_reports()
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
    #[must_use]
    pub fn current_context_type_id(&self) -> TypeId {
        self.as_ref().current_context_type_id()
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
    #[must_use]
    pub fn current_context_handler_type_id(&self) -> TypeId {
        self.as_ref().current_context_handler_type_id()
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
    #[must_use]
    pub fn current_context_error_source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.as_ref().current_context_error_source()
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
    #[must_use]
    pub fn format_current_context(&self) -> impl core::fmt::Display + core::fmt::Debug {
        self.as_ref().format_current_context()
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
    #[must_use]
    pub fn format_current_context_unhooked(&self) -> impl core::fmt::Display + core::fmt::Debug {
        self.as_ref().format_current_context_unhooked()
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
    #[must_use]
    pub fn preferred_context_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let report: ReportRef<'_, dyn Any, Uncloneable, Local> =
            self.as_ref().into_dyn_any().into_uncloneable().into_local();
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
    #[must_use]
    pub fn preferred_context_formatting_style_unhooked(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        self.as_ref()
            .preferred_context_formatting_style_unhooked(report_formatting_function)
    }

    /// Returns the number of references to this report.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// let report: Report = report!("error message");
    /// assert_eq!(report.strong_count(), 1); // We just created the report so it has a single owner
    /// ```
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.as_ref().strong_count()
    }
}

impl<C, O, T> Report<C, O, T>
where
    C: markers::ObjectMarker,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
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
    pub fn current_context(&self) -> &C {
        self.as_ref().current_context()
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
        self.as_ref().downcast_current_context()
    }

    /// Downcasts the current context to a specific type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The current context is actually of type `C` (can be verified by
    ///    calling [`current_context_type_id()`] first)
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
        let report = self.as_ref();

        // SAFETY:
        // 1. Guaranteed by the caller
        unsafe { report.downcast_current_context_unchecked() }
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
        if TypeId::of::<C>() == self.current_context_type_id() {
            // SAFETY:
            // 1. We just checked that the type IDs match.
            let report = unsafe { self.downcast_report_unchecked() };

            Ok(report)
        } else {
            Err(self)
        }
    }

    /// Downcasts the report to a specific context type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The current context is actually of type `C` (can be verified by
    ///    calling [`current_context_type_id()`] first)
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
        let raw = self.into_raw();

        // SAFETY:
        // 1. Guaranteed by the caller
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        unsafe { Report::from_raw(raw) }
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

// SAFETY: The `SendSync` marker indicates that all objects in the report are
// `Send`+`Sync`. Therefore it is safe to implement `Send`+`Sync` for the report
// itself.
unsafe impl<C, O> Send for Report<C, O, SendSync>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
{
}

// SAFETY: The `SendSync` marker indicates that all objects in the report are
// `Send`+`Sync`. Therefore it is safe to implement `Send`+`Sync` for the report
// itself.
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
        self.as_ref().clone_arc()
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

    macro_rules! from_impls {
        ($(
            <
                $($param:ident),*
            >:
            $context1:ty => $context2:ty,
            $ownership1:ty => $ownership2:ty,
            $thread_safety1:ty => $thread_safety2:ty,
            [$($op:ident),*]
        ),* $(,)?) => {
            $(
                impl<$($param),*> From<Report<$context1, $ownership1, $thread_safety1>> for Report<$context2, $ownership2, $thread_safety2>
                    where $(
                        $param: markers::ObjectMarker,
                    )*
                 {
                    #[track_caller]
                    fn from(report: Report<$context1, $ownership1, $thread_safety1>) -> Self {
                        report
                            $(.$op())*
                    }
                }
            )*
        };
    }

    from_impls!(
        <C>: C => C, Mutable => Mutable, SendSync => Local, [into_local],
        <C>: C => C, Mutable => Cloneable, SendSync => SendSync, [into_cloneable],
        <C>: C => C, Mutable => Cloneable, SendSync => Local, [into_cloneable, into_local],
        <C>: C => C, Mutable => Cloneable, Local => Local, [into_cloneable],
        <C>: C => C, Cloneable => Cloneable, SendSync => Local, [into_local],
        <C>: C => dyn Any, Mutable => Mutable, SendSync => SendSync, [into_dyn_any],
        <C>: C => dyn Any, Mutable => Mutable, SendSync => Local, [into_dyn_any, into_local],
        <C>: C => dyn Any, Mutable => Mutable, Local => Local, [into_dyn_any],
        <C>: C => dyn Any, Mutable => Cloneable, SendSync => SendSync, [into_dyn_any, into_cloneable],
        <C>: C => dyn Any, Mutable => Cloneable, SendSync => Local, [into_dyn_any, into_cloneable, into_local],
        <C>: C => dyn Any, Mutable => Cloneable, Local => Local, [into_dyn_any, into_cloneable],
        <C>: C => dyn Any, Cloneable => Cloneable, SendSync => SendSync, [into_dyn_any, into_cloneable],
        <C>: C => dyn Any, Cloneable => Cloneable, SendSync => Local, [into_dyn_any, into_cloneable, into_local],
        <C>: C => dyn Any, Cloneable => Cloneable, Local => Local, [into_dyn_any, into_cloneable],
        <>:  dyn Any => dyn Any, Mutable => Mutable, SendSync => Local, [into_local],
        <>:  dyn Any => dyn Any, Mutable => Cloneable, SendSync => SendSync, [into_cloneable],
        <>:  dyn Any => dyn Any, Mutable => Cloneable, SendSync => Local, [into_cloneable, into_local],
        <>:  dyn Any => dyn Any, Mutable => Cloneable, Local => Local, [into_cloneable],
        <>:  dyn Any => dyn Any, Cloneable => Cloneable, SendSync => Local, [into_local],
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
