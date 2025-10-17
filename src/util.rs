use core::fmt;

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
