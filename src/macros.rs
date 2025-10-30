/// Creates a new error report.
///
/// This macro provides a convenient way to create [`Report`](crate::Report) instances with automatic
/// type inference for thread-safety markers and error handlers.
///
/// # Two Usage Modes
///
/// ## Format String Mode
///
/// When the first argument is a string literal, the macro works like [`format!()`],
/// creating a report with a formatted string as context:
///
/// ```rust
/// use rootcause::prelude::*;
///
/// let report: Report = report!("File not found");
/// let report: Report = report!("Failed to open {}", "config.toml");
/// ```
///
/// The resulting report has type `Report<dyn Any, Mutable, SendSync>`. The context
/// is typically a `String`, but when there are no format arguments, it may be
/// optimized to a `&'static str`.
///
/// ## Context Object Mode
///
/// When given any other expression, the macro creates a report from that value:
///
/// ```rust
/// use rootcause::prelude::*;
/// # use std::io;
///
/// # fn get_io_error() -> io::Error {
/// #     io::Error::new(io::ErrorKind::NotFound, "file not found")
/// # }
/// let error: io::Error = get_io_error();
/// let report: Report<io::Error> = report!(error);
/// ```
///
/// This mode automatically:
/// - Infers the thread-safety marker based on whether the context type is `Send + Sync`
/// - Selects the appropriate handler based on the context type
///
/// This is similar to calling [`Report::new`], but with better type inference.
///
/// # Examples
///
/// ## Basic String Reports
///
/// ```
/// use std::{
///     any::{Any, TypeId},
///     rc::Rc,
/// };
///
/// use rootcause::prelude::*;
///
/// // Static string (no formatting)
/// let report: Report<dyn Any, markers::Mutable, markers::SendSync> = report!("Something broke");
/// assert_eq!(
///     report.current_context_type_id(),
///     TypeId::of::<&'static str>()
/// );
///
/// // Formatted string
/// let report: Report<dyn Any, markers::Mutable, markers::SendSync> =
///     report!("Something broke hard: {}", "it was bad");
/// assert_eq!(report.current_context_type_id(), TypeId::of::<String>());
/// assert_eq!(
///     report.current_context_handler_type_id(),
///     TypeId::of::<handlers::Display>()
/// );
/// ```
///
/// ## Error Type Reports
///
/// ```
/// use std::{
///     any::TypeId,
///     io,
/// };
///
/// use rootcause::prelude::*;
///
/// # fn something_that_fails() -> Result<(), std::io::Error> {
/// #    std::fs::read("/nonexistant")?; Ok(())
/// # }
/// let io_error: std::io::Error = something_that_fails().unwrap_err();
/// let report: Report<std::io::Error, markers::Mutable, markers::SendSync> = report!(io_error);
/// assert_eq!(
///     report.current_context_handler_type_id(),
///     TypeId::of::<handlers::Error>()
/// );
/// ```
///
/// ## Local (Non-Send) Reports
///
/// When using non-thread-safe types like [`Rc`](std::rc::Rc), the macro
/// automatically infers the [`Local`](crate::markers::Local) marker:
///
/// ```
/// use std::{
///     any::TypeId,
///     rc::Rc,
/// };
///
/// use rootcause::prelude::*;
///
/// # fn something_else_that_fails() -> Result<(), Rc<std::io::Error>> {
/// #    std::fs::read("/nonexistant")?; Ok(())
/// # }
/// let local_io_error: Rc<std::io::Error> = something_else_that_fails().unwrap_err();
/// let report: Report<Rc<std::io::Error>, markers::Mutable, markers::Local> =
///     report!(local_io_error);
/// assert_eq!(
///     report.current_context_handler_type_id(),
///     TypeId::of::<handlers::Display>()
/// );
/// ```
///
/// [`format!()`]: std::format
/// [`Report::new`]: crate::Report::new
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
            macro_helper_new_report(handler, thread_safety, context)
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

/// Creates a report attachment with contextual data.
///
/// This macro creates a [`ReportAttachment`] that can be added to error reports
/// to provide additional context. It accepts the same arguments as the [`report!`]
/// macro but produces an attachment instead of a full report.
///
/// Attachments are useful for adding supplementary information to errors without
/// changing the main error context. For example, you might attach configuration
/// values, request parameters, or debugging information.
///
/// # Usage Modes
///
/// Like [`report!`], this macro supports both format string mode and context
/// object mode. See the [`report!`] documentation for details on each mode.
///
/// # Examples
///
/// ## String Attachments
///
/// ```
/// use std::any::{Any, TypeId};
/// use rootcause::{prelude::*, report_attachment, report_attachment::ReportAttachment};
///
/// // Static string
/// let attachment: ReportAttachment<dyn Any, markers::SendSync> =
///     report_attachment!("Additional context");
/// assert_eq!(attachment.inner_type_id(), TypeId::of::<&'static str>());
/// assert_eq!(
///     attachment.inner_handler_type_id(),
///     TypeId::of::<handlers::Display>()
/// );
///
/// // Formatted string
/// let attachment: ReportAttachment<dyn Any, markers::SendSync> =
///     report_attachment!("Error occurred at line: {}", 42);
/// assert_eq!(attachment.inner_type_id(), TypeId::of::<String>());
/// assert_eq!(
///     attachment.inner_handler_type_id(),
///     TypeId::of::<handlers::Display>()
/// );
/// ```
///
/// ## Structured Data Attachments
///
/// ```
/// use std::any::TypeId;
/// use rootcause::{prelude::*, report_attachment, report_attachment::ReportAttachment};
///
/// #[derive(Debug)]
/// struct ErrorData {
///     code: i32,
///     message: String,
/// }
///
/// impl std::fmt::Display for ErrorData {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "Error {}: {}", self.code, self.message)
///     }
/// }
///
/// impl std::error::Error for ErrorData {}
///
/// let error_data = ErrorData {
///     code: 404,
///     message: "Not found".to_string(),
/// };
/// let attachment: ReportAttachment<ErrorData, markers::SendSync> =
///     report_attachment!(error_data);
/// assert_eq!(
///     attachment.inner_handler_type_id(),
///     TypeId::of::<handlers::Display>()
/// );
/// ```
///
/// ## Local (Non-Send) Attachments
///
/// ```
/// use std::rc::Rc;
/// use rootcause::{prelude::*, report_attachment, report_attachment::ReportAttachment};
///
/// let local_data: Rc<String> = Rc::new("Local context".to_string());
/// let attachment: ReportAttachment<Rc<String>, markers::Local> =
///     report_attachment!(local_data);
/// ```
///
/// [`ReportAttachment`]: crate::report_attachment::ReportAttachment
/// [`Report`]: crate::Report
#[macro_export]
macro_rules! report_attachment {
    ($msg:literal $(,)?) => {
        $crate::__private::format_report_attachment($crate::__private::format_args!($msg))
    };
    ($context:expr $(,)?) => {
        {
            use $crate::__private::kind::*;
            let context = $context;
            let handler = (&&&Wrap(&context)).handler();
            let thread_safety = (&context).thread_safety();
            macro_helper_new_report_attachment(handler, thread_safety, context)
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::report_attachment::ReportAttachment::<
            _,
            $crate::markers::SendSync
        >::new_custom::<$crate::handlers::Display>(
            $crate::__private::format!($fmt, $($arg)*)
        ).into_dyn_any()
    };
}

/// Returns early from a function with an error report.
///
/// This macro creates a new [`Report`] and immediately returns it wrapped in an `Err`.
/// It's a convenience shorthand for `return Err(report!(...).into())`.
///
/// The macro is similar to the [`bail!`] macro from the [`anyhow`] crate and accepts
/// the same arguments as the [`report!`] macro.
///
/// # When to Use
///
/// Use `bail!` when you want to:
/// - Return an error immediately without additional processing
/// - Keep error-handling code concise and readable
/// - Avoid writing explicit `return Err(...)` statements
///
/// # Examples
///
/// ## Basic Validation
///
/// ```
/// use rootcause::prelude::*;
///
/// fn validate_positive(value: i32) -> Result<(), Report> {
///     if value < 0 {
///         bail!("Value must be non-negative, got {}", value);
///     }
///     Ok(())
/// }
///
/// assert!(validate_positive(-5).is_err());
/// assert!(validate_positive(10).is_ok());
/// ```
///
/// ## Multiple Validation Checks
///
/// ```
/// use rootcause::prelude::*;
///
/// fn validate_age(age: i32) -> Result<(), Report> {
///     if age < 0 {
///         bail!("Age cannot be negative: {}", age);
///     }
///     if age > 150 {
///         bail!("Age seems unrealistic: {}", age);
///     }
///     Ok(())
/// }
/// ```
///
/// [`bail!`]: https://docs.rs/anyhow/latest/anyhow/macro.bail.html
/// [`anyhow`]: https://docs.rs/anyhow/latest/anyhow/
/// [`Report`]: crate::Report
#[macro_export]
macro_rules! bail {
    ($($args:tt)*) => {
        return $crate::__private::Err($crate::report!($($args)*).into())
    };
}
