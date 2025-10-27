#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::missing_safety_doc)]
#![forbid(
//     missing_docs,
    clippy::alloc_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
//     clippy::missing_safety_doc,
//     clippy::missing_docs_in_private_items,
//     clippy::undocumented_unsafe_blocks,
//     clippy::multiple_unsafe_ops_per_block,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::broken_intra_doc_links
)]
//! A flexible, ergonomic, and inspectable error reporting library for Rust.
//!
//! <img src="https://github.com/rootcause-rs/rootcause/raw/main/rootcause.png" width="192">
//!
//! ## Overview
//!
//! This crate provides a structured way to represent and work with errors and their context.
//! The main goal is to enable you to build rich, structured error reports that automatically
//! capture not just what went wrong, but also the context and supporting data at each step.
//!
//! ## Project goals
//!
//! - **Ergonomic**: The `?` operator should work with most error types, even ones not designed
//!   for this library.
//! - **Fast happy path**: A `Result<(), Report>` should never be larger than a `usize`.
//! - **Typable**: Users should be able to (optionally) specify the type of the context in the
//!   root node.
//! - **Inspectable**: The objects in a Report should not be glorified strings. Inspecting and
//!   interacting with them should be easy.
//! - **Cloneable**: It should be possible to clone a `Report` when you need to.
//! - **Mergeable**: It should be possible to merge multiple `Report`s into a single one.
//! - **Customizable**: It should be possible to customize what data gets collected, or how
//!   reports are formatted.
//! - **Rich**: Reports should automatically capture information (like backtraces) that might
//!   be useful in debugging.
//! - **Beautiful**: The default formatting of a Report should look pleasant (at least to the authors).
//!
//! ## Core Concepts
//!
//! At a high level, you can think of the library as a way to build a tree of error reports, where each
//! node in the tree represents a step in the error's history.
//!
//! You can think of a Report as roughly implementing this data structure:
//!
//! ```rust
//! # use std::any::Any;
//! struct Report<C: ?Sized> {
//!     root: triomphe::Arc<Node<C>>,
//! }
//!
//! struct Node<C: ?Sized> {
//!     attachments: Vec<Attachment>,
//!     children: Vec<Report<dyn Any>>,
//!     context: C,
//! }
//!
//! struct Attachment(Box<dyn Any>);
//! ```
//!
//! In practice, the actual implementation differs from this somewhat. There are multiple reasons for this,
//! including performance, ergonomics and being able to support the features we want.
//!
//! However this simpler implementation does illustrate the core concepts and the ownership model well.
//!
//! If you want the full details, take a look at the [`rootcause-internals`] crate.
//!
//! [`rootcause-internals`]: rootcause_internals
//!
//! ## Generics and Report Variants
//!
//! To support the different, incompatible use-cases, the `Report` type is generic over
//! three parameters:
//!
//! 1. The type of the context in the root node. The default is `dyn Any`, which means that
//!    the root node can have any type of context.
//! 2. The ownership and cloning behavior of the report. The default is [`Mutable`], which means
//!    that the root node of the report is mutable, but cannot be cloned.
//! 3. The thread-safety of the objects inside the report. The default is [`SendSync`], which means
//!    that the report implements `Send+Sync`, but also requires all objects inside it to
//!    be `Send+Sync`.
//!
//! If you want an experience similar to [`anyhow`], you can ignore all the type parameters.
//! You should simply use `Report`, which is the same as `Report<dyn Any, Mutable, SendSync>`.
//!
//! If you want an experience similar to [`error-stack`], you should use `Report<SomeContextType>`,
//! which is the same as `Report<SomeContextType, Mutable, SendSync>`.
//!
//! #### Tables of Report Variants
//! ##### Context Variants
//! | Variant                       | Context of root node | Context of internal nodes |
//! |------------------- -----------|----------------------|---------------------------|
//! | `Report<SomeContextType,*,*>` | `SomeContextType`    | Can be anything           |
//! | `Report<dyn Any,*,*>`         | Can be anything      | Can be anything           |
//!
//! Note that `dyn Any` though only be thought of as a marker. No actual trait object is stored anywhere. Converting
//! from a `Report<SomeContextType>` to `Report<dyn Any>` is a zero-cost operation.
//!
//! ##### Ownership and Cloning Variants
//! | Variant                | `Clone` | Mutation supported | Intuition                                                                 |
//! |------------------------|---------|--------------------|---------------------------------------------------------------------------|
//! | `Report<*,Mutable,*>`  | ❌      | 🟡 - Root only     | Root node is allocated using [`ArcUnique`], internal nodes using [`Arc`]. |
//! | `Report<*,Clonable,*>` | ✅      | ❌                 | All nodes are allocated using [`Arc`].                                    |
//!
//! [`ArcUnique`]: https://docs.rs/triomphe/latest/triomphe/struct.UniqueArc.html
//! [`Arc`]: https://docs.rs/triomphe/latest/triomphe/struct.Arc.html
//!
//! The `Mutable` exist to allow mutating the root node (e.g. adding attachments).
//!
//! ##### Thread-Safety Variants
//! | Variant                | `Send+Sync` | Permits insertion of `!Send` and `!Sync` objects | Intuition                                                                                |
//! |------------------------|-------------|--------------------------------------------------|------------------------------------------------------------------------------------------|
//! | `Report<*,*,SendSync>` | ✅          | ❌                                               | The objects inside the report are `Send+Sync`, so the report itself is also `Send+Sync`. |
//! | `Report<*,*,Local>`    | ❌          | ✅                                               | Since objects inside the report might not be `Send+Sync`, the report is `!Send+!Sync`.   |
//!
//! ## Converting Between Report Variants
//!
//! The lists have been ordered so that it is always possible to convert to an element further
//! down the list using the [`From`] trait. This also means you can use `?` when converting
//! downwards. There are also more specific methods (implemented using [`From`]), to help with
//! type inference and to more clearly communicate intent:
//! - [`Report::into_dyn_any`] converts from `Report<C, *, *>` to `Report<dyn Any, *, *>`.
//! - [`Report::into_cloneable`] converts from `Report<*, Mutable, *>` to `Report<*, Cloneable, *>`.
//! - [`Report::into_local`] converts from `Report<*, *, SendSync>` to `Report<*, *, Local>`.
//!
//! On the other hand, it is generally harder to convert to an element further up the list. Here
//! are some of the ways to do it:
//!
//! - From `Report<dyn Any, *, *>` to `Report<SomeContextType, *, *>`:
//!   - You can check if the type of the root node matches a specific type by using
//!     [`Report::downcast_report`]. This will return either the requested report type or the
//!     original report depending on whether the types match.
//! - From `Report<*, Clonable, *>` to `Report<*, Mutable, *>`:
//!   - You can check if the root node only has a single owner using
//!     [`Report::try_into_mutable`]. This will check the number of references to the root
//!     node and return either the requested report variant or the original report depending
//!     on whether it is unique.
//!   - You can allocate a new root node and set the current node as a child of the
//!     new node. The new root node will be [`Mutable`]. One method for allocating a
//!     new root node is to call [`Report::context`].
//! - From `Report<*, *, *>` to `Report<PreformattedContext, Mutable, SendSync>`:
//!   - You can preformat the entire the report using [`Report::preformat`]. This creates
//!     an entirely new report that has the same structure and will look the same as the
//!     current one if printed, but all contexts and attachments will be replaced with a
//!     [`PreformattedContext`] version.
//!
//! # Acknowledgements
//!
//! This library was inspired by and draws ideas from several existing error handling
//! libraries in the Rust ecosystem, including [`anyhow`], [`thiserror`], and
//! [`error-stack`].
//!
//! [`PreformattedContext`]: crate::preformatted::PreformattedContext
//! [`Mutable`]: crate::markers::Mutable
//! [`SendSync`]: crate::markers::SendSync
//! [`anyhow`]: https://docs.rs/anyhow
//! [`thiserror`]: https://docs.rs/thiserror
//! [`error-stack`]: https://docs.rs/error-stack

extern crate alloc;

#[macro_use]
mod macros;

pub mod handlers;
pub mod hooks;
pub mod markers;
pub mod preformatted;

mod report;
pub mod report_attachment;
pub mod report_attachments;
pub mod report_collection;

mod into_report;
pub mod iterator_ext;
pub mod prelude;
mod result_ext;
mod util;

pub use self::{
    into_report::{IntoReport, IntoReportCollection},
    report::{iter::ReportIter, mut_::ReportMut, owned::Report, ref_::ReportRef},
};

// Not public API. Referenced by macro-generated code.
#[doc(hidden)]
pub mod __private {
    use alloc::fmt;
    #[doc(hidden)]
    pub use alloc::format;
    #[doc(hidden)]
    pub use core::{any::Any, format_args, result::Result::Err};

    use crate::{Report, handlers, markers};

    #[doc(hidden)]
    #[inline]
    #[cold]
    #[must_use]
    #[track_caller]
    pub fn format_report(
        args: fmt::Arguments<'_>,
    ) -> Report<dyn Any, markers::Mutable, markers::SendSync> {
        if let Some(message) = args.as_str() {
            Report::new_sendsync_custom::<handlers::Display>(message).into_dyn_any()
        } else {
            Report::new_sendsync_custom::<handlers::Display>(fmt::format(args)).into_dyn_any()
        }
    }

    #[doc(hidden)]
    pub mod kind {
        use crate::{
            Report, handlers, markers, report_attachments::ReportAttachments,
            report_collection::ReportCollection,
        };

        #[doc(hidden)]
        pub struct Wrap<'a, T>(pub &'a T);

        #[doc(hidden)]
        pub trait SendSyncKind {
            fn thread_safety(&self) -> markers::SendSync {
                markers::SendSync
            }
        }

        impl<C> SendSyncKind for C where C: markers::ObjectMarkerFor<markers::SendSync> {}

        #[doc(hidden)]
        pub trait LocalKind {
            fn thread_safety(&self) -> markers::Local {
                markers::Local
            }
        }

        impl<C> LocalKind for &C where C: markers::ObjectMarkerFor<markers::Local> {}

        #[doc(hidden)]
        pub trait HandlerErrorKind {
            fn handler(&self) -> handlers::Error {
                handlers::Error
            }
        }

        impl<C> HandlerErrorKind for &&&Wrap<'_, C> where handlers::Error: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        pub trait HandlerDisplayKind {
            fn handler(&self) -> handlers::Display {
                handlers::Display
            }
        }

        impl<C> HandlerDisplayKind for &&Wrap<'_, C> where handlers::Display: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        pub trait HandlerDebugKind {
            fn handler(&self) -> handlers::Debug {
                handlers::Debug
            }
        }

        impl<C> HandlerDebugKind for &Wrap<'_, C> where handlers::Debug: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        pub trait HandlerAnyKind {
            fn handler(&self) -> handlers::Any {
                handlers::Any
            }
        }

        impl<C> HandlerAnyKind for Wrap<'_, C> where handlers::Any: handlers::ContextHandler<C> {}

        #[doc(hidden)]
        #[must_use]
        #[track_caller]
        pub fn macro_helper_new<H, T, C>(
            _handler: H,
            _thread_safety: T,
            context: C,
        ) -> Report<C, markers::Mutable, T>
        where
            H: handlers::ContextHandler<C>,
            T: markers::ThreadSafetyMarker,
            C: markers::ObjectMarkerFor<T>,
        {
            Report::from_parts::<H>(context, ReportCollection::new(), ReportAttachments::new())
        }
    }
}
