//! Internal utility functions.

use core::fmt;

/// Creates a type that implements both `Display` and `Debug` by delegating
/// to provided functions.
///
/// Used internally to return formatted representations from methods.
pub(crate) fn format_helper<State, DisplayFn, DebugFn>(
    state: State,
    display_fn: DisplayFn,
    debug_fn: DebugFn,
) -> impl fmt::Display + fmt::Debug
where
    State: Copy,
    for<'a, 'b> DisplayFn: Copy + Fn(State, &'a mut fmt::Formatter<'b>) -> fmt::Result,
    for<'a, 'b> DebugFn: Copy + Fn(State, &'a mut fmt::Formatter<'b>) -> fmt::Result,
{
    FormatHelper {
        state,
        display_fn,
        debug_fn,
    }
}

/// Helper type created by [`format_helper`].
struct FormatHelper<State, DisplayFn, DebugFn> {
    state: State,
    display_fn: DisplayFn,
    debug_fn: DebugFn,
}

impl<State, DisplayFn, DebugFn> core::fmt::Display for FormatHelper<State, DisplayFn, DebugFn>
where
    State: Copy,
    for<'a, 'b> DisplayFn: Fn(State, &'a mut core::fmt::Formatter<'b>) -> core::fmt::Result,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        (self.display_fn)(self.state, f)
    }
}

impl<State, DisplayFn, DebugFn> core::fmt::Debug for FormatHelper<State, DisplayFn, DebugFn>
where
    State: Copy,
    for<'a, 'b> DebugFn: Fn(State, &'a mut core::fmt::Formatter<'b>) -> core::fmt::Result,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        (self.debug_fn)(self.state, f)
    }
}

/// Wrapper type that implements `Error` without a source, delegating `Display`
/// and `Debug` to the inner type.
#[repr(transparent)]
pub(crate) struct ErrorNoSourceWrapper<T>(T);

impl<T> ErrorNoSourceWrapper<T> {
    pub(crate) fn new(inner: &T) -> &Self {
        // SAFETY:
        //
        // This is safe because `ErrorNoSourceWrapper<T>` is `repr(transparent)` and has
        // the same layout as `T`. The `ErrorNoSourceWrapper` has no safety invariants
        // itself, and it does not allow mutating the inner value, so whatever safety
        // invariants `T` has are preserved.
        let ptr = core::ptr::from_ref(inner).cast::<ErrorNoSourceWrapper<T>>();
        unsafe { &*ptr }
    }
}

impl<T> core::fmt::Display for ErrorNoSourceWrapper<T>
where
    T: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}

impl<T> core::fmt::Debug for ErrorNoSourceWrapper<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T> core::error::Error for ErrorNoSourceWrapper<T> where T: core::fmt::Display + core::fmt::Debug
{}
