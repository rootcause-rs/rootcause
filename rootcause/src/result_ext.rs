use rootcause_internals::handlers;

use crate::{
    IntoReport,
    into_report::IntoReportCollection,
    markers::{Local, Mutable, ObjectMarker, SendSync},
    report::Report,
};

mod sealed {
    pub trait Sealed {}
    impl<A, E> Sealed for Result<A, E> {}
}

pub trait ResultExt<V, E>: sealed::Sealed {
    #[track_caller]
    #[must_use]
    fn into_report(self) -> Result<V, Report<E::Context, E::Ownership, SendSync>>
    where
        E: IntoReport<SendSync>;

    #[track_caller]
    #[must_use]
    fn context<C>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        C: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn context_lazy<C, F>(self, context: F) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        F: FnOnce() -> C,
        C: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn context_with_handler<C, H>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        C: ObjectMarker + Send + Sync,
        H: handlers::ContextHandler<C>;

    #[track_caller]
    #[must_use]
    fn context_with_handler_lazy<C, F, H>(
        self,
        context: F,
    ) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        F: FnOnce() -> C,
        C: ObjectMarker + Send + Sync,
        H: handlers::ContextHandler<C>;

    #[track_caller]
    #[must_use]
    fn attach<A>(self, attachment: A) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        A: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn attach_with_handler<A, H>(
        self,
        attachment: A,
    ) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        A: ObjectMarker + Send + Sync,
        H: handlers::AttachmentHandler<A>;

    #[track_caller]
    #[must_use]
    fn attach_lazy_with_handler<A, F, H>(
        self,
        attachment: F,
    ) -> Result<V, Report<E::Context, E::Ownership, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug,
        H: handlers::AttachmentHandler<A>;

    #[track_caller]
    #[must_use]
    fn local_into_report(self) -> Result<V, Report<E::Context, E::Ownership, Local>>
    where
        E: IntoReport<Local>;

    #[track_caller]
    #[must_use]
    fn local_context<C>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        C: ObjectMarker + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn local_context_lazy<C, F>(self, context: F) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        F: FnOnce() -> C,
        C: ObjectMarker + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn local_context_with_handler<C, H>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        C: ObjectMarker,
        H: handlers::ContextHandler<C>;

    #[track_caller]
    #[must_use]
    fn local_context_with_handler_lazy<C, F, H>(
        self,
        context: F,
    ) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        F: FnOnce() -> C,
        C: ObjectMarker,
        H: handlers::ContextHandler<C>;

    #[track_caller]
    #[must_use]
    fn local_attach<A>(self, attachment: A) -> Result<V, Report<E::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        A: ObjectMarker + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn local_attach_lazy<A, F>(
        self,
        attachment: F,
    ) -> Result<V, Report<E::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + core::fmt::Display + core::fmt::Debug;

    #[track_caller]
    #[must_use]
    fn local_attach_with_handler<A, H>(
        self,
        attachment: A,
    ) -> Result<V, Report<E::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        A: ObjectMarker,
        H: handlers::AttachmentHandler<A>;

    #[track_caller]
    #[must_use]
    fn local_attach_lazy_with_handler<A, F, H>(
        self,
        attachment: F,
    ) -> Result<V, Report<E::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + core::fmt::Display + core::fmt::Debug,
        H: handlers::AttachmentHandler<A>;
}

impl<V, E> ResultExt<V, E> for Result<V, E> {
    #[inline(always)]
    fn into_report(self) -> Result<V, Report<E::Context, E::Ownership, SendSync>>
    where
        E: IntoReport<SendSync>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report()),
        }
    }

    #[inline(always)]
    fn context<C>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        C: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report_collection().context(context)),
        }
    }

    #[inline(always)]
    fn context_lazy<C, F>(self, context: F) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        F: FnOnce() -> C,
        C: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report_collection().context(context())),
        }
    }

    #[inline(always)]
    fn context_with_handler<C, H>(self, context: C) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        C: ObjectMarker + Send + Sync,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e
                .into_report_collection()
                .context_with_handler::<H, _>(context)),
        }
    }

    #[inline(always)]
    fn context_with_handler_lazy<C, F, H>(
        self,
        context: F,
    ) -> Result<V, Report<C, Mutable, SendSync>>
    where
        E: IntoReportCollection<SendSync>,
        F: FnOnce() -> C,
        C: ObjectMarker + Send + Sync,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e
                .into_report_collection()
                .context_with_handler::<H, _>(context())),
        }
    }

    #[inline(always)]
    fn attach<A>(self, attachment: A) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        A: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach(attachment)),
        }
    }

    #[inline(always)]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach(attachment())),
        }
    }

    #[inline(always)]
    fn attach_with_handler<A, H>(
        self,
        attachment: A,
    ) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        A: ObjectMarker + Send + Sync,
        H: handlers::AttachmentHandler<A>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach_with_handler::<H, _>(attachment)),
        }
    }

    #[inline(always)]
    fn attach_lazy_with_handler<A, F, H>(
        self,
        attachment: F,
    ) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + Send + Sync + core::fmt::Display + core::fmt::Debug,
        H: handlers::AttachmentHandler<A>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach_with_handler::<H, _>(attachment())),
        }
    }

    #[inline(always)]
    fn local_into_report(self) -> Result<V, Report<E::Context, E::Ownership, Local>>
    where
        E: IntoReport<Local>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report()),
        }
    }

    #[inline(always)]
    fn local_context<C>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        C: ObjectMarker + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report_collection().context(context)),
        }
    }

    #[inline(always)]
    fn local_context_lazy<C, F>(self, context: F) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        F: FnOnce() -> C,
        C: ObjectMarker + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report_collection().context(context())),
        }
    }

    #[inline(always)]
    fn local_context_with_handler<C, H>(self, context: C) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        C: ObjectMarker,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e
                .into_report_collection()
                .context_with_handler::<H, _>(context)),
        }
    }

    #[inline(always)]
    fn local_context_with_handler_lazy<C, F, H>(
        self,
        context: F,
    ) -> Result<V, Report<C, Mutable, Local>>
    where
        E: IntoReportCollection<Local>,
        F: FnOnce() -> C,
        C: ObjectMarker,
        H: handlers::ContextHandler<C>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e
                .into_report_collection()
                .context_with_handler::<H, _>(context())),
        }
    }

    #[inline(always)]
    fn local_attach<A>(self, attachment: A) -> Result<V, Report<<E>::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        A: ObjectMarker + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach(attachment)),
        }
    }

    #[inline(always)]
    fn local_attach_lazy<A, F>(
        self,
        attachment: F,
    ) -> Result<V, Report<<E>::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + core::fmt::Display + core::fmt::Debug,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach(attachment())),
        }
    }

    #[inline(always)]
    fn local_attach_with_handler<A, H>(
        self,
        attachment: A,
    ) -> Result<V, Report<<E>::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        A: ObjectMarker,
        H: handlers::AttachmentHandler<A>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach_with_handler::<H, _>(attachment)),
        }
    }

    #[inline(always)]
    fn local_attach_lazy_with_handler<A, F, H>(
        self,
        attachment: F,
    ) -> Result<V, Report<<E>::Context, Mutable, Local>>
    where
        E: IntoReport<Local, Ownership = Mutable>,
        F: FnOnce() -> A,
        A: ObjectMarker + core::fmt::Display + core::fmt::Debug,
        H: handlers::AttachmentHandler<A>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_report().attach_with_handler::<H, _>(attachment())),
        }
    }
}
