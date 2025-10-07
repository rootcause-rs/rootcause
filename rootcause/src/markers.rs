use core::any::Any;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Mutable;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Cloneable;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Uncloneable;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct SendSync;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct Local;

mod sealed_report_ownership_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for Mutable {}
    impl Sealed for Cloneable {}
}

mod sealed_report_ref_ownership_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for Cloneable {}
    impl Sealed for Uncloneable {}
}

mod sealed_send_sync_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl Sealed for SendSync {}
    impl Sealed for Local {}
}

mod sealed_context_marker {
    use super::*;

    pub trait Sealed: 'static {}

    impl<C: 'static> Sealed for C {}
    impl Sealed for dyn Any {}
}

pub trait ObjectMarker: 'static + sealed_context_marker::Sealed {}
impl<T> ObjectMarker for T where T: 'static {}
impl ObjectMarker for dyn Any {}

pub trait ReportOwnershipMarker: sealed_report_ownership_marker::Sealed {
    type RefMarker: ReportRefOwnershipMarker;
}
impl ReportOwnershipMarker for Mutable {
    type RefMarker = Uncloneable;
}
impl ReportOwnershipMarker for Cloneable {
    type RefMarker = Cloneable;
}

pub trait ReportRefOwnershipMarker: sealed_report_ref_ownership_marker::Sealed {}
impl ReportRefOwnershipMarker for Cloneable {}
impl ReportRefOwnershipMarker for Uncloneable {}

pub trait ThreadSafetyMarker: sealed_send_sync_marker::Sealed {}
impl ThreadSafetyMarker for SendSync {}
impl ThreadSafetyMarker for Local {}

pub trait ObjectMarkerFor<T: ThreadSafetyMarker>: ObjectMarker {}
impl<T> ObjectMarkerFor<Local> for T where T: ObjectMarker {}
impl<T> ObjectMarkerFor<SendSync> for T where T: ObjectMarker + Send + Sync {}
impl ObjectMarkerFor<Local> for dyn Any {}
impl ObjectMarkerFor<SendSync> for dyn Any {}
