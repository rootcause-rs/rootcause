use core::any::TypeId;

use rootcause_internals::{
    RawReport,
    handlers::{ContextFormattingStyle, FormattingFunction},
};

use crate::{
    ReportConversion, ReportIter, ReportMut, ReportRef,
    handlers::{self, ContextHandler},
    markers::{self, Cloneable, Dynamic, Local, Mutable, SendSync, Uncloneable},
    preformatted::{self, PreformattedContext},
    report_attachment::ReportAttachment,
    report_attachments::ReportAttachments,
    report_collection::ReportCollection,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::{RawReport, RawReportMut, RawReportRef};

    use crate::markers::{self, Dynamic, Mutable, SendSync};

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
    ///   [`Dynamic`])
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
    pub struct Report<
        Context: ?Sized + 'static = Dynamic,
        Ownership: 'static = Mutable,
        ThreadSafety: 'static = SendSync,
    > {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. `C` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. `O` must either be `Mutable` or `Cloneable`.
        /// 3. `T` must either be `SendSync` or `Local`.
        /// 4. If `C` is a `Sized` type: The context embedded in the report must
        ///    be of type `C`
        /// 5. If `O = Mutable`: This is the unique owner of the report. More
        ///    specifically this means that the strong count of the underlying
        ///    `triomphe::Arc` is exactly 1.
        /// 6. If `O = Cloneable`: All other references to this report are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 7. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 8. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`.
        raw: RawReport,
        _context: PhantomData<Context>,
        _ownership: PhantomData<Ownership>,
        _thread_safety: PhantomData<ThreadSafety>,
    }

    impl<C: ?Sized, O, T> Report<C, O, T> {
        /// Creates a new [`Report`] from a [`RawReport`]
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. `C` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. `O` must either be `Mutable` or `Cloneable`.
        /// 3. `T` must either be `SendSync` or `Local`.
        /// 4. If `C` is a `Sized` type: The context embedded in the report must
        ///    be of type `C`
        /// 5. If `O = Mutable`: This is the unique owner of the report. More
        ///    specifically this means that the strong count of the underlying
        ///    `triomphe::Arc` is exactly 1.
        /// 6. If `O = Cloneable`: All other references to this report are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 7. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 8. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`.
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawReport) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            // 3. Guaranteed by the caller
            // 4. Guaranteed by the caller
            // 5. Guaranteed by the caller
            // 6. Guaranteed by the caller
            // 7. Guaranteed by the caller
            // 8. Guaranteed by the caller
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
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. Upheld as the type parameters do not change.
            // 4. Trivially upheld, as no mutation occurs
            // 5. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    incompatible with shared ownership when `O=Mutable`, this cannot happen.
            // 6. Trivially upheld, as no mutation occurs
            // 7. Upheld, as this does not create any such references.
            // 8. Trivially upheld, as no mutation occurs
            let raw = &self.raw;

            raw.as_ref()
        }
    }

    impl<C: ?Sized, T> Report<C, markers::Mutable, T> {
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
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. Upheld as the type parameters do not change.
            // 4. While mutation of the context is possible through this reference, it is
            //    not possible to change the type of the context. Therefore this invariant
            //    is upheld.
            // 5. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    the unique owner, this cannot happen.
            // 6. `O = Mutable`, so this is trivially upheld.
            // 7. Upheld, as this does not create any such references.
            // 8. The caller guarantees that the current report is not modified in an
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

impl<C: Sized, T> Report<C, Mutable, T> {
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
        C: markers::ObjectMarkerFor<T> + core::error::Error,
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
        C: markers::ObjectMarkerFor<T>,
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
        children: ReportCollection<Dynamic, T>,
        attachments: ReportAttachments<T>,
    ) -> Self
    where
        C: markers::ObjectMarkerFor<T>,
        H: ContextHandler<C>,
    {
        let mut report: Self = Self::from_parts_unhooked::<H>(context, children, attachments);
        C::run_creation_hooks(report.as_mut().into_dynamic());
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
        children: ReportCollection<Dynamic, T>,
        attachments: ReportAttachments<T>,
    ) -> Self
    where
        C: markers::ObjectMarkerFor<T>,
        H: ContextHandler<C>,
    {
        let raw = RawReport::new::<C, H>(context, children.into_raw(), attachments.into_raw());

        // SAFETY:
        // 1. `C` is bounded by `Sized`, so this is upheld.
        // 2. `O=Mutable`, so this is trivially upheld.
        // 3. `C` is bounded by `markers::ObjectMarkerFor<T>` and this can only be
        //    implemented for `T=Local` and `T=SendSync`, so this is
        //   upheld.
        // 4. We just created the report and the `C` does indeed match.
        // 5. We just created the report and we are the unique owner.
        // 6. `O=Mutable`, so this is trivially upheld.
        // 7. This is guaranteed by the invariants of the `ReportCollection` we used to
        //    create the raw report.
        // 8. If `T=Local`, then this is trivially true. If `T=SendSync`, then we have
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

    /// Decomposes the [`Report`] into its constituent parts.
    ///
    /// Returns a tuple containing the context, the children collection and the
    /// attachments collection in that order. This is the inverse operation
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
    /// let child_report = report!("child error").into_dynamic().into_cloneable();
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
    pub fn into_parts(self) -> (C, ReportCollection<Dynamic, T>, ReportAttachments<T>) {
        let raw = self.into_raw();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. Since `O=Mutable` and we are consuming `self`, then this is guaranteed to
        //    be the unique owner of the report. We also know that there are no other
        //    references such as `ReportRef` or `ReportMut` active, as those would
        //    require a borrow of `self`.
        let (context, children, attachments) = unsafe { raw.into_parts() };

        // SAFETY:
        // 1. `C=Dynamic` for the created collection, so this is trivially true.
        // 2. The invariants of this type guarantee that `T` is either `Local` or
        //    `SendSync`.
        // 3. `C=Dynamic` for the created collection, so this is trivially true.
        // 4. This is guaranteed by the invariants of this type.
        // 5. If `T=Local`, then this is trivially true. If `T=SendSync`, then this is
        //    guaranteed by the invariants of this type.
        let children = unsafe { ReportCollection::<Dynamic, T>::from_raw(children) };

        // SAFETY:
        // 1. `T` is guaranteed to either be `Local` or `SendSync` by the invariants of
        //    this type.
        // 2. If `T=Local`, then this is trivially true. If `T=SendSync`, then this is
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

    /// Transforms the context of this report using a closure, preserving the
    /// report structure.
    ///
    /// This method extracts the current context, applies the provided closure
    /// to transform it, and creates a new report with the transformed
    /// context while keeping all children and attachments intact. The
    /// transformation bypasses the report creation hook to avoid running
    /// hooks twice (which would result in duplicate hook-added data such as
    /// backtraces and location tracking).
    ///
    /// Unlike [`context()`](Report::context) which wraps the current report as
    /// a child, this method replaces the context in-place while maintaining
    /// the same report structure.
    ///
    /// # When to Use
    ///
    /// **For reusable conversions:** Implement
    /// [`ReportConversion`](crate::ReportConversion) and use
    /// [`context_to()`](Report::context_to) in your application code. Use
    /// `context_transform()` *inside* your trait implementation when the
    /// wrapping is just a type change with no semantic meaning.
    ///
    /// **Use `context_transform()` when:**
    /// - Wrapping library errors in your application error enum where the
    ///   wrapping is mechanical (no additional semantic layer)
    /// - You want to preserve the original report structure without nesting
    /// - You want to keep the original hook data (backtraces, locations)
    /// - The type change doesn't represent a meaningful abstraction boundary
    ///
    /// **Use [`context_transform_nested()`](Report::context_transform_nested)
    /// when:**
    /// - The transformation marks a significant boundary and you want fresh
    ///   hook data at the transformation point
    /// - You need to track both where the error occurred AND where it was
    ///   wrapped
    ///
    /// **Use [`context()`](Report::context) when:**
    /// - You want to wrap the error under a new parent context
    ///
    /// See [`examples/error_hierarchy.rs`] for a complete comparison of these
    /// approaches.
    ///
    /// [`examples/error_hierarchy.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/error_hierarchy.rs
    ///
    /// # Examples
    ///
    /// ## Common Use Case: Wrapping in Error Hierarchy
    ///
    /// The most common use case is wrapping library errors in your
    /// application's error enum:
    ///
    /// ```rust
    /// use rootcause::prelude::*;
    ///
    /// // Library-specific error
    /// #[derive(Debug)]
    /// struct DeserializationError {
    ///     details: String,
    /// }
    ///
    /// impl std::fmt::Display for DeserializationError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "Deserialization failed: {}", self.details)
    ///     }
    /// }
    ///
    /// impl std::error::Error for DeserializationError {}
    ///
    /// // Application error hierarchy
    /// #[derive(Debug)]
    /// enum AppError {
    ///     DeserializationError(DeserializationError),
    ///     // ... other variants
    /// }
    ///
    /// impl std::fmt::Display for AppError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         match self {
    ///             AppError::DeserializationError(e) => write!(f, "App error: {}", e),
    ///         }
    ///     }
    /// }
    ///
    /// impl std::error::Error for AppError {}
    ///
    /// let deserialization_report: Report<DeserializationError> = report!(DeserializationError {
    ///     details: "Invalid JSON".to_string(),
    /// });
    ///
    /// // Wrap in application error hierarchy
    /// let app_report: Report<AppError> =
    ///     deserialization_report.context_transform(AppError::DeserializationError);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`context_transform_nested()`](Report::context_transform_nested) -
    ///   Transforms while nesting the entire original report as a child
    /// - [`context()`](Report::context) - Wraps the report as a child under new
    ///   context
    /// - [`context_to()`](Report::context_to) - Converts using a
    ///   [`ReportConversion`](crate::ReportConversion) trait implementation
    pub fn context_transform<F, D>(self, f: F) -> Report<D, Mutable, T>
    where
        F: FnOnce(C) -> D,
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
    {
        let (context, children, attachments) = self.into_parts();
        let new_context = f(context);

        Report::from_parts_unhooked::<handlers::Display>(new_context, children, attachments)
    }

    /// Transforms the context of this report while nesting the entire
    /// original report structure as a child.
    ///
    /// This method is similar to
    /// [`context_transform()`](Report::context_transform), but instead of
    /// replacing the context in-place, it preformats the current report
    /// (converting the context to
    /// [`PreformattedContext`](crate::preformatted::PreformattedContext) which
    /// stores its string representation along with any hook-generated data),
    /// then wraps the entire preformatted report as a child under the new
    /// context. The new context is created by applying the provided closure
    /// to the old context value.
    ///
    /// Since this creates a new context node via
    /// [`context()`](Report::context), the report creation hooks run again,
    /// capturing fresh hook data (such as a new backtrace and location) for
    /// the transformed context. This results in a report with the new
    /// context as the root and the entire original report structure (with
    /// preformatted context) nested as its single child.
    ///
    /// # When to Use
    ///
    /// **For reusable conversions:** Implement
    /// [`ReportConversion`](crate::ReportConversion) and use
    /// [`context_to()`](Report::context_to) in your application code. Use
    /// `context_transform_nested()` *inside* your trait implementation when
    /// crossing significant abstraction boundaries.
    ///
    /// **Use `context_transform_nested()` when:**
    /// - The transformation marks a significant abstraction boundary (e.g.,
    ///   library error → application error) and you want fresh hook data
    /// - You need to track both where the error originally occurred AND where
    ///   it was wrapped in your error hierarchy
    /// - Using hooks like `rootcause-backtrace` and want separate backtraces at
    ///   both the original error location and the wrapping location
    /// - The act of transforming the error is semantically meaningful
    ///
    /// **Use [`context_transform()`](Report::context_transform) when:**
    /// - The transformation is just a mechanical type wrapper
    /// - You want to preserve original hook data without running creation hooks
    ///   again
    /// - The type change doesn't represent a meaningful abstraction boundary
    ///
    /// **Use [`context()`](Report::context) when:**
    /// - You want to wrap the error under a new parent context
    ///
    /// See [`examples/error_hierarchy.rs`] for a complete comparison of these
    /// approaches.
    ///
    /// [`examples/error_hierarchy.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/error_hierarchy.rs
    ///
    /// # Examples
    ///
    /// ## Common Use Case: Wrapping in Error Hierarchy with Metadata Preservation
    ///
    /// The most common use case is wrapping library errors in your
    /// application's error enum while preserving the original error's
    /// backtrace:
    ///
    /// ```rust
    /// use rootcause::prelude::*;
    ///
    /// // Library-specific error
    /// #[derive(Debug)]
    /// struct DeserializationError {
    ///     details: String,
    /// }
    ///
    /// impl std::fmt::Display for DeserializationError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "Deserialization failed: {}", self.details)
    ///     }
    /// }
    ///
    /// impl std::error::Error for DeserializationError {}
    ///
    /// // Application error hierarchy
    /// #[derive(Debug)]
    /// enum AppError {
    ///     DeserializationError(DeserializationError),
    ///     // ... other variants
    /// }
    ///
    /// impl std::fmt::Display for AppError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         match self {
    ///             AppError::DeserializationError(e) => write!(f, "App error: {}", e),
    ///         }
    ///     }
    /// }
    ///
    /// impl std::error::Error for AppError {}
    ///
    /// let deserialization_report: Report<DeserializationError> = report!(DeserializationError {
    ///     details: "Invalid JSON".to_string(),
    /// });
    ///
    /// // Wrap in application error hierarchy with fresh creation hooks
    /// let app_report: Report<AppError> =
    ///     deserialization_report.context_transform_nested(AppError::DeserializationError);
    ///
    /// // The formatted output will show:
    /// // - The new AppError context with fresh hook data from this location
    /// //   (such as a new backtrace if using rootcause-backtrace)
    /// // - The entire original report (with preformatted DeserializationError
    /// //   context and all its children/attachments) nested as a child, preserving
    /// //   its original hook data
    /// println!("{}", app_report);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`context_transform()`](Report::context_transform) - Transforms
    ///   without nesting (keeps same structure level)
    /// - [`preformat_root()`](Report::preformat_root) - Extracts context and
    ///   creates preformatted report
    /// - [`context()`](Report::context) - Wraps the report as a child under new
    ///   context
    pub fn context_transform_nested<F, D>(self, f: F) -> Report<D, Mutable, T>
    where
        F: FnOnce(C) -> D,
        D: markers::ObjectMarkerFor<T> + core::fmt::Display + core::fmt::Debug,
        PreformattedContext: markers::ObjectMarkerFor<T>,
    {
        let (context, report) = self.preformat_root();
        report.context_custom::<handlers::Display, _>(f(context))
    }

    /// Extracts the context and returns it along with a preformatted version of
    /// the report.
    ///
    /// This method decomposes the report into two parts:
    /// 1. The original context value of type `C`
    /// 2. A new report with a
    ///    [`PreformattedContext`](crate::preformatted::PreformattedContext)
    ///    that contains the string representation of the original context
    ///
    /// The preformatted report maintains the same structure (children and
    /// attachments) as the original, but replaces the typed context with a
    /// preformatted string version. This is useful when you need both the
    /// original typed value for processing and a formatted version for display.
    ///
    /// Unlike [`preformat()`](Report::preformat), which preformats the entire
    /// report hierarchy, this method only preformats the root context and
    /// returns it separately.
    ///
    /// # When to Use
    ///
    /// This method is primarily useful for implementing custom transformation
    /// logic similar to
    /// [`context_transform_nested()`](Report::context_transform_nested).
    /// Most users should use the higher-level transformation methods instead:
    /// - [`context_transform()`](Report::context_transform) - Transform without
    ///   running creation hooks again
    /// - [`context_transform_nested()`](Report::context_transform_nested) -
    ///   Transform with fresh creation hooks
    /// - [`context_to()`](Report::context_to) - Transform using a
    ///   [`ReportConversion`](crate::ReportConversion) trait
    ///
    /// # Examples
    ///
    /// Implementing a custom preserving transformation with conditional
    /// behavior:
    ///
    /// ```rust
    /// use rootcause::{preformatted::PreformattedContext, prelude::*};
    ///
    /// #[derive(Debug)]
    /// enum AppError {
    ///     Critical(String),
    ///     Warning(String),
    /// }
    ///
    /// impl std::fmt::Display for AppError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         match self {
    ///             AppError::Critical(msg) => write!(f, "CRITICAL: {}", msg),
    ///             AppError::Warning(msg) => write!(f, "Warning: {}", msg),
    ///         }
    ///     }
    /// }
    ///
    /// impl std::error::Error for AppError {}
    ///
    /// #[derive(Debug)]
    /// struct ParseError {
    ///     severity: u8,
    /// }
    ///
    /// impl std::fmt::Display for ParseError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "Parse error (severity {})", self.severity)
    ///     }
    /// }
    ///
    /// impl std::error::Error for ParseError {}
    ///
    /// let parse_report: Report<ParseError> = report!(ParseError { severity: 8 });
    ///
    /// // Custom transformation: preformat and wrap only if severity is high
    /// let (context, preformatted_report) = parse_report.preformat_root();
    /// let app_report: Report<AppError> = if context.severity >= 5 {
    ///     // High severity: wrap with new context (runs creation hooks again)
    ///     preformatted_report.context(AppError::Critical("High severity parse error".into()))
    /// } else {
    ///     // Low severity: just transform in-place (reuses existing hook data)
    ///     preformatted_report.context_transform(|_| AppError::Warning("Minor parse issue".into()))
    /// };
    /// ```
    ///
    /// # See Also
    ///
    /// - [`context_transform_nested()`](Report::context_transform_nested) -
    ///   Uses this method internally for standard preserving transformations
    /// - [`preformat()`](Report::preformat) - Preformats the entire report
    ///   hierarchy
    /// - [`into_parts()`](Report::into_parts) - Extracts context without
    ///   preformatting
    /// - [`current_context()`](crate::ReportRef::current_context) - Gets a
    ///   reference to the context without extraction
    pub fn preformat_root(self) -> (C, Report<PreformattedContext, Mutable, T>)
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

impl<C: ?Sized, T> Report<C, Mutable, T> {
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
        // 1. If `T=Local`, then this is trivially true. If `T=SendSync`, then we are
        //    not allowed to mutate the returned raw report in a way that adds
        //    non-`Send+Sync` objects. However the invariants of the created `ReportMut`
        //    guarantee that no such mutation can occur.
        let raw = unsafe { self.as_raw_mut() };

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        // 6. This is guaranteed by the invariants of this type.
        // 7. We have a `&mut self`. This means that there are no other borrows active
        //    to `self`. We also know that this is the unique owner of the report, so
        //    there are no other references to it through different ownership. This
        //    means that there are no other references to this report at all, so there
        //    cannot be any incompatible references.
        unsafe { ReportMut::from_raw(raw) }
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
            .push(ReportAttachment::new(attachment).into_dynamic());
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
            .push(ReportAttachment::new_custom::<H>(attachment).into_dynamic());
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
    pub fn children_mut(&mut self) -> &mut ReportCollection<Dynamic, T> {
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

impl<C: ?Sized, O, T> Report<C, O, T> {
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
            ReportCollection::from([self.into_dynamic().into_cloneable()]),
            ReportAttachments::<T>::new(),
        )
    }

    /// Converts this report to a different context type using a
    /// [`ReportConversion`] implementation.
    ///
    /// This is the **call-site API** for error conversion. You implement
    /// [`ReportConversion`] once to define *how* errors convert, then use
    /// `context_to()` throughout your code to *apply* that conversion. This
    /// separation enables clean, consistent error handling across your
    /// codebase.
    ///
    /// # When to Use
    ///
    /// **Use `context_to()` when you have a standard, reusable conversion
    /// pattern** between error types that you've encoded in a
    /// [`ReportConversion`] implementation.
    ///
    /// The typical pattern is:
    ///
    /// 1. **Define conversions once:** Implement [`ReportConversion`] for your
    ///    application error type. Common strategies include:
    ///    - [`context_transform()`](Report::context_transform) for lightweight
    ///      type wrapping (preserves structure, no hooks)
    ///    - [`context_transform_nested()`](Report::context_transform_nested)
    ///      for boundaries where you want fresh hook data
    ///    - [`context()`](Report::context) for wrapping with a new parent
    ///      context
    ///
    ///    You can also inspect the report and choose different strategies
    ///    based on its contents.
    ///
    /// 2. **Use everywhere:** Call `.context_to()` at call sites—the
    ///    conversion happens automatically based on your trait implementation.
    ///
    /// See [`examples/error_hierarchy.rs`] for a complete guide on choosing
    /// between transformation strategies, and [`examples/thiserror_interop.rs`]
    /// for patterns when integrating with thiserror.
    ///
    /// [`examples/error_hierarchy.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/error_hierarchy.rs
    /// [`examples/thiserror_interop.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/thiserror_interop.rs
    ///
    /// # Type Inference
    ///
    /// You typically need to specify the target type explicitly using the
    /// turbofish syntax (`::<Type>`), as Rust cannot always infer it from
    /// context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rootcause::{ReportConversion, markers::Mutable, prelude::*};
    /// # #[derive(Debug)]
    /// # enum MyError { ParseError(String) }
    /// # impl std::fmt::Display for MyError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #         match self { MyError::ParseError(msg) => write!(f, "Parse error: {}", msg) }
    /// #     }
    /// # }
    /// # impl std::error::Error for MyError {}
    /// # impl<O, T> ReportConversion<std::num::ParseIntError, O, T> for MyError
    /// #   where MyError: markers::ObjectMarkerFor<T>
    /// # {
    /// #     fn convert_report(report: Report<std::num::ParseIntError, O, T>) -> Report<Self, Mutable, T>
    /// #     {
    /// #         report.context(MyError::ParseError("Invalid number".to_string()))
    /// #     }
    /// # }
    /// fn parse_number(s: &str) -> Result<i32, Report<MyError>> {
    ///     s.parse::<i32>().context_to()  // Convert ParseIntError to MyError
    /// }
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context_to<D: ReportConversion<C, O, T>>(self) -> Report<D, Mutable, T> {
        D::convert_report(self)
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
    pub fn children(&self) -> &ReportCollection<Dynamic, T> {
        self.as_uncloneable_ref().children()
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
        self.as_uncloneable_ref().attachments()
    }

    /// Changes the context type of the [`Report`] to [`Dynamic`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the context mode to
    /// [`Dynamic`].
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
    /// # use rootcause::{prelude::*, markers::Dynamic};
    /// # struct MyError;
    /// # let my_error = MyError;
    /// let report: Report<MyError> = report!(my_error);
    /// let dyn_report: Report<Dynamic> = report.into_dynamic();
    /// ```
    #[must_use]
    pub fn into_dynamic(self) -> Report<Dynamic, O, T> {
        let raw = self.into_raw();

        // SAFETY:
        // 1. `C=Dynamic`, so this is trivially true.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. `C=Dynamic`, so this is trivially true.
        // 5. This is guaranteed by the invariants of this type.
        // 6. This is guaranteed by the invariants of this type.
        // 7. This is guaranteed by the invariants of this type.
        // 8. This is guaranteed by the invariants of this type.
        unsafe {
            // @add-unsafe-context: Dynamic
            Report::<Dynamic, O, T>::from_raw(raw)
        }
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
        // 1. This is guaranteed by the invariants of `self`.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. This is guaranteed by the invariants of `self`.
        // 4. This is guaranteed by the invariants of `self`.
        // 5. `O=Cloneable`, so this is trivially true.
        // 6. If the ownership of `self` is already `Cloneable`, then this is guaranteed
        //    by the invariants of `self`. If the ownership of `self` is `Mutable`, then
        //    the invariants of `self` guarantee that we are the only owner and we are
        //    consuming `self` in this method, so there are no other owners.
        // 7. This is guaranteed by the invariants of `self`.
        // 8. This is guaranteed by the invariants of `self`.
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
        // 3. `T=Local`, so this is trivially upheld.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        // 6. This is guaranteed by the invariants of this type.
        // 7. This is guaranteed by the invariants of this type.
        // 8. `T=Local`, so this is trivially true.
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
            // 2. `O=Mutable`, so this is trivially true.
            // 3. This is guaranteed by the invariants of this type.
            // 4. This is guaranteed by the invariants of this type.
            // 5. We just checked that the strong count is `1`, and we are taking ownership
            //    of `self`, so we are the unique owner.
            // 6. `O=Mutable`, so this is trivially upheld.
            // 7. This is guaranteed by the invariants of this type.
            // 8. This is guaranteed by the invariants of this type.
            let report = unsafe { Report::<C, Mutable, T>::from_raw(raw) };
            Ok(report)
        } else {
            Err(self)
        }
    }

    pub(crate) fn as_uncloneable_ref(&self) -> ReportRef<'_, C, Uncloneable, T> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. `O=Uncloneable`, so this is trivially true.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. `O=Uncloneable`, so this is trivially true.
        // 6. This is guaranteed by the invariants of this type.
        // 7. This is guaranteed by the invariants of this type.
        unsafe {
            // @add-unsafe-context: markers::ReportOwnershipMarker
            ReportRef::<C, Uncloneable, T>::from_raw(raw)
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
    pub fn as_ref(&self) -> ReportRef<'_, C, O::RefMarker, T>
    where
        O: markers::ReportOwnershipMarker,
    {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. If the ownership of `self` is `Mutable`, then the `O=Uncloneable`, which
        //    means that this is trivially true. On the other hand, if the ownership of
        //    `self` is `Cloneable`, then the `O=Cloneable`, which is also trivially
        //    true.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. If the ownership of `self` is `Mutable`, then the `O=Uncloneable`, which
        //    means that this is trivially true. On the other hand, if the ownership of
        //    `self` is `Cloneable`, then the `O=Cloneable`. However in that case, the
        //    invariants of this type guarantee that all references are compatible.
        // 6. This is guaranteed by the invariants of this type.
        // 7. This is guaranteed by the invariants of this type.
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
    /// root.children_mut().push(with_context1.into_dynamic().into_cloneable());
    /// root.children_mut().push(with_context2.into_dynamic().into_cloneable());
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
    pub fn iter_reports(&self) -> ReportIter<'_, O::RefMarker, T>
    where
        O: markers::ReportOwnershipMarker,
    {
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
    /// root.children_mut().push(with_context1.into_dynamic().into_cloneable());
    /// root.children_mut().push(with_context2.into_dynamic().into_cloneable());
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
    pub fn iter_sub_reports(&self) -> ReportIter<'_, Cloneable, T>
    where
        O: markers::ReportOwnershipMarker,
    {
        self.as_uncloneable_ref().iter_sub_reports()
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
    /// let mut report: Report<NonSendSyncError, Mutable, Local> = report!(non_send_sync_error);
    /// let preformatted: Report<PreformattedContext, Mutable, SendSync> = report.preformat();
    /// assert_eq!(format!("{report}"), format!("{preformatted}"));
    /// ```
    #[track_caller]
    #[must_use]
    pub fn preformat(&self) -> Report<PreformattedContext, Mutable, SendSync> {
        self.as_uncloneable_ref().preformat()
    }

    /// Returns the [`TypeId`] of the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::Dynamic};
    /// # use core::any::TypeId;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let type_id = report.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    ///
    /// let report: Report<Dynamic> = report.into_dynamic();
    /// let type_id = report.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    /// ```
    #[must_use]
    pub fn current_context_type_id(&self) -> TypeId {
        self.as_uncloneable_ref().current_context_type_id()
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
        self.as_uncloneable_ref().current_context_handler_type_id()
    }

    /// Returns the error source if the context implements [`Error`].
    ///
    /// [`Error`]: core::error::Error
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # use core::any::TypeId;
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
        self.as_uncloneable_ref().current_context_error_source()
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
        self.as_uncloneable_ref().format_current_context()
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
        self.as_uncloneable_ref().format_current_context_unhooked()
    }

    /// Formats the entire report using a specific report formatting hook.
    ///
    /// This method allows you to format a report with a custom formatter
    /// without globally registering it. This is useful for:
    /// - One-off custom formatting
    /// - Testing different formatters
    /// - Using different formatters in different parts of your application
    ///
    /// Unlike the default `Display` and `Debug` implementations which use the
    /// globally registered hook, this method uses the hook you provide
    /// directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{hooks::builtin_hooks::report_formatter::DefaultReportFormatter, prelude::*};
    ///
    /// let report = report!("error message");
    ///
    /// // Format with ASCII-only output (no Unicode or ANSI colors)
    /// let formatted = report.format_with(&DefaultReportFormatter::ASCII);
    /// println!("{}", formatted);
    /// ```
    #[must_use]
    pub fn format_with<H>(&self, hook: &H) -> impl core::fmt::Display + core::fmt::Debug
    where
        H: crate::hooks::report_formatter::ReportFormatter,
    {
        self.as_uncloneable_ref().format_with(hook)
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
        let report: ReportRef<'_, Dynamic, Uncloneable, Local> = self
            .as_uncloneable_ref()
            .into_dynamic()
            .into_uncloneable()
            .into_local();
        crate::hooks::context_formatter::get_preferred_context_formatting_style(
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
        self.as_uncloneable_ref()
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
        self.as_uncloneable_ref().strong_count()
    }
}

impl<C: Sized, O, T> Report<C, O, T> {
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
        self.as_uncloneable_ref().current_context()
    }
}

impl<O, T> Report<Dynamic, O, T> {
    /// Attempts to downcast the current context to a specific type.
    ///
    /// Returns `Some(&C)` if the current context is of type `C`, otherwise
    /// returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, markers::Dynamic};
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report<Dynamic> = report.into_dynamic();
    /// let context: Option<&MyError> = dyn_report.downcast_current_context();
    /// assert!(context.is_some());
    /// ```
    #[must_use]
    pub fn downcast_current_context<C>(&self) -> Option<&C>
    where
        C: Sized + 'static,
    {
        self.as_uncloneable_ref().downcast_current_context()
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
    /// # use core::any::TypeId;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report = report.into_dynamic();
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
        C: Sized + 'static,
    {
        let report = self.as_uncloneable_ref();

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
    /// let dyn_report: Report = report.into_dynamic();
    /// let downcasted: Result<Report<MyError>, _> = dyn_report.downcast_report();
    /// assert!(downcasted.is_ok());
    /// ```
    pub fn downcast_report<C>(self) -> Result<Report<C, O, T>, Self>
    where
        C: Sized + 'static,
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
    /// # use core::any::TypeId;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let dyn_report: Report = report.into_dynamic();
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
        C: Sized + 'static,
    {
        let raw = self.into_raw();

        // SAFETY:
        // 1. `C` is bounded by `Sized` in the function signature.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. Guaranteed by the caller
        // 5. This is guaranteed by the invariants of this type.
        // 6. This is guaranteed by the invariants of this type.
        // 7. This is guaranteed by the invariants of this type.
        // 8. This is guaranteed by the invariants of this type.
        unsafe { Report::from_raw(raw) }
    }
}

impl<C: Sized + Send + Sync> Report<C, Mutable, SendSync> {
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

impl<C: Sized> Report<C, Mutable, Local> {
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
unsafe impl<C: ?Sized, O> Send for Report<C, O, SendSync> {}

// SAFETY: The `SendSync` marker indicates that all objects in the report are
// `Send`+`Sync`. Therefore it is safe to implement `Send`+`Sync` for the report
// itself.
unsafe impl<C: ?Sized, O> Sync for Report<C, O, SendSync> {}

impl<C: Sized, T> From<C> for Report<C, Mutable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context)
    }
}

impl<C: Sized, T> From<C> for Report<C, Cloneable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context).into_cloneable()
    }
}

impl<C: Sized, T> From<C> for Report<Dynamic, Mutable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context).into_dynamic()
    }
}

impl<C: Sized, T> From<C> for Report<Dynamic, Cloneable, T>
where
    C: markers::ObjectMarkerFor<T> + core::error::Error,
{
    #[track_caller]
    fn from(context: C) -> Self {
        Report::new(context).into_dynamic().into_cloneable()
    }
}

impl<C: ?Sized, T> Clone for Report<C, Cloneable, T> {
    fn clone(&self) -> Self {
        self.as_ref().clone_arc()
    }
}

impl<C: ?Sized, O, T> core::fmt::Display for Report<C, O, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.as_uncloneable_ref(), f)
    }
}

impl<C: ?Sized, O, T> core::fmt::Debug for Report<C, O, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_uncloneable_ref(), f)
    }
}

impl<C: ?Sized, O, T> Unpin for Report<C, O, T> {}

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
    <C>: C => Dynamic, Mutable => Mutable, SendSync => SendSync, [into_dynamic],
    <C>: C => Dynamic, Mutable => Mutable, SendSync => Local, [into_dynamic, into_local],
    <C>: C => Dynamic, Mutable => Mutable, Local => Local, [into_dynamic],
    <C>: C => Dynamic, Mutable => Cloneable, SendSync => SendSync, [into_dynamic, into_cloneable],
    <C>: C => Dynamic, Mutable => Cloneable, SendSync => Local, [into_dynamic, into_cloneable, into_local],
    <C>: C => Dynamic, Mutable => Cloneable, Local => Local, [into_dynamic, into_cloneable],
    <C>: C => Dynamic, Cloneable => Cloneable, SendSync => SendSync, [into_dynamic, into_cloneable],
    <C>: C => Dynamic, Cloneable => Cloneable, SendSync => Local, [into_dynamic, into_cloneable, into_local],
    <C>: C => Dynamic, Cloneable => Cloneable, Local => Local, [into_dynamic, into_cloneable],
    <>:  Dynamic => Dynamic, Mutable => Mutable, SendSync => Local, [into_local],
    <>:  Dynamic => Dynamic, Mutable => Cloneable, SendSync => SendSync, [into_cloneable],
    <>:  Dynamic => Dynamic, Mutable => Cloneable, SendSync => Local, [into_cloneable, into_local],
    <>:  Dynamic => Dynamic, Mutable => Cloneable, Local => Local, [into_cloneable],
    <>:  Dynamic => Dynamic, Cloneable => Cloneable, SendSync => Local, [into_local],
);

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
        static_assertions::assert_impl_all!(Report<Dynamic, Mutable, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(Report<(), Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<(), Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<String, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<NonSend, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Mutable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Cloneable, Local>: Send, Sync);
    }

    #[test]
    fn test_report_unpin() {
        static_assertions::assert_impl_all!(Report<(), Mutable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(Report<(), Cloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(Report<String, Mutable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(Report<String, Cloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(Report<NonSend, Mutable, SendSync>: Unpin); // This still makes sense, since you won't actually be able to construct this report
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(Report<Dynamic, Mutable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, SendSync>: Unpin);

        static_assertions::assert_impl_all!(Report<(), Mutable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<(), Cloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<String, Mutable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<String, Cloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<NonSend, Mutable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<Dynamic, Mutable, Local>: Unpin);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, Local>: Unpin);
    }

    #[test]
    fn test_report_copy_clone() {
        static_assertions::assert_not_impl_any!(Report<(), Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<(), Mutable, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, Local>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Mutable, SendSync>: Copy, Clone);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Mutable, Local>: Copy, Clone);

        static_assertions::assert_impl_all!(Report<(), Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<(), Cloneable, Local>: Clone);
        static_assertions::assert_impl_all!(Report<String, Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<String, Cloneable, Local>: Clone);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, Local>: Clone);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, SendSync>: Clone);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, Local>: Clone);

        static_assertions::assert_not_impl_any!(Report<(), Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<(), Cloneable, Local>: Copy);
        static_assertions::assert_not_impl_any!(Report<String, Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<String, Cloneable, Local>: Copy);
        static_assertions::assert_not_impl_any!(Report<NonSend, Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<NonSend, Cloneable, Local>: Copy);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Cloneable, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Cloneable, Local>: Copy);
    }
}
