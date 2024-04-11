use core::mem::MaybeUninit;

#[derive(Copy)]
pub struct RingBuffer<T, const N: usize> {
    ring: [MaybeUninit<T>; N],
    start: usize,
    filled: usize,
}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<const N: usize, T: Copy> Clone for RingBuffer<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: usize, T> RingBuffer<T, N> {
    const EMPTY: MaybeUninit<T> = MaybeUninit::zeroed();

    /// # Panics
    /// [`RingBuffer`] size too small
    #[must_use]
    pub const fn new() -> Self {
        assert!(N > 1, "A ring-buffer of 1 makes no sense");
        Self {
            ring: [Self::EMPTY; N],
            start: 0,
            filled: 0,
        }
    }

    #[inline]
    fn has_slot(&self) -> bool {
        self.filled < N
    }

    pub fn try_push(&mut self, val: T) -> bool {
        if !self.has_slot() {
            return false;
        }
        let mut offset = self.start + self.filled;
        if offset > N - 1 {
            offset -= N;
        }
        unsafe {
            self.ring[offset].as_mut_ptr().write(val);
        }
        self.filled += 1;
        true
    }

    pub fn try_pop(&mut self) -> Option<T> {
        if self.filled == 0 {
            return None;
        }
        let res = unsafe { Some(self.ring[self.start].as_ptr().read()) };
        if self.start >= N - 1 {
            self.start = 0;
        } else {
            self.start += 1;
        }
        self.filled -= 1;
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::ring_buffer::RingBuffer;

    #[test]
    fn can_push_pop() {
        let mut rb: RingBuffer<i32, 16> = RingBuffer::new();
        assert!(rb.try_pop().is_none());
        assert!(rb.try_push(0));
        assert_eq!(Some(0), rb.try_pop());
        assert!(rb.try_pop().is_none());
    }
    #[test]
    fn can_wrap() {
        let mut rb: RingBuffer<i32, 8> = RingBuffer::new();
        assert!(rb.try_pop().is_none());
        for i in 0..256 {
            assert!(rb.try_push(i));
            assert_eq!(Some(i), rb.try_pop());
        }
        assert!(rb.try_pop().is_none());
    }
    #[test]
    fn can_fill_drain() {
        let mut rb: RingBuffer<i32, 128> = RingBuffer::new();
        assert!(rb.try_pop().is_none());
        for i in 0..128 {
            assert!(rb.try_push(i));
        }
        for i in 0..128 {
            assert_eq!(Some(i), rb.try_pop());
        }
        assert!(rb.try_pop().is_none());
    }
    #[test]
    fn wrap_chunks() {
        let mut rb: RingBuffer<u8, 128> = RingBuffer::new();
        assert!(rb.try_pop().is_none());
        for i in 0..128 {
            for j in 0..i {
                rb.try_push(j);
            }
            for j in 0..i {
                let val = rb.try_pop();
                assert_eq!(Some(j), val);
            }
            assert!(rb.try_pop().is_none());
        }
    }
}
