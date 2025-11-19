use alloc::vec::Vec;
use core::any::Any;

use rootcause_internals::handlers::{ContextHandler, FormattingFunction};

use crate::{
    Report, ReportRef, handlers,
    markers::{self, Cloneable, Local, Mutable, SendSync, Uncloneable},
    report_attachments::ReportAttachments,
    report_collection::{ReportCollectionIntoIter, ReportCollectionIter},
};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use alloc::vec::Vec;
    use core::{any::Any, marker::PhantomData};

    use rootcause_internals::RawReport;

    use crate::markers::{self, SendSync};

    /// A collection of reports.
    ///
    /// You can think of a [`ReportCollection<C, T>`] as a wrapper around a
    /// `Vec<Report<C, markers::Cloneable, T>>`, however, it has a slightly
    /// different API:
    /// - It provides methods such as [`context`](Self::context) and
    ///   [`context_custom`](Self::context_custom) to create new reports with
    ///   the collection as children.
    /// - It has convenience methods to convert between different context and
    ///   thread safety markers such as [`into_dyn_any`](Self::into_dyn_any) and
    ///   [`into_local`](Self::into_local).
    /// - It is also possible to convert between different context and thread
    ///   safety markers using the [`From`] and [`Into`] traits.
    #[repr(transparent)]
    pub struct ReportCollection<Context = dyn Any, ThreadSafety = SendSync>
    where
        Context: markers::ObjectMarker + ?Sized,
        ThreadSafety: markers::ThreadSafetyMarker,
    {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long as this
        /// struct exists:
        ///
        /// 1. If `C` is a concrete type: The contexts contained in all of the
        ///    reports in the `Vec` are of type `C`.
        /// 2. All references to these reports or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 3. If `T = SendSync`: All contexts and attachments in the all of the
        ///    report and all sub-reports must be `Send+Sync`
        raw: Vec<RawReport>,
        _context: PhantomData<Context>,
        _thread_safety: PhantomData<ThreadSafety>,
    }

    impl<C, T> ReportCollection<C, T>
    where
        C: markers::ObjectMarker + ?Sized,
        T: markers::ThreadSafetyMarker,
    {
        /// Creates a new [`ReportCollection`] from a vector of raw reports
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `C` is a concrete type: The contexts contained in all of the
        ///    reports in the `Vec` are of type `C`.
        /// 2. All references to these reports or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 3. If `T = SendSync`: All contexts and attachments in the all of the
        ///    report and all sub-reports must be `Send+Sync`
        #[must_use]
        pub(crate) unsafe fn from_raw(raw: Vec<RawReport>) -> Self {
            Self {
                raw,
                _context: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Creates a reference to [`ReportCollection`] from reference to a
        /// vector of raw reports
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `C` is a concrete type: The contexts contained in all of the
        ///    reports in the `Vec` are of type `C`.
        /// 2. All references to these reports or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 3. If `T = SendSync`: All contexts and attachments in the all of the
        ///    report and all sub-reports must be `Send+Sync`
        #[must_use]
        pub(crate) unsafe fn from_raw_ref(raw: &Vec<RawReport>) -> &Self {
            let raw_ptr = core::ptr::from_ref(raw).cast::<Self>();

            // SAFETY:
            // - The raw pointer is derived from a valid reference with the same lifetime
            //   and representation
            // - Creating this reference does not violate any aliasing rules as we are only
            //   creating a shared reference
            // - The type invariants of `Self` are upheld as per the caller's guarantee
            unsafe { &*raw_ptr }
        }

        /// Creates a mutable reference to [`ReportCollection`] from mutable
        /// reference
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `C` is a concrete type: The contexts contained in all of the
        ///    reports in the `Vec` are of type `C`.
        /// 2. All references to these reports or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 3. If `T = SendSync`: All contexts and attachments in the all of the
        ///    report and all sub-reports must be `Send+Sync`
        #[must_use]
        pub(crate) unsafe fn from_raw_mut(raw: &mut Vec<RawReport>) -> &mut Self {
            let raw_ptr = core::ptr::from_mut(raw).cast::<Self>();

            // SAFETY:
            // - The raw pointer is derived from a valid reference with the same lifetime
            //   and representation
            // - Creating this reference does not violate any aliasing rules as we are only
            //   creating a mutable reference from a different reference that is no longer
            //   being used.
            // - The type invariants of `Self` are upheld as per the caller's guarantee
            unsafe { &mut *raw_ptr }
        }

        #[must_use]
        pub(crate) fn into_raw(self) -> Vec<RawReport> {
            // SAFETY: We are destroying `self`, so we no longer
            // need to uphold any safety invariants.
            self.raw
        }

        #[must_use]
        pub(crate) fn as_raw(&self) -> &Vec<RawReport> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Upheld, as we are not allowing mutation
            // 2. Upheld, as we are not creating any such references
            // 3. Upheld, as we are not allowing mutation
            &self.raw
        }

        /// Provides mutable access to the inner raw reports vector
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `C` is a concrete type: No mutation is performed that would
        ///    invalidate the invariant that all contexts are of type `C`.
        /// 2. No mutation is performed that would invalidate the shared
        ///    ownership invariant.
        /// 3. If `T = SendSync`: No mutation is performed that invalidate the
        ///    invariant that all inner contexts and attachments are `Send +
        ///    Sync`.
        #[must_use]
        pub(crate) unsafe fn as_raw_mut(&mut self) -> &mut Vec<RawReport> {
            // SAFETY: We must uphold the safety invariants of the raw field:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            // 3. Guaranteed by the caller
            &mut self.raw
        }
    }
}
pub use limit_field_access::ReportCollection;

impl<C, T> ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    /// Creates a new, empty `ReportCollection`.
    ///
    /// The collection will be initially empty and will have no capacity
    /// allocated. This method is equivalent to calling
    /// [`Default::default()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::report_collection::ReportCollection;
    ///
    /// let collection: ReportCollection = ReportCollection::new();
    /// assert!(collection.is_empty());
    /// assert_eq!(collection.len(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let reports = Vec::new();
        // SAFETY:
        // 1. We just created the empty Vec, so the invariants are upheld for all
        //    reports in it.
        // 2. We just created the empty Vec, so the invariants are upheld for all
        //    reports in it.
        // 3. We just created the empty Vec, so the invariants are upheld for all
        //    reports in it.
        unsafe { Self::from_raw(reports) }
    }

    /// Creates a new, empty `ReportCollection` with the specified capacity.
    ///
    /// The collection will be able to hold at least `capacity` reports without
    /// reallocating. If you plan to add a known number of reports to the
    /// collection, using this method can help improve performance by reducing
    /// the number of memory allocations needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::report_collection::ReportCollection;
    ///
    /// let collection: ReportCollection = ReportCollection::with_capacity(10);
    /// assert!(collection.is_empty());
    /// assert_eq!(collection.len(), 0);
    /// assert!(collection.capacity() >= 10);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        let reports = Vec::with_capacity(capacity);
        // SAFETY: We just created the empty Vec, so there are no reports in it.
        // 1. No reports, so the invariants are upheld.
        // 2. No reports, so the invariants are upheld.
        // 3. No reports, so the invariants are upheld.
        unsafe { Self::from_raw(reports) }
    }

    /// Appends a report to the end of the collection.
    ///
    /// This method takes ownership of the report and adds it to the collection.
    /// The report must have the [`Cloneable`] ownership marker, which allows it
    /// to be stored in the collection and cloned when needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::new();
    /// let report = report!("An error occurred").into_cloneable();
    ///
    /// collection.push(report);
    /// assert_eq!(collection.len(), 1);
    /// ```
    pub fn push(&mut self, report: Report<C, Cloneable, T>) {
        // SAFETY:
        // 1. The invariants of the pushed report guarantee this.
        // 2. The argument has `O=Cloneable`, so the invariants of the pushed report
        //    guarantee this.
        // 3. If `T = SendSync`: The invariants of the pushed report guarantee this.
        let raw = unsafe { self.as_raw_mut() };

        raw.push(report.into_raw())
    }

    /// Removes and returns the last report from the collection.
    ///
    /// Returns [`None`] if the collection is empty.
    ///
    /// This method provides LIFO (last in, first out) behavior, making the
    /// collection behave like a stack for the most recently added reports.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::new();
    /// let report1 = report!("First error").into_cloneable();
    /// let report2 = report!("Second error").into_cloneable();
    ///
    /// collection.push(report1);
    /// collection.push(report2);
    ///
    /// let last_report = collection.pop().unwrap();
    /// assert_eq!(collection.len(), 1);
    ///
    /// let empty_pop = ReportCollection::<&str>::new().pop();
    /// assert!(empty_pop.is_none());
    /// ```
    pub fn pop(&mut self) -> Option<Report<C, Cloneable, T>> {
        // SAFETY:
        // 1. We only remove reports, so the invariants of the collection remain upheld.
        // 2. We only remove reports, so the invariants of the collection remain upheld.
        // 3. We only remove reports, so the invariants of the collection remain upheld.
        // 4. We only remove reports, so the invariants of the collection remain upheld.
        let raw = unsafe { self.as_raw_mut() };

        let report = raw.pop()?;

        // SAFETY:
        // 1. If `C` is a concrete type: Guaranteed by the invariants of the collection.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of the collection.
        // 4. Guaranteed by the invariants of the collection.
        // 5. If `T = SendSync`: Guaranteed by the invariants of the collection.
        let report = unsafe { Report::<C, Cloneable, T>::from_raw(report) };

        Some(report)
    }

    /// Returns the number of reports in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::new();
    /// assert_eq!(collection.len(), 0);
    ///
    /// collection.push(report!("Error 1").into_cloneable());
    /// collection.push(report!("Error 2").into_cloneable());
    /// assert_eq!(collection.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.as_raw().len()
    }

    /// Returns the capacity of the collection.
    ///
    /// The capacity is the number of reports the collection can hold without
    /// allocating additional memory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::any::Any;
    /// use rootcause::{markers::SendSync, report, report_collection::ReportCollection};
    ///
    /// let collection = ReportCollection::<dyn Any, SendSync>::with_capacity(5);
    /// assert!(collection.capacity() <= 5);
    /// ```
    pub fn capacity(&self) -> usize {
        self.as_raw().capacity()
    }

    /// Reserves capacity for at least `additional` more reports to be inserted
    /// in the collection.
    ///
    /// The collection may reserve more space to avoid frequent reallocations.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::any::Any;
    /// use rootcause::{markers::SendSync, report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::<dyn Any, SendSync>::new();
    /// collection.reserve(10);
    /// assert!(collection.capacity() >= 10);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        // SAFETY:
        // 1. We only reserve space, so the invariants of the collection remain upheld.
        // 2. We only reserve space, so the invariants of the collection remain upheld.
        // 3. We only reserve space, so the invariants of the collection remain upheld.
        // 4. We only reserve space, so the invariants of the collection remain upheld.
        let raw = unsafe { self.as_raw_mut() };

        raw.reserve(additional)
    }

    /// Returns a reference to the report at the given index.
    ///
    /// Returns [`None`] if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::new();
    /// collection.push(report!("First error").into_cloneable());
    /// collection.push(report!("Second error").into_cloneable());
    ///
    /// let first_report = collection.get(0).unwrap();
    /// let second_report = collection.get(1).unwrap();
    /// let out_of_bounds = collection.get(2);
    ///
    /// assert!(out_of_bounds.is_none());
    /// ```
    #[must_use]
    pub fn get(&self, index: usize) -> Option<ReportRef<'_, C, Cloneable, T>> {
        let raw = self.as_raw().get(index)?.as_ref();

        // SAFETY:
        // 1. If `C` is a concrete type: Guaranteed by the invariants of the collection.
        // 2. Guaranteed by the invariants of the collection.
        // 3. Guaranteed by the invariants of the collection.
        // 4. If `T = SendSync`: All contexts and attachments in the report and all
        //    sub-reports must be `Send+Sync`
        let report = unsafe { ReportRef::from_raw(raw) };

        Some(report)
    }

    /// Returns `true` if the collection contains no reports.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::new();
    /// assert!(collection.is_empty());
    ///
    /// collection.push(report!("An error").into_cloneable());
    /// assert!(!collection.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.as_raw().is_empty()
    }

    /// Returns an iterator over references to the reports in the collection.
    ///
    /// The iterator yields [`ReportRef`] items, which are lightweight
    /// references to the reports in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection = ReportCollection::new();
    /// collection.push(report!("Error 1").into_cloneable());
    /// collection.push(report!("Error 2").into_cloneable());
    ///
    /// for (i, report_ref) in collection.iter().enumerate() {
    ///     println!("Report {}: {}", i, report_ref);
    /// }
    /// ```
    pub fn iter(&self) -> ReportCollectionIter<'_, C, T> {
        let raw = self.as_raw();

        // SAFETY:
        // 1. Guaranteed by the invariants of the collection.
        // 2. Guaranteed by the invariants of the collection.
        // 3. Guaranteed by the invariants of the collection.
        unsafe { ReportCollectionIter::from_raw(raw) }
    }

    /// Converts the collection to use type-erased contexts via `dyn Any`.
    ///
    /// This performs type erasure on the context type parameter, allowing
    /// collections with different concrete context types to be stored
    /// together or passed to functions that accept `ReportCollection<dyn
    /// Any, T>`.
    ///
    /// This method does not actually modify the collection in any way. It only
    /// has the effect of "forgetting" that the context actually has the
    /// type `C`.
    ///
    /// The thread safety marker `T` is preserved during this conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::any::Any;
    ///
    /// use rootcause::{report, report_collection::ReportCollection};
    ///
    /// let mut collection: ReportCollection<dyn Any> = ReportCollection::new();
    /// collection.push(report!("String error").into_cloneable());
    ///
    /// let erased: ReportCollection<dyn Any> = collection.into_dyn_any();
    /// assert_eq!(erased.len(), 1);
    /// ```
    #[must_use]
    pub fn into_dyn_any(self) -> ReportCollection<dyn Any, T> {
        let raw = self.into_raw();

        // SAFETY:
        // 1. `C=dyn Any`, so this is trivially true.
        // 2. The invariants of the collection guarantee this.
        // 3. The invariants of the collection guarantee this.
        unsafe { ReportCollection::<dyn Any, T>::from_raw(raw) }
    }

    /// Returns a reference to the collection with type-erased contexts via
    /// `dyn Any`.
    #[must_use]
    pub fn as_dyn_any(&self) -> &ReportCollection<dyn Any, T> {
        let raw = self.as_raw();

        // SAFETY:
        // 1. `C=dyn Any`, so this is trivially true.
        // 2. The invariants of the collection guarantee this.
        // 3. The invariants of the collection guarantee this.
        unsafe { ReportCollection::<dyn Any, T>::from_raw_ref(raw) }
    }

    /// Converts the collection to use [`Local`] thread safety semantics.
    ///
    /// This changes the thread safety marker from any type to [`Local`], which
    /// means the resulting collection will not implement [`Send`] or
    /// [`Sync`]. This is useful when you want to use the collection in
    /// single-threaded contexts and potentially store non-thread-safe data.
    ///
    /// This method does not actually modify the collection in any way. It only
    /// has the effect of "forgetting" that all objects in the
    /// [`ReportCollection`] are actually [`Send`] and [`Sync`].
    ///
    /// The context type `C` is preserved during this conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{report_collection::ReportCollection, markers::Local, report};
    ///
    /// let mut collection: ReportCollection<dyn std::any::Any> = ReportCollection::new(); // defaults to SendSync
    /// collection.push(report!("An error").into_cloneable());
    ///
    /// let local_collection: ReportCollection<dyn std::any::Any, Local> = collection.into_local();
    /// assert_eq!(local_collection.len(), 1);
    /// ```
    #[must_use]
    pub fn into_local(self) -> ReportCollection<C, Local> {
        let raw = self.into_raw();

        // SAFETY:
        // 1. The invariants of the collection guarantee this.
        // 2. The invariants of the collection guarantee this.
        // 3. `T=Local`, so this is trivially true.
        unsafe { ReportCollection::<C, Local>::from_raw(raw) }
    }

    /// Returns a reference to the collection with [`Local`] thread safety
    /// semantics.
    #[must_use]
    pub fn as_local(&self) -> &ReportCollection<C, Local> {
        let raw = self.as_raw();

        // SAFETY:
        // 1. The invariants of the collection guarantee this.
        // 2. The invariants of the collection guarantee this.
        // 3. `T=Local`, so this is trivially true.
        unsafe { ReportCollection::<C, Local>::from_raw_ref(raw) }
    }

    /// Creates a new [`Report`] with the given context and sets the current
    /// report collection as the children of the new report.
    ///
    /// The new context will use the [`handlers::Display`] handler to format the
    /// context.
    ///
    /// This is a convenience method used for chaining method calls; it consumes
    /// the [`ReportCollection`] and returns a new [`Report`].
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
    /// # use rootcause::{Report, report_collection::ReportCollection, report};
    /// let report_collection: ReportCollection = [report!("error A"), report!("error B")]
    ///     .into_iter()
    ///     .collect();
    /// let report: Report<&str> = report_collection.context("additional context");
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
    /// report collection as the children of the new report.
    ///
    /// This is a convenience method used for chaining method calls; it consumes
    /// the [`ReportCollection`] and returns a [`Report`].
    ///
    /// If you want to more directly control the allocation of the new report,
    /// you can use [`Report::from_parts`], which is the underlying method
    /// used to implement this method.
    ///
    /// # Examples
    /// ```
    /// # use rootcause::{Report, report_collection::ReportCollection, report, handlers};
    /// let report_collection: ReportCollection = [report!("error A"), report!("error B")]
    ///     .into_iter()
    ///     .collect();
    /// let report: Report<&str> = report_collection.context_custom::<handlers::Debug, _>("context");
    /// ```
    #[track_caller]
    #[must_use]
    pub fn context_custom<H, D>(self, context: D) -> Report<D, Mutable, T>
    where
        D: markers::ObjectMarkerFor<T>,
        H: ContextHandler<D>,
    {
        Report::from_parts::<H>(context, self.into_dyn_any(), ReportAttachments::new())
    }
}

impl<C, T> Default for ReportCollection<C, T>
where
    C: markers::ObjectMarkerFor<T> + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<C> ReportCollection<C, SendSync>
where
    C: markers::ObjectMarkerFor<SendSync> + ?Sized,
{
    /// Creates a new, empty `ReportCollection` with [`SendSync`] thread safety.
    ///
    /// This is equivalent to calling [`new()`](Self::new) but makes the thread
    /// safety marker explicit. The resulting collection can be safely sent
    /// between threads and shared across threads.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{markers::SendSync, report_collection::ReportCollection};
    ///
    /// let collection: ReportCollection<&str, SendSync> = ReportCollection::new_sendsync();
    /// assert!(collection.is_empty());
    /// ```
    #[must_use]
    pub fn new_sendsync() -> Self {
        Self::new()
    }
}

impl<C> ReportCollection<C, Local>
where
    C: markers::ObjectMarker + ?Sized,
{
    /// Creates a new, empty `ReportCollection` with [`Local`] thread safety.
    ///
    /// This creates a collection that is not [`Send`] or [`Sync`], meaning it
    /// cannot be transferred between threads or shared across threads. This
    /// is useful for single-threaded applications or when you need to store
    /// non-thread-safe data.
    ///
    /// # Examples
    ///
    /// ```
    /// use rootcause::{markers::Local, report_collection::ReportCollection};
    ///
    /// let collection: ReportCollection<&str, Local> = ReportCollection::new_local();
    /// assert!(collection.is_empty());
    /// ```
    #[must_use]
    pub fn new_local() -> Self {
        Self::new()
    }
}

impl<C, O, T> Extend<Report<C, O, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = Report<C, O, T>>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for report in iter {
            self.push(report.into_cloneable());
        }
    }
}

impl<C, O, T> Extend<Report<C, O, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = Report<C, O, T>>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for report in iter {
            self.push(report.into_dyn_any().into_cloneable());
        }
    }
}

impl<'a, C, T> Extend<ReportRef<'a, C, Cloneable, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for report in iter {
            self.push(report.clone_arc());
        }
    }
}

impl<'a, C, T> Extend<ReportRef<'a, C, Cloneable, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    T: markers::ThreadSafetyMarker,
{
    fn extend<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for report in iter {
            self.push(report.clone_arc().into_dyn_any());
        }
    }
}

impl<C, O, T> FromIterator<Report<C, O, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = Report<C, O, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<C, O, T> FromIterator<Report<C, O, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    O: markers::ReportOwnershipMarker,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = Report<C, O, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<'a, C, T> FromIterator<ReportRef<'a, C, Cloneable, T>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<'a, C, T> FromIterator<ReportRef<'a, C, Cloneable, T>> for ReportCollection<dyn Any, T>
where
    C: markers::ObjectMarker,
    T: markers::ThreadSafetyMarker,
{
    fn from_iter<I: IntoIterator<Item = ReportRef<'a, C, Cloneable, T>>>(iter: I) -> Self {
        let mut siblings = ReportCollection::new();
        siblings.extend(iter);
        siblings
    }
}

impl<C, T> core::fmt::Display for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let raw = self.as_raw();

        // SAFETY:
        // 1. For the called method we set `C=dyn Any`, so this is trivially true.
        // 2. For the called method we set `O=Uncloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of the collection.
        // 4. For the called method we set `T=Local`, so this is trivially true.
        let slice = unsafe {
            // @add-unsafe-context: rootcause_internals::RawReport
            // @add-unsafe-context: rootcause_internals::RawReportRef
            ReportRef::<dyn Any, Uncloneable, Local>::from_raw_slice(raw)
        };

        crate::hooks::report_formatting::format_reports(slice, f, FormattingFunction::Display)
    }
}

impl<C, T> core::fmt::Debug for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let raw = self.as_raw();

        // SAFETY:
        // 1. For the called method we set `C=dyn Any`, so this is trivially true.
        // 2. For the called method we set `O=Uncloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of the collection.
        // 4. For the called method we set `T=Local`, so this is trivially true.
        let slice = unsafe {
            // @add-unsafe-context: rootcause_internals::RawReport
            // @add-unsafe-context: rootcause_internals::RawReportRef
            ReportRef::<dyn Any, Uncloneable, Local>::from_raw_slice(raw)
        };

        crate::hooks::report_formatting::format_reports(slice, f, FormattingFunction::Debug)
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
            $thread_safety1:ty => $thread_safety2:ty,
            [$($op:ident),*]
        ),* $(,)?) => {
            $(
                impl<$($param),*> From<ReportCollection<$context1, $thread_safety1>> for ReportCollection<$context2, $thread_safety2>
                    where
                        $($param: markers::ObjectMarker)*
                    {
                    fn from(report_collection: ReportCollection<$context1, $thread_safety1>) -> Self {
                        report_collection
                            $(
                                .$op()
                            )*
                    }
                }
            )*
        };
    }

    from_impls! {
        <C>: C => C, SendSync => Local, [into_local],
        <C>: C => dyn Any, SendSync => SendSync, [into_dyn_any],
        <C>: C => dyn Any, SendSync => Local, [into_dyn_any, into_local],
        <C>: C => dyn Any, Local => Local, [into_dyn_any],
        <>: dyn Any => dyn Any, SendSync => Local, [into_local],
    }
}

impl<C, T> From<Vec<Report<C, Cloneable, T>>> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(reports: Vec<Report<C, Cloneable, T>>) -> Self {
        reports.into_iter().collect()
    }
}

impl<const N: usize, C, T> From<[Report<C, Cloneable, T>; N]> for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn from(reports: [Report<C, Cloneable, T>; N]) -> Self {
        reports.into_iter().collect()
    }
}

// SAFETY: The `SendSync` marker guarantees that all reports are `Send + Sync`
// so the collection can safely implement `Send` and `Sync`.
unsafe impl<C> Send for ReportCollection<C, SendSync> where C: markers::ObjectMarker + ?Sized {}

// SAFETY: The `SendSync` marker guarantees that all reports are `Send + Sync`
// so the collection can safely implement `Send` and `Sync`.
unsafe impl<C> Sync for ReportCollection<C, SendSync> where C: markers::ObjectMarker + ?Sized {}

impl<C, T> IntoIterator for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportCollectionIntoIter<C, T>;
    type Item = Report<C, Cloneable, T>;

    fn into_iter(self) -> Self::IntoIter {
        let raw = self.into_raw();

        // SAFETY:
        // 1. Guaranteed by the invariants of the collection.
        // 2. Guaranteed by the invariants of the collection.
        // 3. Guaranteed by the invariants of the collection.
        unsafe { ReportCollectionIntoIter::<C, T>::from_raw(raw) }
    }
}

impl<'a, C, T> IntoIterator for &'a ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    type IntoIter = ReportCollectionIter<'a, C, T>;
    type Item = ReportRef<'a, C, Cloneable, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<C, T> Clone for ReportCollection<C, T>
where
    C: markers::ObjectMarker + ?Sized,
    T: markers::ThreadSafetyMarker,
{
    fn clone(&self) -> Self {
        self.iter().map(|child| child.clone_arc()).collect()
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
    fn test_report_collection_send_sync() {
        static_assertions::assert_impl_all!(ReportCollection<(), SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportCollection<String, SendSync>: Send, Sync);
        static_assertions::assert_impl_all!(ReportCollection<NonSend, SendSync>: Send, Sync); // This still makes sense, since you won't actually be able to construct this report
        static_assertions::assert_impl_all!(ReportCollection<dyn Any, SendSync>: Send, Sync);

        static_assertions::assert_not_impl_any!(ReportCollection<(), Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollection<String, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollection<NonSend, Local>: Send, Sync);
        static_assertions::assert_not_impl_any!(ReportCollection<dyn Any, Local>: Send, Sync);
    }

    #[test]
    fn test_report_collection_copy_clone() {
        static_assertions::assert_impl_all!(ReportCollection<(), SendSync>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<String, SendSync>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<NonSend, SendSync>: Clone); // This still makes sense, since you won't actually be able to construct this report
        static_assertions::assert_impl_all!(ReportCollection<dyn Any, SendSync>: Clone);

        static_assertions::assert_impl_all!(ReportCollection<(), Local>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<String, Local>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<NonSend, Local>: Clone);
        static_assertions::assert_impl_all!(ReportCollection<dyn Any, Local>: Clone);

        static_assertions::assert_not_impl_any!(ReportCollection<(), SendSync>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<String, SendSync>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<NonSend, SendSync>: Copy); // This still makes sense, since you won't actually be able to construct this report_collection collection
        static_assertions::assert_not_impl_any!(ReportCollection<dyn Any, SendSync>: Copy);

        static_assertions::assert_not_impl_any!(ReportCollection<(), Local>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<String, Local>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<NonSend, Local>: Copy);
        static_assertions::assert_not_impl_any!(ReportCollection<dyn Any, Local>: Copy);
    }
}
