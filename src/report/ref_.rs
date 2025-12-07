use alloc::vec;
use core::any::TypeId;

use rootcause_internals::handlers::{ContextFormattingStyle, FormattingFunction};

use crate::{
    Report, ReportIter,
    markers::{Cloneable, Dynamic, Local, Mutable, SendSync, Uncloneable},
    preformatted::{self, PreformattedAttachment, PreformattedContext},
    report_attachment::ReportAttachment,
    report_attachments::ReportAttachments,
    report_collection::ReportCollection,
    util::format_helper,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::{RawReport, RawReportRef};

    use crate::markers::{Cloneable, Dynamic, SendSync};

    /// A reference to a [`Report`].
    ///
    /// [`ReportRef`] is a lightweight, copyable reference to a report that
    /// allows you to inspect report data without taking ownership. It's the
    /// primary way to work with reports in a read-only manner.
    ///
    /// # Key Characteristics
    ///
    /// - **Always `Copy` + `Clone`**: Unlike [`Report`], all [`ReportRef`]
    ///   instances can be freely copied regardless of their ownership marker
    /// - **Lifetime-bound**: Tied to the lifetime of the underlying report
    /// - **Type parameters**: Like [`Report`], has context type `C`, ownership
    ///   marker `O`, and thread safety marker `T`
    ///
    /// # Ownership Markers
    ///
    /// The ownership marker on [`ReportRef`] indicates what the *underlying*
    /// report's ownership status is:
    ///
    /// - [`Cloneable`]: The underlying report is shared (can use [`clone_arc`]
    ///   to get an owned [`Report`])
    /// - [`Uncloneable`]: The underlying report has unique ownership (cannot
    ///   use [`clone_arc`])
    ///
    /// Note that when you create a [`ReportRef`] from a [`Report`] marked as
    /// [`Mutable`], it becomes a [`ReportRef`] with the [`Uncloneable`] marker
    /// to prevent cloning while mutable access exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{ReportRef, markers::Uncloneable, prelude::*};
    ///
    /// let report: Report = report!("error message");
    ///
    /// // Get a reference - this is Uncloneable because report is Mutable
    /// let report_ref: ReportRef<'_, _, Uncloneable> = report.as_ref();
    ///
    /// // Inspect the report
    /// println!("{}", report_ref);
    /// assert_eq!(report_ref.children().len(), 0);
    /// ```
    ///
    /// [`Report`]: crate::Report
    /// [`Cloneable`]: crate::markers::Cloneable
    /// [`Uncloneable`]: crate::markers::Uncloneable
    /// [`Mutable`]: crate::markers::Mutable
    /// [`clone_arc`]: ReportRef::clone_arc
    // # Safety invariants
    //
    // This reference behaves like a `&'a Report<C, O, T>` for some unknown
    // `C` and `O`, and upholds the usual safety invariants of shared references:
    //
    // 1. The pointee is properly initialized for the entire lifetime `'a`.
    // 2. The pointee is not mutated for the entire lifetime `'a`.
    #[repr(transparent)]
    pub struct ReportRef<
        'a,
        Context: ?Sized + 'static = Dynamic,
        Ownership: 'static = Cloneable,
        ThreadSafety: 'static = SendSync,
    > {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. `C` must either be a type bounded by `Sized + 'static`, or
        ///    `Dynamic`.
        /// 2. `O` must either be `Cloneable` or `Uncloneable`.
        /// 3. `T` must either be `SendSync` or `Local`.
        /// 4. If `C` is a `Sized` type: The context embedded in the report must
        ///    be of type `C`
        /// 5. If `O = Cloneable`: All other references to this report are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 6. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 7. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`
        ///
        /// [`RawReport`]: rootcause_internals::RawReport
        /// [`Mutable`]: crate::markers::Mutable
        raw: RawReportRef<'a>,
        _context: PhantomData<Context>,
        _ownership: PhantomData<Ownership>,
        _thread_safety: PhantomData<ThreadSafety>,
    }

    impl<'a, C: ?Sized, O, T> ReportRef<'a, C, O, T> {
        /// Creates a new Report from a raw report
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. `C` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. `O` must either be `Cloneable` or `Uncloneable`.
        /// 3. `T` must either be `SendSync` or `Local`.
        /// 4. If `C` is a `Sized` type: The context embedded in the report must
        ///    be of type `C`
        /// 5. If `O = Cloneable`: All other references to this report are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 6. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 7. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`
        ///
        /// [`Report`]: crate::Report
        /// [`RawReport`]: rootcause_internals::RawReport
        /// [`ReportMut`]: crate::ReportMut
        /// [`Mutable`]: crate::markers::Mutable
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawReportRef<'a>) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by our caller
            // 2. Guaranteed by our caller
            // 3. Guaranteed by our caller
            // 4. Guaranteed by our caller
            // 5. Guaranteed by our caller
            // 6. Guaranteed by our caller
            // 7. Guaranteed by our caller
            Self {
                raw,
                _context: PhantomData,
                _ownership: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Creates a slice of [`ReportRef`] from a slice of [`RawReport`].
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. `C` must either be a type bounded by `Sized`, or `Dynamic`.
        /// 2. `O` must either be `Cloneable` or `Uncloneable`.
        /// 3. `T` must either be `SendSync` or `Local`.
        /// 4. If `C` is a `Sized` type: The contexts embedded in all of the
        ///    [`RawReport`]s in the slice are of type `C`
        /// 5. If `O = Cloneable`: All other references to these reports are
        ///    compatible with shared ownership. Specifically there are no
        ///    references with an assumption that the strong_count is `1`.
        /// 6. All references to any sub-reports of these reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 7. If `T = SendSync`: All contexts and attachments in these reports
        ///    and all sub-reports must be `Send+Sync`
        pub(crate) unsafe fn from_raw_slice(raw: &'a [RawReport]) -> &'a [ReportRef<'a, C, O, T>] {
            let len = raw.len();
            let raw_ptr: *const RawReport = raw.as_ptr();

            // SAFETY: We must uphold the safety invariants of the raw field for
            // all reports in the slice:
            // 1. Guaranteed by our caller
            // 2. Guaranteed by our caller
            // 3. Guaranteed by our caller
            // 4. Guaranteed by our caller
            // 5. Guaranteed by our caller
            // 6. Guaranteed by our caller
            // 7. Guaranteed by our caller
            let report_ref_ptr = raw_ptr.cast::<ReportRef<'a, C, O, T>>();

            // SAFETY:
            // 1. The pointer is valid and properly aligned because it points to the first
            //    element of a valid slice of `RawReport`s
            // 2. The length is correct because we obtained it from the original slice of
            //    `RawReport`s
            // 3. Each `ReportRef` is `repr(transparent)` over `RawReportRef`, which is
            //    repr(transparent) over the same underlying pointer as `RawReport`, so the
            //    alignment and validity are preserved
            // 4. We are not creating mutable references, so there are no aliasing issues to
            //    consider
            // 5. The safety invariants for each `ReportRef` are upheld as guaranteed by our
            //    caller
            unsafe {
                // @add-unsafe-context: RawReport
                // @add-unsafe-context: RawReportRef
                core::slice::from_raw_parts(report_ref_ptr, len)
            }
        }

        /// Returns the underlying raw report reference.
        #[must_use]
        pub(crate) fn as_raw_ref(self) -> RawReportRef<'a> {
            // SAFETY: We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }
    }

    // SAFETY: We must uphold the safety invariants of the raw field for both the
    // original and the copy:
    // 1. This remains true for both the original and the copy
    // 2. This remains true for both the original and the copy
    // 3. This remains true for both the original and the copy
    // 4. This remains true for both the original and the copy
    // 5. This remains true for both the original and the copy
    // 6. This remains true for both the original and the copy
    // 7. This remains true for both the original and the copy
    impl<'a, C: ?Sized, O, T> Copy for ReportRef<'a, C, O, T> {}
}
pub use limit_field_access::ReportRef;

impl<'a, C: ?Sized, O, T> Clone for ReportRef<'a, C, O, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C: Sized, O, T> ReportRef<'a, C, O, T> {
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
    #[must_use]
    pub fn current_context(self) -> &'a C {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        unsafe { raw.context_downcast_unchecked() }
    }
}

impl<'a, C: ?Sized, O, T> ReportRef<'a, C, O, T> {
    /// Maps a Cloneable report reference to a report reference with any
    /// ownership
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. `O` must either be `Cloneable` or `Uncloneable`.
    pub(crate) unsafe fn from_cloneable(
        report: ReportRef<'a, C, Cloneable, T>,
    ) -> ReportRef<'a, C, O, T> {
        let raw = report.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by the safety invariants of the argument
        // 2. This is guaranteed by the caller
        // 3. This is guaranteed by the safety invariants of the argument
        // 4. This is guaranteed by the safety invariants of the argument
        // 5. If `O = Cloneable`: This is guaranteed by the safety invariants of the
        //    argument. If `O = Uncloneable`: This is trivially true. The caller
        //    guarantees that these are the only possibilities.
        // 6. This is guaranteed by the safety invariants of the argument
        // 7. This is guaranteed by the safety invariants of the argument
        unsafe { ReportRef::<'a, C, O, T>::from_raw(raw) }
    }

    /// Returns a reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, report_collection::ReportCollection};
    /// let report = report!("parent error").into_cloneable();
    /// let report_ref: ReportRef<'_, _, _> = report.as_ref();
    /// let children: &ReportCollection = report_ref.children();
    /// assert_eq!(children.len(), 0); // The report has just been created, so it has no children
    /// ```
    #[must_use]
    pub fn children(self) -> &'a ReportCollection<Dynamic, T> {
        let raw = self.as_raw_ref().children();

        // SAFETY:
        // 1. This is guaranteed by our safety invariants.
        // 2. The invariants of this type guarantee that `T` is either `SendSync` or
        //    `Local`.
        // 3. `C=Dynamic`, so this is trivially true.
        // 4. This is guaranteed by our safety invariants.
        // 5. This is guaranteed by our safety invariants.
        unsafe { ReportCollection::<Dynamic, T>::from_raw_ref(raw) }
    }

    /// Returns a reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, report_attachments::ReportAttachments};
    /// # let report = report!("error with attachment").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let attachments: &ReportAttachments = report_ref.attachments();
    /// ```
    #[must_use]
    pub fn attachments(self) -> &'a ReportAttachments<T> {
        let raw = self.as_raw_ref().attachments();

        // SAFETY:
        // 1. `T` is guaranteed to either be `Local` or `SendSync` by the invariants of
        //    this type.
        // 2. This is guaranteed by our own safety invariants.
        unsafe { ReportAttachments::<T>::from_raw_ref(raw) }
    }

    /// Changes the context type of the [`ReportRef`] to [`Dynamic`].
    ///
    /// Calling this method is equivalent to calling `report.into()`, however
    /// this method has been restricted to only change the context mode to
    /// [`Dynamic`].
    ///
    /// This method can be useful to help with type inference or to improve code
    /// readability, as it more clearly communicates intent.
    ///
    /// This method does not actually modify the report in any way. It only has
    /// the effect of "forgetting" that the context actually has the
    /// type `C`.
    ///
    /// To get back the report with a concrete `C` you can use the method
    /// [`ReportRef::downcast_report`].
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Dynamic};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, Dynamic> = report_ref.into_dynamic();
    /// ```
    #[must_use]
    pub fn into_dynamic(self) -> ReportRef<'a, Dynamic, O, T> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. `C=Dynamic`, so this is trivially true.
        // 2. This is guaranteed by our own safety invariants.
        // 3. This is guaranteed by our own safety invariants.
        // 4. `C=Dynamic`, so this is trivially true.
        // 5. This is guaranteed by our own safety invariants.
        // 6. This is guaranteed by our own safety invariants.
        // 7. This is guaranteed by our own safety invariants.
        unsafe {
            // @add-unsafe-context: Dynamic
            ReportRef::<Dynamic, O, T>::from_raw(raw)
        }
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
    /// the effect of "forgetting" that the [`ReportRef`] references a report
    /// that could potentially be cloned via [`clone_arc`].
    ///
    /// This is useful when you need a type that explicitly cannot use
    /// [`clone_arc`], typically for API boundaries or when working with
    /// mutable reports.
    ///
    /// [`clone_arc`]: ReportRef::clone_arc
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::{Uncloneable, Cloneable}};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError, Cloneable> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, MyError, Uncloneable> = report_ref.into_uncloneable();
    /// ```
    #[must_use]
    pub fn into_uncloneable(self) -> ReportRef<'a, C, Uncloneable, T> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by our own safety invariants.
        // 2. `O=Uncloneable`, so this is trivially true.
        // 3. This is guaranteed by our own safety invariants.
        // 4. This is guaranteed by our own safety invariants.
        // 5. `O=Uncloneable`, so this is trivially true.
        // 6. This is guaranteed by our own safety invariants.
        // 7. This is guaranteed by our own safety invariants.
        unsafe { ReportRef::<C, Uncloneable, T>::from_raw(raw) }
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
    /// This is useful when you need to work with a report reference in a
    /// context that doesn't require [`Send`] + [`Sync`], or when the report
    /// may contain thread-local data that isn't actually being used.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::{SendSync, Local}};
    /// # let report = report!("my error").into_cloneable();
    /// let report_ref: ReportRef<'_, _, _, SendSync> = report.as_ref();
    /// let local_report_ref: ReportRef<'_, _, _, Local> = report_ref.into_local();
    /// ```
    #[must_use]
    pub fn into_local(self) -> ReportRef<'a, C, O, Local> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by our own safety invariants.
        // 2. This is guaranteed by our own safety invariants.
        // 3. `T=Local`, so this is trivially true.
        // 4. This is guaranteed by our own safety invariants.
        // 5. This is guaranteed by our own safety invariants.
        // 6. This is guaranteed by our own safety invariants.
        // 7. `T=Local`, so this is trivially true.
        unsafe { ReportRef::<C, O, Local>::from_raw(raw) }
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
    ///     .push(with_context1.into_dynamic().into_cloneable());
    /// root.children_mut()
    ///     .push(with_context2.into_dynamic().into_cloneable());
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
        let stack = vec![self.into_dynamic()];
        ReportIter::from_raw(stack)
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
    ///     .push(with_context1.into_dynamic().into_cloneable());
    /// root.children_mut()
    ///     .push(with_context2.into_dynamic().into_cloneable());
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
        let stack = self.children().iter().rev().collect();
        ReportIter::from_raw(stack)
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
                    .into_dynamic()
                })
                .collect(),
        )
    }

    /// Returns the [`TypeId`] of the current context.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Dynamic};
    /// # use core::any::TypeId;
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let type_id = report_ref.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    ///
    /// let report_ref: ReportRef<'_, Dynamic> = report_ref.into_dynamic();
    /// let type_id = report_ref.current_context_type_id();
    /// assert_eq!(type_id, TypeId::of::<MyError>());
    /// ```
    #[must_use]
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
    #[must_use]
    pub fn current_context_handler_type_id(self) -> TypeId {
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
    #[must_use]
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
    #[must_use]
    pub fn format_current_context(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.into_dynamic().into_uncloneable().into_local(),
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
    /// # use rootcause::{prelude::*, ReportRef};
    /// # let report = report!("error message").into_cloneable();
    /// let report_ref: ReportRef<'_> = report.as_ref();
    /// let formatted = report_ref.format_current_context_unhooked();
    /// println!("{formatted}");
    /// ```
    #[must_use]
    pub fn format_current_context_unhooked(self) -> impl core::fmt::Display + core::fmt::Debug {
        format_helper(
            self.as_raw_ref(),
            |report, formatter| report.context_display(formatter),
            |report, formatter| report.context_debug(formatter),
        )
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
    /// let report = report!("error message").into_cloneable();
    /// let report_ref = report.as_ref();
    ///
    /// // Format with ASCII-only output (no Unicode or ANSI colors)
    /// let formatted = report_ref.format_with_hook(&DefaultReportFormatter::ASCII);
    /// println!("{}", formatted);
    /// ```
    #[must_use]
    pub fn format_with_hook<H: crate::hooks::report_formatting::ReportFormatter>(
        self,
        hook: &H,
    ) -> impl core::fmt::Display + core::fmt::Debug {
        let report = self.into_dynamic().into_uncloneable().into_local();
        format_helper(
            (report, hook),
            |(report, hook), formatter| {
                hook.format_report(report, formatter, FormattingFunction::Display)
            },
            |(report, hook), formatter| {
                hook.format_report(report, formatter, FormattingFunction::Debug)
            },
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
    #[must_use]
    pub fn preferred_context_formatting_style(
        self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        crate::hooks::formatting_overrides::context::get_preferred_context_formatting_style(
            self.into_dynamic().into_uncloneable().into_local(),
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
    #[must_use]
    pub fn preferred_context_formatting_style_unhooked(
        self,
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
    #[must_use]
    pub fn strong_count(self) -> usize {
        self.as_raw_ref().strong_count()
    }
}

impl<'a, O, T> ReportRef<'a, Dynamic, O, T> {
    /// Attempts to downcast the current context to a specific type.
    ///
    /// Returns `Some(&C)` if the current context is of type `C`, otherwise
    /// returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Dynamic};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, Dynamic> = report_ref.into_dynamic();
    /// let context: Option<&MyError> = dyn_report_ref.downcast_current_context();
    /// assert!(context.is_some());
    /// ```
    #[must_use]
    pub fn downcast_current_context<C>(self) -> Option<&'a C>
    where
        C: Sized + 'static,
    {
        let report = self.downcast_report()?;
        Some(report.current_context())
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
    /// [`current_context_type_id()`]: ReportRef::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Dynamic};
    /// # use core::any::TypeId;
    /// # struct MyError;
    /// # let report = report!(MyError).into_dynamic().into_cloneable();
    /// let report_ref: ReportRef<'_, Dynamic> = report.as_ref();
    ///
    /// // Verify the type first
    /// if report_ref.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let context: &MyError = unsafe { report_ref.downcast_current_context_unchecked() };
    /// }
    /// ```
    #[must_use]
    pub unsafe fn downcast_current_context_unchecked<C>(self) -> &'a C
    where
        C: Sized + 'static,
    {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by the caller.
        unsafe {
            // @add-unsafe-context: markers::ObjectMarker
            raw.context_downcast_unchecked()
        }
    }

    /// Attempts to downcast the report to a specific context type.
    ///
    /// Returns `Some(report_ref)` if the current context is of type `C`,
    /// otherwise returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Dynamic};
    /// # struct MyError;
    /// # let report = report!(MyError).into_cloneable();
    /// let report_ref: ReportRef<'_, MyError> = report.as_ref();
    /// let dyn_report_ref: ReportRef<'_, Dynamic> = report_ref.into_dynamic();
    /// let downcasted: Option<ReportRef<'_, MyError>> = dyn_report_ref.downcast_report::<MyError>();
    /// assert!(downcasted.is_some());
    /// ```
    #[must_use]
    pub fn downcast_report<C>(self) -> Option<ReportRef<'a, C, O, T>>
    where
        C: Sized,
    {
        if TypeId::of::<C>() == self.current_context_type_id() {
            // SAFETY:
            // 1. We just verified that the type matches.
            let report = unsafe { self.downcast_report_unchecked() };

            Some(report)
        } else {
            None
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
    /// [`current_context_type_id()`]: ReportRef::current_context_type_id
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportRef, markers::Dynamic};
    /// # use core::any::TypeId;
    /// # struct MyError;
    /// # let report = report!(MyError).into_dynamic().into_cloneable();
    /// let report_ref: ReportRef<'_, Dynamic> = report.as_ref();
    ///
    /// // Verify the type first
    /// if report_ref.current_context_type_id() == TypeId::of::<MyError>() {
    ///     // SAFETY: We verified the type matches
    ///     let downcasted = unsafe { report_ref.downcast_report_unchecked::<MyError>() };
    /// }
    /// ```
    #[must_use]
    pub unsafe fn downcast_report_unchecked<C>(self) -> ReportRef<'a, C, O, T>
    where
        C: Sized,
    {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. `C` is bounded by `Sized` in the function signature.
        // 2. This is guaranteed by our own safety invariants.
        // 3. This is guaranteed by our own safety invariants.
        // 4. Guaranteed by the caller.
        // 5. Guaranteed by our own safety invariants.
        // 6. Guaranteed by our own safety invariants.
        // 7. Guaranteed by our own safety invariants.
        unsafe { ReportRef::<C, O, T>::from_raw(raw) }
    }
}

impl<'a, C: ?Sized, T> ReportRef<'a, C, Cloneable, T> {
    /// Clones the underlying [`triomphe::Arc`] of the report, returning
    /// a new owned [`Report`] that references the same root node.
    ///
    /// This method is only available when the ownership marker is
    /// [`Cloneable`], indicating that the underlying report can be safely
    /// cloned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{ReportRef, markers::Cloneable, prelude::*};
    ///
    /// let report1: Report<_, Cloneable> = report!("error").into_cloneable();
    /// let report_ref: ReportRef<'_, _, Cloneable> = report1.as_ref();
    ///
    /// // Clone the Arc to get a new owned Report
    /// let report2: Report<_, Cloneable> = report_ref.clone_arc();
    ///
    /// // Both reports reference the same underlying data
    /// assert_eq!(format!("{}", report1), format!("{}", report2));
    /// ```
    ///
    /// [`Cloneable`]: crate::markers::Cloneable
    #[must_use]
    pub fn clone_arc(self) -> Report<C, Cloneable, T> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. Since `O=Cloneable`, this is guaranteed by our own safety invariants.
        let cloned_raw = unsafe { raw.clone_arc() };

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of this type.
        // 4. This is guaranteed by our own safety invariants.
        // 5. `O=Cloneable`, so this is trivially true.
        // 6. This is guaranteed by our own safety invariants.
        // 7. This is guaranteed by our own safety invariants.
        // 8. This is guaranteed by our own safety invariants.
        unsafe { Report::<C, Cloneable, T>::from_raw(cloned_raw) }
    }
}

impl<'a, C: ?Sized, T> From<ReportRef<'a, C, Cloneable, T>> for Report<C, Cloneable, T> {
    fn from(report: ReportRef<'a, C, Cloneable, T>) -> Self {
        report.clone_arc()
    }
}

impl<'a, C: ?Sized, O, T> core::fmt::Display for ReportRef<'a, C, O, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let report = self.into_dynamic().into_uncloneable().into_local();
        crate::hooks::report_formatting::format_report(report, f, FormattingFunction::Display)
    }
}

impl<'a, C: ?Sized, O, T> core::fmt::Debug for ReportRef<'a, C, O, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let report = self.into_dynamic().into_uncloneable().into_local();
        crate::hooks::report_formatting::format_report(report, f, FormattingFunction::Debug)
    }
}

impl<'a, C: ?Sized, O, T> Unpin for ReportRef<'a, C, O, T> {}

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
            impl<'a, $($param),*> From<ReportRef<'a, $context1, $ownership1, $thread_safety1>> for ReportRef<'a, $context2, $ownership2, $thread_safety2>
            {
                fn from(report: ReportRef<'a, $context1, $ownership1, $thread_safety1>) -> Self {
                    report
                        $(
                            .$op()
                        )*
                }
            }
        )*
    };
}

from_impls!(
    <C>: C => C, Cloneable => Cloneable, SendSync => Local, [into_local],
    <C>: C => C, Cloneable => Uncloneable, SendSync => SendSync, [into_uncloneable],
    <C>: C => C, Cloneable => Uncloneable, SendSync => Local, [into_uncloneable, into_local],
    <C>: C => C, Cloneable => Uncloneable, Local => Local, [into_uncloneable],
    <C>: C => C, Uncloneable => Uncloneable, SendSync => Local, [into_local],
    <C>: C => Dynamic, Cloneable => Cloneable, SendSync => SendSync, [into_dynamic],
    <C>: C => Dynamic, Cloneable => Cloneable, SendSync => Local, [into_dynamic, into_local],
    <C>: C => Dynamic, Cloneable => Cloneable, Local => Local, [into_dynamic],
    <C>: C => Dynamic, Cloneable => Uncloneable, SendSync => SendSync, [into_dynamic, into_uncloneable],
    <C>: C => Dynamic, Cloneable => Uncloneable, SendSync => Local, [into_dynamic, into_uncloneable, into_local],
    <C>: C => Dynamic, Cloneable => Uncloneable, Local => Local, [into_dynamic, into_uncloneable],
    <C>: C => Dynamic, Uncloneable => Uncloneable, SendSync => SendSync, [into_dynamic],
    <C>: C => Dynamic, Uncloneable => Uncloneable, SendSync => Local, [into_dynamic, into_local],
    <C>: C => Dynamic, Uncloneable => Uncloneable, Local => Local, [into_dynamic, into_uncloneable],
    <>:  Dynamic => Dynamic, Cloneable => Cloneable, SendSync => Local, [into_local],
    <>:  Dynamic => Dynamic, Cloneable => Uncloneable, SendSync => SendSync, [into_uncloneable],
    <>:  Dynamic => Dynamic, Cloneable => Uncloneable, SendSync => Local, [into_uncloneable, into_local],
    <>:  Dynamic => Dynamic, Cloneable => Uncloneable, Local => Local, [into_uncloneable],
    <>:  Dynamic => Dynamic, Uncloneable => Uncloneable, SendSync => Local, [into_local],
);

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
        static_assertions::assert_not_impl_any!(ReportRef<'static, Dynamic, Uncloneable, SendSync>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, Dynamic, Cloneable, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportRef<'static, (), Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, (), Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, String, Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, String, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, NonSend, Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, NonSend, Cloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, Dynamic, Uncloneable, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportRef<'static, Dynamic, Cloneable, Local>: Send, Sync);
    }

    #[test]
    fn test_report_ref_unpin() {
        static_assertions::assert_impl_all!(ReportRef<'static, (), Uncloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, (), Cloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Uncloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Cloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Uncloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Cloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Uncloneable, SendSync>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Cloneable, SendSync>: Unpin);

        static_assertions::assert_impl_all!(ReportRef<'static, (), Uncloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, (), Cloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Uncloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, String, Cloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Uncloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, NonSend, Cloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Uncloneable, Local>: Unpin);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Cloneable, Local>: Unpin);
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
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Uncloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Cloneable, SendSync>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Uncloneable, Local>: Copy, Clone);
        static_assertions::assert_impl_all!(ReportRef<'static, Dynamic, Cloneable, Local>: Copy, Clone);
    }

    #[test]
    fn test_report_ref_into_report() {
        static_assertions::assert_impl_all!(Report<(), Cloneable, Local>: From<ReportRef<'static, (), Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<(), Cloneable, SendSync>: From<ReportRef<'static, (), Cloneable, SendSync>>);
        static_assertions::assert_impl_all!(Report<String, Cloneable, Local>: From<ReportRef<'static, String, Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<String, Cloneable, SendSync>: From<ReportRef<'static, String, Cloneable, SendSync>>);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, Local>: From<ReportRef<'static, NonSend, Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<NonSend, Cloneable, SendSync>: From<ReportRef<'static, NonSend, Cloneable, SendSync>>);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, Local>: From<ReportRef<'static, Dynamic, Cloneable, Local>>);
        static_assertions::assert_impl_all!(Report<Dynamic, Cloneable, SendSync>: From<ReportRef<'static, Dynamic, Cloneable, SendSync>>);

        static_assertions::assert_not_impl_any!(Report<(), Mutable, Local>: From<ReportRef<'static, (), Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<(), Mutable, SendSync>: From<ReportRef<'static, (), Uncloneable, SendSync>>);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, Local>: From<ReportRef<'static, String, Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<String, Mutable, SendSync>: From<ReportRef<'static, String, Uncloneable, SendSync>>);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, Local>: From<ReportRef<'static, NonSend, Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<NonSend, Mutable, SendSync>: From<ReportRef<'static, NonSend, Uncloneable, SendSync>>);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Mutable, Local>: From<ReportRef<'static, Dynamic, Uncloneable, Local>>);
        static_assertions::assert_not_impl_any!(Report<Dynamic, Mutable, SendSync>: From<ReportRef<'static, Dynamic, Uncloneable, SendSync>>);
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
