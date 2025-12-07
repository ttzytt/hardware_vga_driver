use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, Ordering};

/// Generic double-buffered storage.
///
/// Internally keeps two copies of `T` and an atomic index selecting
/// which one is currently "active" (read-only for the consumer).
/// The other one is "inactive" (writable for the producer).
pub struct DoubleBuffer<T> {
    bufs: [UnsafeCell<T>; 2],
    active_idx: AtomicU8, // 0 or 1
}

// We promise that if T is Send/Sync, then DoubleBuffer<T> can be
// shared across tasks/cores safely as long as the API is respected.
unsafe impl<T: Send> Send for DoubleBuffer<T> {}
unsafe impl<T: Send + Sync> Sync for DoubleBuffer<T> {}

impl<T: Clone> DoubleBuffer<T> {
    /// Create a new double-buffer, initialising both buffers with `init`.
    pub fn new(init: T) -> Self {
        Self {
            bufs: [UnsafeCell::new(init.clone()), UnsafeCell::new(init)],
            active_idx: AtomicU8::new(0),
        }
    }
}

impl<T> DoubleBuffer<T> {
    #[inline]
    fn active_index(&self) -> usize {
        self.active_idx.load(Ordering::Acquire) as usize
    }

    #[inline]
    fn inactive_index(&self) -> usize {
        1 ^ self.active_index()
    }

    /// Run `f` with an immutable reference to the active buffer.
    ///
    /// The closure must not store the reference beyond its lifetime.
    pub fn with_active<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let idx = self.active_index();
        // Safe because we only hand out &T, and we never mutate this buffer
        // through this method.
        let buf = unsafe { &*self.bufs[idx].get() };
        f(buf)
    }

    /// Run `f` with a mutable reference to the inactive buffer.
    ///
    /// This is intended for the producer (drawing task). You are expected
    /// to call `swap()` once a full frame is ready.
    pub fn with_inactive<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let idx = self.inactive_index();
        // Safe if only the producer calls this and we never alias &mut T.
        let buf = unsafe { &mut *self.bufs[idx].get() };
        f(buf)
    }

    /// Swap active and inactive buffers.
    ///
    /// Typically called by the producer after finishing drawing a frame.
    pub fn swap(&self) {
        let cur = self.active_index() as u8;
        let next = cur ^ 1;
        self.active_idx.store(next, Ordering::Release);
    }
}
