#[cfg(feature = "std")]
use std::sync as impl_;

#[cfg(not(feature = "std"))]
use spin as impl_;

pub struct HookLock<T: 'static + Send + Sync>(impl_::RwLock<Option<T>>);
pub struct HookLockReadGuard<T: 'static + Send + Sync>(impl_::RwLockReadGuard<'static, Option<T>>);
pub struct HookLockWriteGuard<T: 'static + Send + Sync>(
    impl_::RwLockWriteGuard<'static, Option<T>>,
);

impl<T: 'static + Send + Sync> HookLock<T> {
    pub const fn new() -> Self {
        Self(impl_::RwLock::new(None))
    }

    pub fn read(&'static self) -> HookLockReadGuard<T> {
        #[cfg(not(feature = "std"))]
        let guard = self.0.read();

        #[cfg(feature = "std")]
        let guard = self.0.read().expect("Unable to lock attachment hooks");

        HookLockReadGuard(guard)
    }

    pub fn write(&'static self) -> HookLockWriteGuard<T> {
        #[cfg(not(feature = "std"))]
        let guard = self.0.write();

        #[cfg(feature = "std")]
        let guard = self.0.write().expect("Unable to lock attachment hooks");

        HookLockWriteGuard(guard)
    }
}

impl<T: 'static + Send + Sync> HookLockReadGuard<T> {
    pub fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

impl<T: 'static + Send + Sync> HookLockWriteGuard<T> {
    pub fn get(&mut self) -> &mut Option<T> {
        &mut self.0
    }
}
