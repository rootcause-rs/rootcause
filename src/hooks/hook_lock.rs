#[cfg(feature = "std")]
use std::sync as impl_;

#[cfg(not(feature = "std"))]
use spin as impl_;

#[repr(transparent)]
pub(crate) struct HookLock<T: 'static + Send + Sync>(impl_::RwLock<Option<T>>);

#[repr(transparent)]
pub(crate) struct HookLockReadGuard<T: 'static + Send + Sync>(
    impl_::RwLockReadGuard<'static, Option<T>>,
);

#[repr(transparent)]
pub(crate) struct HookLockWriteGuard<T: 'static + Send + Sync>(
    impl_::RwLockWriteGuard<'static, Option<T>>,
);

impl<T: 'static + Send + Sync> HookLock<T> {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self(impl_::RwLock::new(None))
    }

    #[inline]
    pub(crate) fn read(&'static self) -> HookLockReadGuard<T> {
        #[cfg(not(feature = "std"))]
        let guard = self.0.read();

        #[cfg(feature = "std")]
        let guard = self.0.read().expect("Unable to acquire hook lock");

        HookLockReadGuard(guard)
    }

    #[inline]
    pub(crate) fn write(&'static self) -> HookLockWriteGuard<T> {
        #[cfg(not(feature = "std"))]
        let guard = self.0.write();

        #[cfg(feature = "std")]
        let guard = self.0.write().expect("Unable to acquire hook lock");

        HookLockWriteGuard(guard)
    }
}

impl<T: 'static + Send + Sync> HookLockReadGuard<T> {
    #[inline]
    pub(crate) fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

impl<T: 'static + Send + Sync> HookLockWriteGuard<T> {
    #[inline]
    pub(crate) fn get(&mut self) -> &mut Option<T> {
        &mut self.0
    }
}
