use crate::debugger::DebugBuffer;
use core::cell::UnsafeCell;
use rp2040_hal::sio::{Spinlock, Spinlock0, Spinlock16};

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct SpinLockN<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for SpinLockN<T> {}

impl<T> SpinLockN<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    #[inline]
    pub(crate) fn lock_mutex(&self) -> SpinLockGuard<T, Spinlock0> {
        let lock = Spinlock::claim();
        SpinLockGuard {
            _lock: lock,
            value: unsafe { self.value.get().as_mut().unwrap_unchecked() },
        }
    }

    pub(crate) fn lock_debugger(&self) -> SpinLockGuard<T, Spinlock16> {
        let lock = Spinlock::claim();
        SpinLockGuard {
            _lock: lock,
            value: unsafe { self.value.get().as_mut().unwrap_unchecked() },
        }
    }
}

#[derive(Debug)]
pub(crate) struct SpinLockGuard<'a, T, L> {
    _lock: L,
    pub(crate) value: &'a mut T,
}
