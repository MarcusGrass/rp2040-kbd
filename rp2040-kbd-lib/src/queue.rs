use core::mem::MaybeUninit;

pub struct Queue<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    head: usize,
    tail: usize,
}

impl<T, const N: usize> Queue<T, N> {
    const NULL: MaybeUninit<T> = MaybeUninit::uninit();

    #[inline]
    fn rem(&self) -> usize {
        let occ = if self.head > self.tail {
            N - (self.head - self.tail)
        } else {
            self.tail - self.head
        };
        N - occ
    }
    pub fn push_back(&mut self, val: T) -> bool {
        if self.rem() == 0 {
            return false;
        }
        // Safety: Tail always in range and points to initialized memory
        unsafe {
            let cur = self.buffer.get_unchecked_mut(self.tail);
            cur.as_mut_ptr().write(val);
        }
        if self.tail >= N - 1 {
            self.tail = 0;
        } else {
            self.tail += 1;
        }
        true
    }
    pub fn peek(&self) -> Option<&T> {
        if self.head == self.tail {
            return None;
        };
        // Safety: Head always in range (always moves after tail) and points to initialized memory
        let val = unsafe {
            let cur = self.buffer.get_unchecked(self.head);
            cur.as_ptr().as_ref()
        };
        val
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.head == self.tail {
            return None;
        };
        // Safety: Head always in range (always moves after tail) and points to initialized memory
        let val = unsafe {
            let cur = self.buffer.get_unchecked_mut(self.head);
            cur.as_ptr().read()
        };
        self.head += 1;
        if self.head > N - 1 {
            self.head = 0;
        }
        Some(val)
    }

    pub const fn new() -> Self {
        Self {
            buffer: [Self::NULL; N],
            head: 0,
            tail: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::queue::Queue;

    #[test]
    fn push_to_cap() {
        let mut queue: Queue<u8, 128> = Queue::new();
        assert!(queue.pop_front().is_none());
        for i in 0..u8::MAX {
            queue.push_back(i);
            let val = queue.pop_front();
            assert_eq!(Some(i), val);
        }
        assert!(queue.pop_front().is_none());
    }

    #[test]
    fn fill_clear() {
        let mut queue: Queue<u8, 128> = Queue::new();
        assert!(queue.pop_front().is_none());
        for i in 0..128 {
            queue.push_back(i);
        }
        for i in 0..128 {
            let val = queue.pop_front();
            assert_eq!(Some(i), val);
        }
        assert!(queue.pop_front().is_none());
    }

    #[test]
    fn wrap() {
        let mut queue: Queue<u8, 128> = Queue::new();
        assert!(queue.pop_front().is_none());
        queue.push_back(1);
        queue.push_back(2);
        queue.push_back(3);
        assert_eq!(1, queue.pop_front().unwrap());
        assert_eq!(2, queue.pop_front().unwrap());
        assert_eq!(3, queue.pop_front().unwrap());
        assert!(queue.pop_front().is_none());
        for i in 27..27 + 64 {
            queue.push_back(i);
            let val = queue.pop_front();
            assert_eq!(Some(i), val);
            assert!(queue.pop_front().is_none());
        }
        assert!(queue.pop_front().is_none());
    }

    #[test]
    fn wrap_chunks() {
        let mut queue: Queue<u8, 128> = Queue::new();
        assert!(queue.pop_front().is_none());
        for i in 0..128 {
            for j in 0..i {
                queue.push_back(j);
            }
            for j in 0..i {
                let val = queue.pop_front();
                assert_eq!(Some(j), val);
            }
            assert!(queue.pop_front().is_none());
        }
    }
}
