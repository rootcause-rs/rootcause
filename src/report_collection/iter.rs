use core::iter::FusedIterator;

use crate::{Report, ReportRef, markers::Cloneable};

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access {
    use core::marker::PhantomData;

    use rootcause_internals::RawReport;

    use crate::markers::{Dynamic, SendSync};

    /// An iterator over references to reports in a [`ReportCollection`].
    ///
    /// This iterator yields [`ReportRef`] instances, allowing you to iterate
    /// over the reports in a collection without taking ownership.
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
    /// // Iterate over references to reports
    /// for report_ref in collection.iter() {
    ///     println!("Report: {}", report_ref);
    /// }
    /// ```
    ///
    /// [`ReportRef`]: crate::ReportRef
    /// [`ReportCollection`]: crate::report_collection::ReportCollection
    #[must_use]
    pub struct ReportCollectionIter<
        'a,
        Context: ?Sized + 'static = Dynamic,
        ThreadSafety: 'static = SendSync,
    > {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. Either the collection must be empty, `C` must either be a type
        ///    bounded by `Sized`, or C must be `Dynamic`.
        /// 2. Either the collection must be empty or `T` must either be
        ///    `SendSync` or `Local`.
        /// 3. If `C` is a `Sized` type: The contexts of the [`RawReport`]s are
        ///    all of type `C`.
        /// 4. All references to these report or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 5. If `T = SendSync`: All contexts and attachments in the
        ///    [`RawReport`]s and all sub-reports must be `Send+Sync`.
        raw: core::slice::Iter<'a, RawReport>,
        _context: PhantomData<Context>,
        _thread_safety: PhantomData<ThreadSafety>,
    }

    impl<'a, C: ?Sized, T> ReportCollectionIter<'a, C, T> {
        /// Creates a new `ReportCollectionIter` from an iterator of raw reports
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. Either the collection must be empty, `C` must either be a type
        ///    bounded by `Sized`, or C must be `Dynamic`.
        /// 2. Either the collection must be empty or `T` must either be
        ///    `SendSync` or `Local`.
        /// 3. If `C` is a `Sized` type: The contexts of the [`RawReport`]s are
        ///    all of type `C`.
        /// 4. All references to these report or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 5. If `T = SendSync`: All contexts and attachments in the
        ///    [`RawReport`]s and all sub-reports must be `Send+Sync`.
        pub(crate) unsafe fn from_raw(raw: &'a [RawReport]) -> Self {
            // SAFETY: We must uphold the safety invariants of this type:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            // 3. Guaranteed by the caller
            // 4. Guaranteed by the caller
            // 5. Guaranteed by the caller
            Self {
                raw: raw.iter(),
                _context: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Returns a reference to the underlying raw report iterator
        pub(crate) fn as_raw(&self) -> &core::slice::Iter<'a, RawReport> {
            // SAFETY: We must uphold the safety invariants of this type:
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. No mutation occurs here, so the invariants are preserved
            // 4. Upheld, as all references created here are compatible
            // 5. No mutation occurs here, so the invariants are preserved
            &self.raw
        }

        /// Returns a mutable reference to the underlying raw report iterator
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `C` is a `Sized` type: No mutation is performed that would
        ///    invalidate the invariant that all contexts are of type `C`.
        /// 2. No mutation is performed that would invalidate the shared
        ///    ownership invariant.
        /// 3. If `T = SendSync`: No mutation is performed that invalidates the
        ///    invariant that all inner contexts and attachments are `Send +
        ///    Sync`.
        pub(crate) unsafe fn as_raw_mut(&mut self) -> &mut core::slice::Iter<'a, RawReport> {
            // SAFETY: We must uphold the safety invariants of this type:
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. Guaranteed by the caller
            // 4. Upheld, as all references created here are compatible
            // 5. Guaranteed by the caller
            &mut self.raw
        }
    }
}
pub use limit_field_access::ReportCollectionIter;

impl<'a, C: ?Sized, T> Iterator for ReportCollectionIter<'a, C, T> {
    type Item = ReportRef<'a, C, Cloneable, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: We only remove items, we don't mutate them.
        // 1. No mutation that breaks the context-type invariant
        // 2. No mutation that breaks the shared-ownership invariant
        // 3. No mutation that breaks the Send+Sync invariant for T=SendSync
        let raw = unsafe { self.as_raw_mut() };

        let item = raw.next()?.as_ref();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of this type.
        // 4. Guaranteed by the invariants of this type.
        // 5. Guaranteed by the invariants of this type.
        // 6. Guaranteed by the invariants of this type.
        // 7. Guaranteed by the invariants of this type.
        let raw = unsafe { ReportRef::<C, Cloneable, T>::from_raw(item) };

        Some(raw)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.as_raw().size_hint()
    }
}

impl<'a, C: ?Sized, T> DoubleEndedIterator for ReportCollectionIter<'a, C, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY: We only remove items, we don't mutate them.
        // 1. No mutation that breaks the context-type invariant
        // 2. No mutation that breaks the shared-ownership invariant
        // 3. No mutation that breaks the Send+Sync invariant for T=SendSync
        let raw = unsafe { self.as_raw_mut() };

        let item = raw.next_back()?.as_ref();

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of this type.
        // 4. Guaranteed by the invariants of this type.
        // 5. Guaranteed by the invariants of this type.
        // 6. Guaranteed by the invariants of this type.
        // 7. Guaranteed by the invariants of this type.
        let raw = unsafe { ReportRef::<C, Cloneable, T>::from_raw(item) };

        Some(raw)
    }
}

impl<'a, C: ?Sized, T> ExactSizeIterator for ReportCollectionIter<'a, C, T> {
    fn len(&self) -> usize {
        self.as_raw().len()
    }
}

impl<'a, C: ?Sized, T> FusedIterator for ReportCollectionIter<'a, C, T> {}

impl<'a, C: ?Sized, T> Unpin for ReportCollectionIter<'a, C, T> {}

/// FIXME: Once rust-lang/rust#132922 gets resolved, we can make the `raw` field
/// an unsafe field and remove this module.
mod limit_field_access2 {
    use alloc::vec::Vec;
    use core::marker::PhantomData;

    use rootcause_internals::RawReport;

    use crate::markers::{Dynamic, SendSync};

    /// An owning iterator over reports in a [`ReportCollection`].
    ///
    /// This iterator consumes a [`ReportCollection`] and yields owned
    /// [`Report`] instances. Unlike [`ReportCollectionIter`], this iterator
    /// takes ownership of the reports, allowing you to move them out of the
    /// collection.
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
    /// // Consume the collection and take ownership of reports
    /// for report in collection {
    ///     println!("Owned report: {}", report);
    /// }
    /// ```
    ///
    /// [`Report`]: crate::Report
    /// [`ReportCollectionIter`]: crate::report_collection::ReportCollectionIter
    /// [`ReportCollection`]: crate::report_collection::ReportCollection
    #[must_use]
    pub struct ReportCollectionIntoIter<
        Context: ?Sized + 'static = Dynamic,
        ThreadSafety: 'static = SendSync,
    > {
        /// # Safety
        ///
        /// The following safety invariants are guaranteed to be upheld as long
        /// as this struct exists:
        ///
        /// 1. Either the collection must be empty, `C` must either be a type
        ///    bounded by `Sized`, or C must be `Dynamic`.
        /// 2. Either the collection must be empty or `T` must either be
        ///    `SendSync` or `Local`.
        /// 3. If `C` is a `Sized` type: The contexts of the [`RawReport`]s are
        ///    all of type `C`.
        /// 4. All references to these report or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 5. If `T = SendSync`: All contexts and attachments in the
        ///    [`RawReport`]s and all sub-reports must be `Send+Sync`.
        raw: alloc::vec::IntoIter<RawReport>,
        _context: PhantomData<fn(Context) -> Context>,
        _thread_safety: PhantomData<ThreadSafety>,
    }

    impl<C: ?Sized, T> ReportCollectionIntoIter<C, T> {
        /// Creates a new [`ReportCollectionIntoIter`] from a vector of raw
        /// reports
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. Either the collection must be empty, `C` must either be a type
        ///    bounded by `Sized`, or C must be `Dynamic`.
        /// 2. Either the collection must be empty or `T` must either be
        ///    `SendSync` or `Local`.
        /// 3. If `C` is a `Sized` type: The contexts of the [`RawReport`]s are
        ///    all of type `C`.
        /// 4. All references to these report or any sub-reports are compatible
        ///    with shared ownership. Specifically there are no references with
        ///    an assumption that the strong_count is `1`.
        /// 5. If `T = SendSync`: All contexts and attachments in the
        ///    [`RawReport`]s and all sub-reports must be `Send+Sync`.
        pub(crate) unsafe fn from_raw(raw: Vec<RawReport>) -> Self {
            // SAFETY: We must uphold the safety invariants of this type:
            // 1. Guaranteed by the caller
            // 2. Guaranteed by the caller
            // 3. Guaranteed by the caller
            // 4. Guaranteed by the caller
            // 5. Guaranteed by the caller
            Self {
                raw: raw.into_iter(),
                _context: PhantomData,
                _thread_safety: PhantomData,
            }
        }

        /// Returns a reference to the underlying raw report iterator
        pub(crate) fn as_raw(&self) -> &alloc::vec::IntoIter<RawReport> {
            // SAFETY: We must uphold the safety invariants of this type:
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. No mutation occurs here, so the invariants are preserved
            // 4. No mutation occurs here, so the invariants are preserved
            // 5. Upheld, as it is not possible to turn this into a `Report`, `ReportMut` or
            //    `ReportRef` with `T=SendSync`, as that would break the safety invariants
            //    of those types.
            &self.raw
        }

        /// Returns a mutable reference to the underlying raw report iterator
        ///
        /// # Safety
        ///
        /// The caller must ensure:
        ///
        /// 1. If `C` is a `Sized` type: No mutation is performed that would
        ///    invalidate the invariant that all contexts are of type `C`.
        /// 2. No mutation is performed that would invalidate the shared
        ///    ownership invariant.
        /// 3. If `T = SendSync`: No mutation is performed that invalidates the
        ///    invariant that all inner contexts and attachments are `Send +
        ///    Sync`.
        #[must_use]
        pub(crate) unsafe fn as_raw_mut(&mut self) -> &mut alloc::vec::IntoIter<RawReport> {
            // SAFETY: We must uphold the safety invariants of this type:
            // 1. Upheld as the type parameters do not change.
            // 2. Upheld as the type parameters do not change.
            // 3. Guaranteed by the caller
            // 4. Guaranteed by the caller
            // 5. Guaranteed by the caller
            &mut self.raw
        }
    }
}
pub use limit_field_access2::ReportCollectionIntoIter;

impl<C: ?Sized, T> Iterator for ReportCollectionIntoIter<C, T> {
    type Item = Report<C, Cloneable, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: We only remove items, we don't mutate them.
        // 1. No mutation that breaks the context-type invariant
        // 2. No mutation that breaks the shared-ownership invariant
        // 3. No mutation that breaks the Send+Sync invariant for T=SendSync
        let raw = unsafe { self.as_raw_mut() };

        let item = raw.next()?;

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of this type.
        // 4. Guaranteed by the invariants of this type.
        // 5. `O=Cloneable`, so this is trivially true.
        // 6. Guaranteed by the invariants of this type.
        // 7. Guaranteed by the invariants of this type.
        // 8. Guaranteed by the invariants of this type.
        let raw = unsafe { Report::<C, Cloneable, T>::from_raw(item) };

        Some(raw)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.as_raw().size_hint()
    }
}

impl<C: ?Sized, T> DoubleEndedIterator for ReportCollectionIntoIter<C, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY: We only remove items, we don't mutate them.
        // 1. No mutation that breaks the context-type invariant
        // 2. No mutation that breaks the shared-ownership invariant
        // 3. No mutation that breaks the Send+Sync invariant for T=SendSync
        let raw = unsafe { self.as_raw_mut() };

        let item = raw.next_back()?;

        // SAFETY:
        // 1. Guaranteed by the invariants of this type.
        // 2. `O=Cloneable`, so this is trivially true.
        // 3. Guaranteed by the invariants of this type.
        // 4. Guaranteed by the invariants of this type.
        // 5. `O=Cloneable`, so this is trivially true.
        // 6. Guaranteed by the invariants of this type.
        // 7. Guaranteed by the invariants of this type.
        // 8. Guaranteed by the invariants of this type.
        let raw = unsafe { Report::<C, Cloneable, T>::from_raw(item) };

        Some(raw)
    }
}

impl<C: ?Sized, T> ExactSizeIterator for ReportCollectionIntoIter<C, T> {
    fn len(&self) -> usize {
        self.as_raw().len()
    }
}

impl<C: ?Sized, T> FusedIterator for ReportCollectionIntoIter<C, T> {}

impl<C: ?Sized, T> Unpin for ReportCollectionIntoIter<C, T> {}
