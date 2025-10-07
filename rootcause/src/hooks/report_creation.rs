use alloc::vec::Vec;
use core::{
    any::{Any, TypeId},
    fmt,
    panic::Location,
};

use rootcause_internals::handlers::AttachmentHandler;
use triomphe::Arc;
use unsize::CoerceUnsize;

#[cfg(feature = "backtrace")]
use crate::hooks::attachment_collectors::backtrace::{BacktraceCollector, BacktraceHandler};
use crate::{
    handlers,
    hooks::{
        attachment_collectors::location::{LocationCollector, LocationHandler},
        hook_lock::{HookLock, HookLockReadGuard},
    },
    markers::{self, Local, SendSync},
    report::ReportMut,
    report_attachment::ReportAttachment,
};

type HookSet = Vec<Arc<dyn UntypedReportCreationHook>>;

static HOOKS: HookLock<HookSet> = HookLock::new();

trait UntypedReportCreationHook: 'static + Send + Sync + core::fmt::Display {
    #[track_caller]
    fn on_local_creation(&self, report: ReportMut<'_, dyn Any, Local>);
    #[track_caller]
    fn on_sendsync_creation(&self, report: ReportMut<'_, dyn Any, SendSync>);
}

pub trait ReportCreationHook: 'static + Send + Sync {
    // TODO: Create a `ReportMut` type to avoid a double indirection.
    #[track_caller]
    fn on_local_creation(&self, report: ReportMut<'_, dyn Any, Local>);
    #[track_caller]
    fn on_sendsync_creation(&self, report: ReportMut<'_, dyn Any, SendSync>);
}

#[track_caller]
fn creation_hook_to_untyped<H>(hook: H) -> Arc<dyn UntypedReportCreationHook>
where
    H: ReportCreationHook + Send + Sync + 'static,
{
    struct Hook<H> {
        hook: H,
        added_at: &'static Location<'static>,
    }

    impl<H> core::fmt::Display for Hook<H> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Report creation hook {} registered at {}:{}",
                core::any::type_name::<H>(),
                self.added_at.file(),
                self.added_at.line()
            )
        }
    }

    impl<H> UntypedReportCreationHook for Hook<H>
    where
        H: ReportCreationHook,
    {
        fn on_local_creation(&self, report: ReportMut<'_, dyn Any, Local>) {
            self.hook.on_local_creation(report);
        }

        fn on_sendsync_creation(&self, report: ReportMut<'_, dyn Any, SendSync>) {
            self.hook.on_sendsync_creation(report);
        }
    }

    let hook: Hook<H> = Hook {
        hook,
        added_at: Location::caller(),
    };
    Arc::new(hook).unsize(unsize::Coercion!(to dyn UntypedReportCreationHook))
}

#[track_caller]
fn attachment_hook_to_untyped<A, H, C>(collector: C) -> Arc<dyn UntypedReportCreationHook>
where
    A: 'static + Send + Sync,
    H: AttachmentHandler<A>,
    C: AttachmentCollectorHook<A> + Send + Sync + 'static,
{
    struct Hook<A, H, C> {
        collector: C,
        added_at: &'static Location<'static>,
        _handled_type: core::marker::PhantomData<A>,
        _handler: core::marker::PhantomData<H>,
    }
    unsafe impl<A, H, C> Send for Hook<A, H, C> where C: 'static + Send + Sync {}
    unsafe impl<A, H, C> Sync for Hook<A, H, C> where C: 'static + Send + Sync {}

    impl<A, H, C> UntypedReportCreationHook for Hook<A, H, C>
    where
        A: 'static + Send + Sync,
        H: AttachmentHandler<A>,
        C: AttachmentCollectorHook<A> + Send + Sync,
    {
        #[track_caller]
        fn on_local_creation(&self, mut report: ReportMut<'_, dyn Any, Local>) {
            let attachment = self.collector.collect();
            report
                .attachments_mut()
                .push(ReportAttachment::new_full_local::<H>(attachment).into_dyn_any());
        }

        #[track_caller]
        fn on_sendsync_creation(&self, mut report: ReportMut<'_, dyn Any, SendSync>) {
            let attachment = self.collector.collect();
            report
                .attachments_mut()
                .push(ReportAttachment::new_sendsync_with_handler::<H>(attachment).into_dyn_any());
        }
    }
    impl<A, H, C> core::fmt::Display for Hook<A, H, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Attachment collector hook {} for attachment type {} with handler {} registered at {}:{}",
                core::any::type_name::<C>(),
                core::any::type_name::<A>(),
                core::any::type_name::<H>(),
                self.added_at.file(),
                self.added_at.line()
            )
        }
    }

    let hook = Hook {
        collector,
        added_at: core::panic::Location::caller(),
        _handled_type: core::marker::PhantomData,
        _handler: core::marker::PhantomData,
    };
    let hook: Arc<Hook<A, H, C>> = Arc::new(hook);

    hook.unsize(unsize::Coercion!(to dyn UntypedReportCreationHook))
}

#[track_caller]
fn default_hooks() -> HookSet {
    let mut hooks = Vec::new();

    hooks.push(attachment_hook_to_untyped::<_, LocationHandler, _>(
        LocationCollector,
    ));

    #[cfg(feature = "backtrace")]
    hooks.push(attachment_hook_to_untyped::<_, BacktraceHandler, _>(
        BacktraceCollector::default(),
    ));

    hooks
}

#[track_caller]
pub fn register_report_creation_hook<H>(hook: H)
where
    H: ReportCreationHook + Send + Sync + 'static,
{
    HOOKS
        .write()
        .get()
        .get_or_insert_with(default_hooks)
        .push(creation_hook_to_untyped(hook));
}

pub trait AttachmentCollectorHook<A>: 'static + Send + Sync {
    type Handler: AttachmentHandler<A>;
    #[track_caller]
    fn collect(&self) -> A;
}

impl<A, F> AttachmentCollectorHook<A> for F
where
    A: 'static + core::fmt::Display + core::fmt::Debug,
    F: 'static + Send + Sync + Fn() -> A,
{
    type Handler = handlers::Display;
    #[track_caller]
    fn collect(&self) -> A {
        (self)()
    }
}

#[track_caller]
pub fn register_attachment_collector_hook<A, C>(collector: C)
where
    A: 'static + Send + Sync,
    C: AttachmentCollectorHook<A> + Send + Sync + 'static,
{
    HOOKS
        .write()
        .get()
        .get_or_insert_with(default_hooks)
        .push(attachment_hook_to_untyped::<A, C::Handler, C>(collector));
}

#[track_caller]
fn get_hooks() -> HookLockReadGuard<HookSet> {
    let read_guard = HOOKS.read();
    if read_guard.get().is_some() {
        read_guard
    } else {
        core::mem::drop(read_guard);
        HOOKS.write().get().get_or_insert_with(default_hooks);
        HOOKS.read()
    }
}

#[track_caller]
#[inline(never)]
fn run_creation_hooks_local(mut report: ReportMut<'_, dyn Any, Local>) {
    if let Some(hooks) = get_hooks().get() {
        for hook in hooks {
            hook.on_local_creation(report.reborrow());
        }
    }
}

#[track_caller]
#[inline(never)]
fn run_creation_hooks_sendsync(mut report: ReportMut<'_, dyn Any, SendSync>) {
    if let Some(hooks) = get_hooks().get() {
        for hook in hooks {
            hook.on_sendsync_creation(report.reborrow());
        }
    }
}

#[track_caller]
#[inline(always)]
pub(crate) fn __run_creation_hooks<T>(report: ReportMut<'_, dyn Any, T>)
where
    T: markers::ThreadSafetyMarker,
{
    if TypeId::of::<T>() == TypeId::of::<Local>() {
        let report: ReportMut<'_, dyn Any, Local> =
            unsafe { ReportMut::from_raw(report.into_raw()) };
        run_creation_hooks_local(report);
    } else if TypeId::of::<T>() == TypeId::of::<SendSync>() {
        let report: ReportMut<'_, dyn Any, SendSync> =
            unsafe { ReportMut::from_raw(report.into_raw()) };
        run_creation_hooks_sendsync(report);
    } else {
        unreachable!(
            "Unsupported thread safety marker for report creation: {:?}",
            TypeId::of::<T>()
        );
    }
}
