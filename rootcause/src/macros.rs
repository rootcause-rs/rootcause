#[macro_export]
macro_rules! report {
    ($msg:literal $(,)?) => {
        $crate::__private::format_report($crate::__private::format_args!($msg))
    };
    ($context:expr $(,)?) => {
        $crate::__private::must_use({
            use $crate::__private::kind::*;
            let context = $context;
            let handler = (&&&&Wrap(&context)).handler();
            let thread_safety = (&context).thread_safety();
            let report: $crate::Report<
                _,
                $crate::markers::Mutable,
                _
            > = new_with_handler_and_thread_marker(handler, thread_safety, context);
            report
        })
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::report::Report::<
            _,
            $crate::markers::Mutable,
            $crate::markers::SendSync
        >::new_with_handler::<$crate::handlers::Display>(
            $crate::__private::format!($fmt, $($arg)*)
        ).into_dyn_any()
    };
}

#[macro_export]
macro_rules! bail {
    ($($args:tt)*) => {{
        let report = $crate::report!($($args)*);
        return $crate::__private::Err(report.into());
    }};
}
