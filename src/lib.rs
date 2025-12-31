#![cfg_attr(not(doc), no_std)]
#![deny(
    missing_docs,
    clippy::alloc_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    clippy::multiple_unsafe_ops_per_block,
    clippy::as_ptr_cast_mut,
    clippy::ptr_as_ptr,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::broken_intra_doc_links,
    missing_copy_implementations,
    unused_doc_comments
)]
// Extra checks on nightly
#![cfg_attr(nightly_extra_checks, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(nightly_extra_checks, forbid(rustdoc::missing_doc_code_examples))]
// Make docs.rs generate better docs
#![cfg_attr(docsrs, feature(doc_cfg))]

//! A flexible, ergonomic, and inspectable error reporting library for Rust.
//!
//! <img
//!     src="https://github.com/rootcause-rs/rootcause/raw/main/rootcause.png"
//!     width="192"
//!     title="Rootcause logo, created by lead designer Tethys Svensson"
//!     alt="A stylized image of a magnifying glass examining a root system, the logo of Rootcause">
//!
//! ## Overview
//!
//! This crate provides a structured way to represent errors. The main goal of
//! this library is to enable you to build rich error reports that
//! automatically capture not just what went wrong, but also the higher-level
//! context and supporting data at each step in the error's propagation.
//!
//! Unlike simple string-based error messages, rootcause allows you to attach
//! typed data to errors, build error chains, and inspect error contents
//! programmatically. This makes debugging easier while providing beautiful,
//! human-readable error messages, with no loss of clarity in error-recovery
//! code.
//!
//! ## Quick Example
//!
//! ```
//! use rootcause::prelude::{Report, ResultExt};
//!
//! fn read_config(path: &str) -> Result<String, Report> {
//!     std::fs::read_to_string(path).context("Failed to read configuration file")?;
//!     Ok(String::new())
//! }
//! ```
//!
//! For more examples, see the
//! [examples directory](https://github.com/rootcause-rs/rootcause/tree/main/examples)
//! in the repository. Start with
//! [`basic.rs`](https://github.com/rootcause-rs/rootcause/blob/main/examples/basic.rs)
//! for a hands-on introduction.
//!
//! ## Core Concepts
//!
//! On a mechanical level, an error report in rootcause is a node in a tree data
//! structure. Each report contains three things:
//! - A single piece of data, called the **context**.
//! - Any number of pieces of metadata, called the **attachments**.
//! - Any number of **child reports**, which are the child nodes in the tree.
//!
//! The **context** is intended to be the most concise part of the report, such
//! as an error code that can be used in error recovery code, or a
//! human-readable error that is short and descriptive. (All the better if the
//! error code has a good `Display` implementation to get the best of both.)
//!
//! Meanwhile the **attachments** are intended to be useful metadata for the
//! purpose of error analysis when debugging, logging, or instrumenting a
//! program. This can be things like operation IDs in an asynchronous REST call,
//! the username of the logged-in user for whom the error occurred, or similar.
//! These are rarely relevant in error recovery.
//!
//! Finally the **child reports** are intended to be a collection of the
//! lower-level reports of errors that led to the current error report. In most
//! cases this is just one child report: when the loading of a configuration
//! file fails, the top-level report expresses this fact, but the child report
//! might distinguish whether the file was not found, or if there was a parse
//! error. In other cases there can be a use for multiple child reports: during
//! a polling operation on a REST api that automatically retries some number of
//! times, collecting the multiple failed responses as child reports of the
//! final error report.
//!
//! For implementation details, see the [`rootcause-internals`] crate.
//!
//! [`rootcause-internals`]: rootcause_internals
//!
//! ## Ecosystem
//!
//! rootcause is designed to be lightweight and extensible. The core library
//! provides essential error handling, while optional companion crates add
//! specialized capabilities:
//!
//! - **[`rootcause-backtrace`]** - Automatic stack trace capture for debugging.
//!   Install hooks to attach backtraces to all errors, or use the extension
//!   trait to add them selectively.
//!
//! [`rootcause-backtrace`]: https://docs.rs/rootcause-backtrace
//!
//! ## Project Goals
//!
//! - **Ergonomic**: The `?` operator should work with most error types, even
//!   ones not designed for this library.
//! - **Multi-failure tracking**: When operations fail multiple times (retry
//!   attempts, batch processing, parallel execution), all failures should be
//!   captured and preserved in a single report.
//! - **Inspectable**: The objects in a Report should not be glorified strings.
//!   Inspecting and interacting with them should be easy.
//! - **Optionally typed**: Users should be able to (optionally) specify the
//!   type of the context in the root node.
//! - **Beautiful**: The default formatting should look pleasant—and if it
//!   doesn't match your style, the [hook system] lets you customize it.
//! - **Cloneable**: It should be possible to clone a [`Report`] when you need
//!   to.
//! - **Self-documenting**: Reports should automatically capture information
//!   (like locations) that might be useful in debugging. Additional
//!   instrumentation like backtraces can be added via extension crates.
//! - **Customizable**: It should be possible to customize what data gets
//!   collected, or how reports are formatted.
//! - **Lightweight**: [`Report`] has a pointer-sized representation, keeping
//!   `Result<T, Report>` small and fast.
//!
//! [hook system]: crate::hooks
//!
//! ## Report Type Parameters
//!
//! The [`Report`] type is generic over three parameters, but for most users the
//! defaults work fine.
//!
//! **Most common usage:**
//!
//! ```
//! # use rootcause::prelude::*;
//! // Just use Report - works like anyhow::Error
//! fn might_fail() -> Result<(), Report> {
//!     # Ok(())
//! }
//! ```
//!
//! **For type safety:**
//!
//! ```
//! # use rootcause::prelude::*;
//! #[derive(Debug)]
//! struct MyError;
//! # impl std::fmt::Display for MyError {
//! #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//! #         write!(f, "MyError")
//! #     }
//! # }
//! # impl std::error::Error for MyError {}
//!
//! // Use Report<YourError> - works like error-stack
//! fn typed_error() -> Result<(), Report<MyError>> {
//!     # Ok(())
//! }
//! ```
//!
//! **Need cloning or thread-local data?** The sections below explain the other
//! type parameters. Come back to these when you need them - they solve specific
//! problems you'll recognize when you encounter them.
//!
//! ------------------
//!
//! ## Type Parameters
//!
//! *This section covers the full type parameter system. Most users won't need
//! these variants immediately - but if you do need cloning, thread-local
//! errors, or want to understand what's possible, read on.*
//!
//! The [`Report`] type has three type parameters:
//! `Report<Context, Ownership, ThreadSafety>`.
//! This section explains all the options and when you'd use them.
//!
//! ### Context Type: Typed vs Dynamic Errors
//!
//! **Use `Report<Dynamic>`** (or just [`Report`]) when errors just need to
//! propagate. **Use `Report<YourErrorType>`** when callers need to pattern
//! match on specific error variants.
//!
//! **`Report<Dynamic>`** (or just [`Report`]) — Flexible, like [`anyhow`]
//!
//! Can hold any error type at the root. The `?` operator automatically converts
//! any error into a [`Report`]. Note: [`Dynamic`] is just a marker signaling
//! that the actual type is unknown. No actual instance of [`Dynamic`] is
//! stored. Converting between typed and dynamic reports is zero-cost.
//!
//! [`Dynamic`]: crate::markers::Dynamic
//!
//! ```
//! # use rootcause::prelude::*;
//! // Can return any error type
//! fn might_fail() -> Result<(), Report> {
//!     # Ok(())
//! }
//! ```
//!
//! **`Report<YourErrorType>`** — Type-safe, like [`error-stack`]
//!
//! The root error must be `YourErrorType`, but child errors can be anything.
//! Callers can use `.current_context()` to pattern match on the typed error.
//!
//! ```
//! # use rootcause::prelude::*;
//! #[derive(Debug)]
//! struct ConfigError {/* ... */}
//! # impl std::fmt::Display for ConfigError {
//! #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
//! # }
//! # impl std::error::Error for ConfigError {}
//!
//! // This function MUST return ConfigError at the root
//! fn load_config() -> Result<(), Report<ConfigError>> {
//!     # Ok(())
//! }
//! ```
//!
//! See [`examples/typed_reports.rs`] for a complete example with retry logic.
//!
//! [`examples/typed_reports.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/typed_reports.rs
//!
//! ### Ownership: Mutable vs Cloneable
//!
//! **Use the default ([`Mutable`])** when errors just propagate with `?`.
//!
//! **Use [`.into_cloneable()`]** when you need to store errors in collections
//! or use them multiple times.
//!
//! [`.into_cloneable()`]: crate::report::owned::Report::into_cloneable
//!
//! **[`Mutable`]** (default) — Unique ownership
//!
//! You can add attachments and context to the root, but can't clone the whole
//! [`Report`]. Note: child reports are still cloneable internally (they use
//! `Arc`), but the top-level [`Report`] doesn't implement `Clone`. Start here,
//! then convert to [`Cloneable`] if you need to clone the entire tree.
//!
//! ```
//! # use rootcause::prelude::*;
//! let mut report: Report<String, markers::Mutable> = report!("error".to_string());
//! let report = report.attach("debug info"); // ✅ Can mutate root
//! // let cloned = report.clone();           // ❌ Can't clone whole report
//! ```
//!
//! **[`Cloneable`]** — Shared ownership
//!
//! The [`Report`] can be cloned cheaply (via `Arc`), but can't be mutated. Use
//! when you need to pass the same error to multiple places.
//!
//! ```
//! # use rootcause::prelude::*;
//! let report: Report<String, markers::Mutable> = report!("error".to_string());
//! let cloneable = report.into_cloneable();
//! let copy1 = cloneable.clone(); // ✅ Can clone
//! let copy2 = cloneable.clone(); // ✅ Cheap (Arc clone)
//! // let modified = copy1.attach("info"); // ❌ Can't mutate
//! ```
//!
//! See [`examples/retry_with_collection.rs`] for collection usage.
//!
//! [`examples/retry_with_collection.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/retry_with_collection.rs
//!
//! ### Thread Safety: SendSync vs Local
//!
//! **Use the default ([`SendSync`])** unless you get compiler errors about
//! `Send` or `Sync`. **Use [`Local`]** only when attaching `!Send` types like
//! `Rc` or `Cell`.
//!
//! **[`SendSync`]** (default) — Thread-safe
//!
//! The [`Report`] and all its contents are `Send + Sync`. Most types (String,
//! Vec, primitives) are already `Send + Sync`, so this just works.
//!
//! ```
//! # use rootcause::prelude::*;
//! let report: Report<String, markers::Mutable, markers::SendSync> = report!("error".to_string());
//!
//! # let thread_join_handle =
//! std::thread::spawn(move || {
//!     println!("{}", report); // ✅ Can send to other threads
//! });
//! # thread_join_handle.join();
//! ```
//!
//! **[`Local`]** — Not thread-safe
//!
//! Use when your error contains thread-local data like `Rc`, raw pointers, or
//! other `!Send` types.
//!
//! ```
//! # use rootcause::prelude::*;
//! use std::rc::Rc;
//!
//! let data = Rc::new("thread-local".to_string());
//! let report: Report<Rc<String>, markers::Mutable, markers::Local> = report!(data);
//! // std::thread::spawn(move || { ... }); // ❌ Can't send to other threads
//! ```
//!
//! ## Converting Between Report Variants
//!
//! The variant lists above have been ordered so that it is always possible to
//! convert to an element further down the list using the [`From`] trait. This
//! also means you can use `?` when converting downwards. There are also more
//! specific methods (implemented using [`From`]) to help with type inference
//! and to more clearly communicate intent:
//!
//! - [`Report::into_dynamic`] converts from `Report<C, *, *>` to
//!   `Report<Dynamic, *, *>`. See [`examples/error_coercion.rs`] for usage
//!   patterns.
//! - [`Report::into_cloneable`] converts from `Report<*, Mutable, *>` to
//!   `Report<*, Cloneable, *>`. See [`examples/retry_with_collection.rs`] for
//!   storing multiple errors.
//! - [`Report::into_local`] converts from `Report<*, *, SendSync>` to
//!   `Report<*, *, Local>`.
//!
//! On the other hand, it is generally harder to convert to an element further
//! up the list. Here are some of the ways to do it:
//!
//! - From `Report<Dynamic, *, *>` to `Report<SomeContextType, *, *>`:
//!   - You can check if the type of the root node matches a specific type by
//!     using [`Report::downcast_report`]. This will return either the requested
//!     report type or the original report depending on whether the types match.
//!     See [`examples/inspecting_errors.rs`] for downcasting techniques.
//! - From `Report<*, Cloneable, *>` to `Report<*, Mutable, *>`:
//!   - You can check if the root node only has a single owner using
//!     [`Report::try_into_mutable`]. This will check the number of references
//!     to the root node and return either the requested report variant or the
//!     original report depending on whether it is unique.
//!   - You can allocate a new root node and set the current node as a child of
//!     the new node. The new root node will be [`Mutable`]. One method for
//!     allocating a new root node is to call [`Report::context`].
//! - From `Report<*, *, *>` to `Report<PreformattedContext, Mutable,
//!   SendSync>`:
//!   - You can preformat the entire [`Report`] using [`Report::preformat`].
//!     This creates an entirely new [`Report`] that has the same structure and
//!     will look the same as the current one if printed, but all contexts and
//!     attachments will be replaced with a [`PreformattedContext`] version.
//!
//! [`examples/error_coercion.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/error_coercion.rs
//! [`examples/inspecting_errors.rs`]: https://github.com/rootcause-rs/rootcause/blob/main/examples/inspecting_errors.rs
//!
//! # Acknowledgements
//!
//! This library was inspired by and draws ideas from several existing error
//! handling libraries in the Rust ecosystem, including [`anyhow`],
//! [`thiserror`], and [`error-stack`].
//!
//! [`PreformattedContext`]: crate::preformatted::PreformattedContext
//! [`Mutable`]: crate::markers::Mutable
//! [`Cloneable`]: crate::markers::Cloneable
//! [`SendSync`]: crate::markers::SendSync
//! [`Local`]: crate::markers::Local
//! [`anyhow`]: https://docs.rs/anyhow
//! [`anyhow::Error`]: https://docs.rs/anyhow/latest/anyhow/struct.Error.html
//! [`thiserror`]: https://docs.rs/thiserror
//! [`error-stack`]: https://docs.rs/error-stack
//! [`error-stack::Report`]: https://docs.rs/error-stack/latest/error_stack/struct.Report.html

extern crate alloc;

#[macro_use]
mod macros;

pub mod handlers;
pub mod hooks;
pub mod markers;
pub mod preformatted;

pub mod compat;
pub mod option_ext;
pub mod prelude;
mod report;
pub mod report_attachment;
pub mod report_attachments;
pub mod report_collection;

mod into_report;
mod iterator_ext;
mod report_conversion;
mod result_ext;
mod util;

pub use self::{
    into_report::{IntoReport, IntoReportCollection},
    report::{iter::ReportIter, mut_::ReportMut, owned::Report, ref_::ReportRef},
    report_conversion::ReportConversion,
};

/// A [`Result`](core::result::Result) type alias where the error is [`Report`].
///
/// This is a convenient shorthand for functions that return errors as
/// [`Report`]. The context type defaults to [`Dynamic`].
///
/// # Examples
///
/// ```
/// use rootcause::prelude::*;
///
/// fn might_fail() -> rootcause::Result<String> {
///     Ok("success".to_string())
/// }
/// ```
///
/// With a typed error:
///
/// ```
/// use rootcause::prelude::*;
///
/// #[derive(Debug)]
/// struct MyError;
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         write!(f, "MyError")
/// #     }
/// # }
/// # impl std::error::Error for MyError {}
///
/// fn typed_error() -> rootcause::Result<String, MyError> {
///     Err(report!(MyError))
/// }
/// ```
///
/// [`Dynamic`]: crate::markers::Dynamic
pub type Result<T, C = markers::Dynamic> = core::result::Result<T, Report<C>>;

// Not public API. Referenced by macro-generated code and rootcause-backtrace.
#[doc(hidden)]
pub mod __private {
    // Used by the rootcause-backtrace
    pub const ROOTCAUSE_LOCATION: &core::panic::Location = core::panic::Location::caller();

    use alloc::fmt;
    #[doc(hidden)]
    pub use alloc::format;
    #[doc(hidden)]
    pub use core::{format_args, result::Result::Err};

    use crate::{
        Report, handlers,
        markers::{self, Dynamic},
        report_attachment::ReportAttachment,
    };

    #[doc(hidden)]
    #[inline]
    #[cold]
    #[must_use]
    #[track_caller]
    pub fn format_report(
        args: fmt::Arguments<'_>,
    ) -> Report<Dynamic, markers::Mutable, markers::SendSync> {
        if let Some(message) = args.as_str() {
            Report::new_sendsync_custom::<handlers::Display>(message).into_dynamic()
        } else {
            Report::new_sendsync_custom::<handlers::Display>(fmt::format(args)).into_dynamic()
        }
    }

    #[doc(hidden)]
    #[inline]
    #[cold]
    #[must_use]
    #[track_caller]
    pub fn format_report_attachment(
        args: fmt::Arguments<'_>,
    ) -> ReportAttachment<Dynamic, markers::SendSync> {
        if let Some(message) = args.as_str() {
            ReportAttachment::new_sendsync_custom::<handlers::Display>(message).into_dynamic()
        } else {
            ReportAttachment::new_sendsync_custom::<handlers::Display>(fmt::format(args))
                .into_dynamic()
        }
    }

    #[doc(hidden)]
    pub mod kind {
        use crate::{
            Report, handlers, markers, report_attachment::ReportAttachment,
            report_attachments::ReportAttachments, report_collection::ReportCollection,
        };

        #[doc(hidden)]
        pub struct Wrap<'a, T>(pub &'a T);

        #[doc(hidden)]
        pub trait SendSyncKind {
            #[inline(always)]
            fn thread_safety(&self) -> markers::SendSync {
                markers::SendSync
            }
        }

        impl<C> SendSyncKind for C where C: markers::ObjectMarkerFor<markers::SendSync> {}

        #[doc(hidden)]
        pub trait LocalKind {
            #[inline(always)]
            fn thread_safety(&self) -> markers::Local {
                markers::Local
            }
        }

        impl<C> LocalKind for &C where C: markers::ObjectMarkerFor<markers::Local> {}

        #[doc(hidden)]
        pub trait HandlerErrorKind {
            #[inline(always)]
            fn handler(&self) -> handlers::Error {
                handlers::Error
            }
        }

        impl<C> HandlerErrorKind for &&&Wrap<'_, C> where handlers::Error: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        pub trait HandlerDisplayKind {
            #[inline(always)]
            fn handler(&self) -> handlers::Display {
                handlers::Display
            }
        }

        impl<C> HandlerDisplayKind for &&Wrap<'_, C> where handlers::Display: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        pub trait HandlerDebugKind {
            #[inline(always)]
            fn handler(&self) -> handlers::Debug {
                handlers::Debug
            }
        }

        impl<C> HandlerDebugKind for &Wrap<'_, C> where handlers::Debug: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        pub trait HandlerAnyKind {
            #[inline(always)]
            fn handler(&self) -> handlers::Any {
                handlers::Any
            }
        }

        impl<C> HandlerAnyKind for Wrap<'_, C> where handlers::Any: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        #[must_use]
        #[track_caller]
        pub fn macro_helper_new_report<H, T, C>(
            _handler: H,
            _thread_safety: T,
            context: C,
        ) -> Report<C, markers::Mutable, T>
        where
            H: handlers::ContextHandler<C>,
            C: markers::ObjectMarkerFor<T>,
        {
            Report::from_parts::<H>(context, ReportCollection::new(), ReportAttachments::new())
        }

        #[doc(hidden)]
        #[must_use]
        #[track_caller]
        pub fn macro_helper_new_report_attachment<H, T, A>(
            _handler: H,
            _thread_safety: T,
            attachment: A,
        ) -> ReportAttachment<A, T>
        where
            H: handlers::AttachmentHandler<A>,
            A: markers::ObjectMarkerFor<T>,
        {
            ReportAttachment::new_custom::<H>(attachment)
        }
    }
}
