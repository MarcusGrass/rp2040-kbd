use core::mem::MaybeUninit;

#[derive(Copy)]
pub(crate) struct RingBuffer<const N: usize, T> {
    ring: [MaybeUninit<T>; N],
    start: usize,
    filled: usize,
}

impl<const N: usize, T: Copy> Clone for RingBuffer<N, T> {
    fn clone(&self) -> Self {
        Self {
            ring: self.ring.clone(),
            start: self.start,
            filled: self.filled,
        }
    }
}

impl<const N: usize, T> RingBuffer<N, T> {
    const EMPTY: MaybeUninit<T> = MaybeUninit::zeroed();

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
