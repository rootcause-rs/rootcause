use core::any::{Any, TypeId};

use rootcause_internals::handlers::{ContextFormattingStyle, FormattingFunction};

use crate::{
    Report, ReportIter, ReportRef,
    markers::{self, Cloneable, Local, Mutable, SendSync, Uncloneable},
    preformatted::PreformattedContext,
    report_attachments::ReportAttachments,
    report_collection::ReportCollection,
    util::format_helper,
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::{any::Any, marker::PhantomData};

    use rootcause_internals::{RawReportMut, RawReportRef};

    use crate::markers::{self, SendSync};

    /// A mutable reference to a [`Report`].
    ///
    /// [`ReportMut`] provides mutable access to a report's children and
    /// attachments while maintaining safe borrowing semantics. Unlike owned
    /// reports, mutable references cannot be consumed for chaining
    /// operations like [`Report::context`] or [`Report::attach`].
    ///
    /// # Key Characteristics
    ///
    /// - **Not `Copy` or `Clone`**: Ensures exclusive mutable access
    /// - **Lifetime-bound**: Tied to the lifetime of the underlying report
    /// - **Two type parameters**: Has context type `C` and thread safety marker
    ///   `T` (no ownership marker since mutable references are always uniquely
    ///   owned)
    ///
    /// # Thread Safety
    ///
    /// Unlike [`Report`] and [`ReportRef`], you cannot change the thread safety
    /// marker on [`ReportMut`]:
    ///
    /// - You cannot convert [`SendSync`] → [`Local`] because that would allow
    ///   adding non-thread-safe data to a report that should remain thread-safe
    /// - You cannot convert [`Local`] → [`SendSync`] because that would allow
    ///   cloning a child report with thread-local data and sending it across
    ///   threads
    ///
    /// # Common Usage
    ///
    /// ```
    /// use rootcause::{ReportMut, prelude::*};
    ///
    /// let mut report: Report = report!("error message");
    ///
    /// // Get mutable access to modify children or attachments
    /// let mut report_mut: ReportMut<'_> = report.as_mut();
    /// report_mut
    ///     .children_mut()
    ///     .push(report!("child error").into_cloneable());
    ///
    /// println!("{}", report);
    /// ```
    #[repr(transparent)]
    pub struct ReportMut<'a, Context = dyn Any, ThreadSafety = SendSync>
    where
        Context: markers::ObjectMarker + ?Sized,
        ThreadSafety: markers::ThreadSafetyMarker,
    {
        /// # Safety
        ///
        /// The following safety invariants must be upheld as long as the
        /// inner `RawReportMut` exists:
        ///
        /// 1. If `C` is a concrete type: The context embedded in the report
        ///    must be of type `C`
        /// 2. The strong count of the underlying `triomphe::Arc` is exactly 1.
        /// 3. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 4. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`
        /// 5. If `T = Local`: No other references to this report are allowed to
        ///    have `T = SendSync`
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
        /// The caller must ensure:
        ///
        /// 1. If `C` is a concrete type: The context embedded in the report
        ///    must be of type `C`
        /// 2. The strong count of the underlying `triomphe::Arc` is exactly 1.
        /// 3. All references to any sub-reports of this report are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 4. If `T = SendSync`: All contexts and attachments in the report and
        ///    all sub-reports must be `Send+Sync`
        /// 5. If `T = Local`: No other references to this report are allowed to
        ///    have `T = SendSync`
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: RawReportMut<'a>) -> Self {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by our caller
            // 2. Guaranteed by our caller
            // 3. Guaranteed by our caller
            // 4. Guaranteed by our caller
            // 5. Guaranteed by our caller
            Self {
                raw,
                _context: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Creates a raw reference to the underlying report.
        #[must_use]
        pub(crate) fn as_raw_ref<'b>(&'b self) -> RawReportRef<'b> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. No mutation of the context occurs through the returned `RawReportRef`
            // 2. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    incompatible with shared ownership when `O=Mutable`, this cannot happen.
            // 3. Upheld, as this does not create any such references.
            // 4. No mutation of the report occurs through the returned `RawReportRef`
            // 5. Upheld, as this does not create any such references.
            let raw = &self.raw;

            raw.as_ref()
        }

        /// Consumes the [`ReportMut`] and returns the inner [`RawReportMut`].
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `T = SendSync`, no objects are added to the report through
        ///    this `RawReportMut` that are not `Send+Sync`
        #[must_use]
        pub(crate) unsafe fn into_raw_mut(self) -> RawReportMut<'a> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. While mutation of the context is possible through this reference, it is
            //    not possible to change the type of the context. Therefore this invariant
            //    is upheld.
            // 2. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    incompatible with shared ownership when `O=Mutable`, this cannot happen.
            // 3. We are not creating any such references here.
            // 4. Guaranteed by the caller
            // 5. Upheld, as this does not create any such references.

            self.raw
        }

        /// Creates a raw reference to the underlying report.
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `T = SendSync`, no objects are added to the report through
        ///    this `RawReportMut` that are not `Send+Sync`
        #[must_use]
        pub(crate) unsafe fn as_raw_mut<'b>(&'b mut self) -> RawReportMut<'b> {
            // SAFETY: We need to uphold the safety invariants of the raw field:
            // 1. While mutation of the context is possible through this reference, it is
            //    not possible to change the type of the context. Therefore this invariant
            //    is upheld.
            // 2. The only way to break this would be to call `RawReportRef::clone_arc`, but
            //    that method has a `safety` requirement that the caller must ensure that no
            //    owners exist which are incompatible with shared ownership. Since `self` is
            //    incompatible with shared ownership when `O=Mutable`, this cannot happen.
            // 3. Upheld, as this does not create any such references.
            // 4. Guaranteed by the caller
            // 5. Upheld, as this does not create any such references.
            let raw = &mut self.raw;

            raw.reborrow()
        }
    }
}
pub use limit_field_access::ReportMut;

impl<'a, C, T> ReportMut<'a, C, T>
where
    C: markers::ObjectMarker,
    T: markers::ThreadSafetyMarker,
{
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
    #[must_use]
    pub fn current_context(&self) -> &C {
        self.as_ref().current_context()
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
    #[must_use]
    pub fn current_context_mut(&mut self) -> &mut C {
        self.as_mut().into_current_context_mut()
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
    #[must_use]
    pub fn into_current_context_mut(self) -> &'a mut C {
        // SAFETY:
        // 1. We are not adding any objects
        let raw = unsafe { self.into_raw_mut() };

        // SAFETY:
        // 1. We know that `C` is a concrete type, so this is guaranteed by the
        //    invariants of this type.
        unsafe { raw.into_context_downcast_unchecked() }
    }
}

impl<'a, C, T> ReportMut<'a, C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Returns an immutable reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_collection::ReportCollection};
    /// let mut report: Report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// let children: &ReportCollection = report_mut.children();
    /// assert_eq!(children.len(), 0); // The report has just been created, so it has no children
    /// ```
    #[must_use]
    pub fn children(&self) -> &ReportCollection<dyn Any, T> {
        self.as_ref().children()
    }

    /// Returns a mutable reference to the child reports.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_collection::ReportCollection};
    /// # let mut report = report!("error message");
    /// let mut report_mut: ReportMut<'_> = report.as_mut();
    /// let children_mut: &mut ReportCollection = report_mut.children_mut();
    /// ```
    #[must_use]
    pub fn children_mut(&mut self) -> &mut ReportCollection<dyn Any, T> {
        self.as_mut().into_children_mut()
    }

    /// Consumes the [`ReportMut`] and returns a mutable reference to the child
    /// reports with the same lifetime.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_collection::ReportCollection};
    /// # let mut report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// let children_mut: &mut ReportCollection = report_mut.into_children_mut();
    /// ```
    #[must_use]
    pub fn into_children_mut(self) -> &'a mut ReportCollection<dyn Any, T> {
        // SAFETY:
        // 1. If `T=Local`: This is trivially true. If `T = SendSync`, then this is
        //    guaranteed by the `&mut ReportCollection` we are creating.
        let raw = unsafe { self.into_raw_mut() };

        // SAFETY:
        // 1. If `T=Local`: We know that no such references are allowed to exist, so
        //    this is trivially true. If `T=SendSync`, then these guarantees are upheld
        //    by the `&mut ReportCollection` we are creating.
        let raw_children = unsafe { raw.into_children_mut() };

        // SAFETY:
        // 1. `C=dyn Any`, so this is trivially true
        // 2. Guaranteed by the invariants of this type.
        // 3. Guaranteed by the invariants of this type.
        unsafe { ReportCollection::<dyn Any, T>::from_raw_mut(raw_children) }
    }

    /// Returns an immutable reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_attachments::ReportAttachments};
    /// # let mut report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// let attachments: &ReportAttachments = report_mut.attachments();
    /// ```
    #[must_use]
    pub fn attachments(&self) -> &ReportAttachments<T> {
        self.as_ref().attachments()
    }

    /// Returns a mutable reference to the attachments.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_attachments::ReportAttachments};
    /// # let mut report = report!("error message");
    /// let mut report_mut: ReportMut<'_> = report.as_mut();
    /// let attachments_mut: &mut ReportAttachments = report_mut.attachments_mut();
    /// ```
    #[must_use]
    pub fn attachments_mut(&mut self) -> &mut ReportAttachments<T> {
        self.as_mut().into_attachments_mut()
    }

    /// Consumes the [`ReportMut`] and returns a mutable reference to the
    /// attachments with the same lifetime.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{prelude::*, ReportMut, report_attachments::ReportAttachments};
    /// # let mut report = report!("error message");
    /// let report_mut: ReportMut<'_> = report.as_mut();
    /// let attachments_mut: &mut ReportAttachments = report_mut.into_attachments_mut();
    /// ```
    #[must_use]
    pub fn into_attachments_mut(self) -> &'a mut ReportAttachments<T> {
        // SAFETY:
        // 1. If `T=Local`: This is trivially true. If `T = SendSync`, then this is
        //    guaranteed by the `&mut ReportCollection` we are creating.
        let raw = unsafe { self.into_raw_mut() };

        // SAFETY:
        // 1. If `T=Local`: We know that no such references are allowed to exist, so
        //    this is trivially true. If `T=SendSync`, then these guarantees are upheld
        //    by the `&mut ReportCollection` we are creating.
        let raw = unsafe { raw.into_attachments_mut() };

        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T = SendSync`, then this is
        //    guaranteed by the invariants of this type.
        unsafe { ReportAttachments::from_raw_mut(raw) }
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
    /// the effect of "forgetting" that the context actually has the
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
    #[must_use]
    pub fn into_dyn_any(self) -> ReportMut<'a, dyn Any, T> {
        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T = SendSync`, then this is
        //    guaranteed by the invariants of the `ReportMut` we are creating.
        let raw = unsafe { self.into_raw_mut() };

        // SAFETY:
        // 1. `C=dyn Any`, so this is trivially true.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        unsafe { ReportMut::<dyn Any, T>::from_raw(raw) }
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
    #[must_use]
    pub fn as_ref(&self) -> ReportRef<'_, C, Uncloneable, T> {
        let raw = self.as_raw_ref();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. `O=Uncloneable`, so this is trivially true.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        unsafe { ReportRef::<C, Uncloneable, T>::from_raw(raw) }
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
    #[must_use]
    pub fn into_ref(self) -> ReportRef<'a, C, Uncloneable, T> {
        // SAFETY:
        // 1. We are creating an immutable reference just after this, so no mutation
        //    will occur through this `RawReportMut`.
        let raw = unsafe { self.into_raw_mut() };

        let raw = raw.into_ref();

        // SAFETY:
        // 1. This is guaranteed by the invariants of this type.
        // 2. `O=Uncloneable`, so this is trivially true.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        unsafe { ReportRef::<C, Uncloneable, T>::from_raw(raw) }
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
    ///     // Create a new mutable reference with a shorter lifetime
    ///     let mut borrowed_report_mut: ReportMut<'_, MyError> = report_mut.as_mut();
    /// }
    /// // After dropping the inner reference report, we can still use the outer one
    /// let _context: &MyError = report_mut.current_context();
    /// ```
    #[must_use]
    pub fn as_mut(&mut self) -> ReportMut<'_, C, T> {
        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T = SendSync`, then this is
        //    guaranteed by the invariants of the `ReportMut` we are creating.
        let raw = unsafe { self.as_raw_mut() };

        // SAFETY:
        // 1. If `C` is a concrete type: This is guaranteed by the invariants of this
        //    type.
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. If `T = SendSync`: This is guaranteed by the invariants of this type.
        // 5. If `T = Local`: This is guaranteed by the invariants of this type.
        unsafe { ReportMut::from_raw(raw) }
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
        self.as_ref().iter_reports()
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn format_current_context(&self) -> impl core::fmt::Display + core::fmt::Debug {
        let report = self.as_ref().into_dyn_any().into_uncloneable().into_local();
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
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let formatted = report_mut.format_current_context_unhooked();
    /// println!("{formatted}");
    /// ```
    #[must_use]
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
    #[must_use]
    pub fn preferred_context_formatting_style(
        &self,
        report_formatting_function: FormattingFunction,
    ) -> ContextFormattingStyle {
        let report = self.as_ref().into_dyn_any().into_uncloneable().into_local();
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
    /// let mut report: Report = report!("error message");
    /// let report_mut = report.as_mut();
    /// let style = report_mut
    ///     .preferred_context_formatting_style_unhooked(handlers::FormattingFunction::Display);
    /// ```
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn downcast_current_context<C>(&self) -> Option<&C>
    where
        C: markers::ObjectMarker,
    {
        self.as_ref().downcast_current_context()
    }

    /// Attempts to downcast the current context to a specific type.
    ///
    /// Returns `Some(&mut C)` if the current context is of type `C`, otherwise
    /// returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::prelude::*;
    /// # struct MyError;
    /// let report: Report<MyError> = report!(MyError);
    /// let mut dyn_report: Report = report.into_dyn_any();
    /// let mut_report = dyn_report.as_mut();
    /// let context: Option<&mut MyError> = mut_report.downcast_current_context_mut();
    /// assert!(context.is_some());
    /// ```
    #[must_use]
    pub fn downcast_current_context_mut<C>(&mut self) -> Option<&mut C>
    where
        C: markers::ObjectMarker,
    {
        let report = self.as_mut().downcast_report().ok()?;
        Some(report.into_current_context_mut())
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

    /// Downcasts the current context to a specific type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The current context is actually of type `C` (can be verified by
    ///    calling [`current_context_type_id()`] first)
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
    ///     let context: &mut MyError = unsafe { mut_report.downcast_current_context_mut_unchecked() };
    /// }
    /// ```
    pub unsafe fn downcast_current_context_mut_unchecked<C>(&mut self) -> &mut C
    where
        C: markers::ObjectMarker,
    {
        let report = self.as_mut();

        // SAFETY:
        // 1. Guaranteed by the caller
        let report = unsafe { report.downcast_report_unchecked() };

        report.into_current_context_mut()
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
        C: markers::ObjectMarker,
    {
        if TypeId::of::<C>() == self.current_context_type_id() {
            // SAFETY:
            // 1. We just verified that the type matches
            let report = unsafe { self.downcast_report_unchecked() };

            Ok(report)
        } else {
            Err(self)
        }
    }

    /// Downcasts the entire report to a specific context type without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    ///
    /// 1. The current context is actually of type `C` (can be verified by
    ///    calling [`current_context_type_id()`] first)
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
    #[must_use]
    pub unsafe fn downcast_report_unchecked<C>(self) -> ReportMut<'a, C, T>
    where
        C: markers::ObjectMarker,
    {
        // SAFETY:
        // 1. If `T=Local`, then this is trivially true. If `T = SendSync`, then this is
        //    guaranteed by the invariants of the `ReportMut` we are creating.
        let raw = unsafe { self.into_raw_mut() };

        // SAFETY:
        // 1. This is guaranteed by the caller
        // 2. This is guaranteed by the invariants of this type.
        // 3. This is guaranteed by the invariants of this type.
        // 4. This is guaranteed by the invariants of this type.
        // 5. This is guaranteed by the invariants of this type.
        unsafe { ReportMut::from_raw(raw) }
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

impl<'a, C: markers::ObjectMarker> From<ReportMut<'a, C, SendSync>>
    for ReportMut<'a, dyn Any, SendSync>
{
    fn from(report: ReportMut<'a, C, SendSync>) -> Self {
        report.into_dyn_any()
    }
}

impl<'a, C: markers::ObjectMarker> From<ReportMut<'a, C, Local>> for ReportMut<'a, dyn Any, Local> {
    fn from(report: ReportMut<'a, C, Local>) -> Self {
        report.into_dyn_any()
    }
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
