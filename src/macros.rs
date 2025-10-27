/// Macro to generate a report
///
/// This macro can be invoked in two different ways, using a format string or using
/// a context object.
///
/// ## Using a format string
///
/// When invoked with a literal as the first argument, this macro will interpret
/// and evaluate the arguments in the same way as the [`format!()`] macro.
///
/// The resulting string will become the context of the new report. The resulting
/// report will have the type `Report<dyn Any, Mutable, SendSync>`.
///
/// The inner context will typically be a `String`, but in cases where
/// the format does not contain arguments, it is typically be optimized to
/// a `&'static str` instead.
///
/// [`format!()`]: std::format
///
/// ## Using a context object
///
/// This macro also accepts any other expression. When used like this, it is
/// mostly equivalent to calling [`Report::new`], however it has some benefits:
///
/// - It automatically infers the correct thread marker based on the context object.
/// - It automatically infers the correct handler based on the context object.
///
/// [`Report::new`]: crate::Report::new
///
/// # Examples
///
/// ```
/// use std::{
///     any::{Any, TypeId},
///     rc::Rc,
/// };
///
/// use rootcause::prelude::*;
///
/// let report: Report<dyn Any, markers::Mutable, markers::SendSync> = report!("Something broke");
/// assert_eq!(
///     report.current_context_type_id(),
///     TypeId::of::<&'static str>()
/// );
///
/// let report: Report<dyn Any, markers::Mutable, markers::SendSync> =
///     report!("Something broke hard: {}", "it was bad");
/// assert_eq!(report.current_context_type_id(), TypeId::of::<String>());
///
/// # fn something_that_fails() -> Result<(), std::io::Error> {
/// #    std::fs::read("/nonexistant")?; Ok(())
/// # }
/// let io_error: std::io::Error = something_that_fails().unwrap_err();
/// let report: Report<std::io::Error, markers::Mutable, markers::SendSync> = report!(io_error);
///
/// # fn something_else_that_fails() -> Result<(), Rc<std::io::Error>> {
/// #    std::fs::read("/nonexistant")?; Ok(())
/// # }
/// let local_io_error: Rc<std::io::Error> = something_else_that_fails().unwrap_err();
/// let report: Report<Rc<std::io::Error>, markers::Mutable, markers::Local> =
///     report!(local_io_error);
/// ```
#[macro_export]
macro_rules! report {
    ($msg:literal $(,)?) => {
        $crate::__private::format_report($crate::__private::format_args!($msg))
    };
    ($context:expr $(,)?) => {
        {
            use $crate::__private::kind::*;
            let context = $context;
            let handler = (&&&&Wrap(&context)).handler();
            let thread_safety = (&context).thread_safety();
            macro_helper_new(handler, thread_safety, context)
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Report::<
            _,
            $crate::markers::Mutable,
            $crate::markers::SendSync
        >::new_custom::<$crate::handlers::Display>(
            $crate::__private::format!($fmt, $($arg)*)
        ).into_dyn_any()
    };
}

/// Return early with an error.
///
/// This macro is similar to the [`bail!`] macro from the [`anyhow`] crate.
/// It constructs a new report using the same arguments as the [`report!`] macro,
/// and then returns early from the function with that report wrapped in an `Err`.
///
/// This is equivalent to writing `return Err(report!(...).into());`
///
/// [`bail!`]: https://docs.rs/anyhow/latest/anyhow/macro.bail.html
/// [`anyhow`]: https://docs.rs/anyhow/latest/anyhow/
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
///
/// fn do_something(
///     value: i32,
/// ) -> Result<(), Report<dyn std::any::Any, markers::Mutable, markers::SendSync>> {
///     if value < 0 {
///         bail!("Value must be non-negative, got {}", value);
///     }
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! bail {
    ($($args:tt)*) => {
        return $crate::__private::Err($crate::report!($($args)*).into())
    };
}
