//! Soundness test for mutable context downcasting.
//!
//! Type-erased mutable downcasting must produce its `&mut T` from a pointer that
//! carries provenance over the whole allocation, never from a shared reference.
//! Deriving a `&mut` from a `&` (even a transient one anywhere in the chain) is
//! undefined behavior, because the optimizer may assume a shared reference never
//! changes, making a write through the resulting `&mut` invalid.
//!
//! rootcause's mutable path (`RawReportMut`) carries a `NonNull` that retains
//! full provenance over the `Arc` allocation (from `triomphe::Arc::into_raw`)
//! and builds the `&mut` from that pointer directly. Each test below takes a
//! `&mut` through a public mutable-downcast entry point and writes through it,
//! so Miri can verify the write violates no aliasing rule. Run under both
//! aliasing models:
//!
//! ```text
//! cargo miri test --test downcast_mut_soundness
//! MIRIFLAGS=-Zmiri-tree-borrows cargo miri test --test downcast_mut_soundness
//! ```
//!
//! The same class of bug was found in several other error libraries; see
//! zkat/miette#469, dtolnay/anyhow#451, and eyre-rs/eyre#285 for background.

use core::fmt;

use rootcause::{Report, markers::Dynamic, prelude::*};

/// A minimal context whose `message` can be mutated in place, giving the
/// mutable downcasts something to write to.
#[derive(Debug)]
struct Ctx {
    message: String,
}

impl Ctx {
    fn new(message: &str) -> Self {
        Ctx {
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for Ctx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// `Report::current_context_mut` -> `&mut C`.
#[test]
fn current_context_mut_allows_write() {
    let mut report: Report<Ctx> = report!(Ctx::new("start"));

    report.current_context_mut().message.push_str("-edited");

    assert_eq!(report.current_context().message, "start-edited");
}

/// `Report::current_context_as_any_mut` -> `&mut dyn Any` -> `&mut C`.
#[test]
fn current_context_as_any_mut_allows_write() {
    let mut report: Report<Ctx> = report!(Ctx::new("start"));

    let any = report.current_context_as_any_mut();
    any.downcast_mut::<Ctx>()
        .expect("context is Ctx")
        .message
        .push_str("-edited");

    assert_eq!(report.current_context().message, "start-edited");
}

/// `ReportMut::downcast_current_context_mut` on a type-erased report -> `&mut C`.
#[test]
fn downcast_current_context_mut_allows_write() {
    let mut report: Report<Dynamic> = report!(Ctx::new("start")).into_dynamic();

    report
        .as_mut()
        .downcast_current_context_mut::<Ctx>()
        .expect("current context is Ctx")
        .message
        .push_str("-edited");

    assert_eq!(
        report
            .as_ref()
            .downcast_current_context::<Ctx>()
            .unwrap()
            .message,
        "start-edited"
    );
}

/// The same mutable downcast, but on a report that owns a child so the backing
/// allocation is non-trivial (real vtable, populated children vector).
#[test]
fn downcast_mut_on_report_with_child() {
    let child: Report<Ctx> = report!(Ctx::new("child"));
    let mut parent: Report<Ctx> = child.context(Ctx::new("parent"));
    assert_eq!(parent.children().len(), 1);

    parent.current_context_mut().message.push_str("-edited");

    assert_eq!(parent.current_context().message, "parent-edited");
}
