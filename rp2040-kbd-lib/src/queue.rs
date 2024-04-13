use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Queue<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    head: usize,
    tail: usize,
}

impl<T, const N: usize> Queue<T, N> {
    const NULL: MaybeUninit<T> = MaybeUninit::uninit();

    #[inline]
    fn rem(&self) -> usize {
        if self.head > self.tail {
            self.head - self.tail
        } else {
            N - self.tail + self.head
        }
    }

    #[inline]
    pub fn push_back(&mut self, val: T) -> bool {
        if self.rem() == 0 {
            return false;
        }
        if self.tail == N {
            self.tail = 0;
        }
        // Safety: Tail always in range and points to initialized memory
        unsafe {
            #[cfg(test)]
            {
                self.buffer.get(self.tail).unwrap();
            }
            let cur = self.buffer.get_unchecked_mut(self.tail);
            cur.as_mut_ptr().write(val);
        }
        self.tail += 1;
        true
    }

    #[inline]
    pub fn peek(&self) -> Option<&T> {
        if self.head == self.tail {
            return None;
        };
        // Safety: Head always in range (always moves after tail) and points to initialized memory
        let val = unsafe {
            #[cfg(test)]
            {
                self.buffer.get(self.head).unwrap();
            }
            let cur = self.buffer.get_unchecked(self.head);
            cur.as_ptr().as_ref()
        };
        val
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        if self.head == self.tail {
            return None;
        };
        // Safety: Head always in range (always moves after tail) and points to initialized memory
        let val = unsafe {
            #[cfg(test)]
            {
                self.buffer.get(self.head).unwrap();
            }
            let cur = self.buffer.get_unchecked_mut(self.head);
            cur.as_ptr().read()
        };
        if self.head >= N - 1 {
            self.head = 0;
            if self.tail == N {
                self.tail = 0;
            }
        } else {
            self.head += 1;
        }
        Some(val)
    }

    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [Self::NULL; N],
            head: 0,
            tail: 0,
        }
    }
}

pub struct AtomicQueueProducer<'a, T, const N: usize> {
    buffer: *mut T, // Length N
    head: &'a AtomicUsize,
    tail: &'a AtomicUsize,
}

unsafe impl<'a, T, const N: usize> Send for AtomicQueueProducer<'a, T, N> {}

impl<'a, T, const N: usize> AtomicQueueProducer<'a, T, N> {
    pub fn push_back(&self, val: T) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let next = (tail + 1) % N;
        let head = self.head.load(Ordering::Acquire);
        if next == head {
            #[cfg(test)]
            {
                eprintln!("bail head={head}, tail={tail}");
            }
            return false;
        }
        unsafe {
            let cur = self.buffer.add(tail);
            cur.write(val);
        }
        self.tail.store(next, Ordering::Release);
        true
    }
}

pub struct AtomicQueueConsumer<'a, T, const N: usize> {
    buffer: *mut T, // Length N
    head: &'a AtomicUsize,
    tail: &'a AtomicUsize,
}
impl<'a, T, const N: usize> AtomicQueueConsumer<'a, T, N> {
    #[must_use]
    pub fn available(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        tail.wrapping_sub(head).wrapping_add(N) % N
    }
    #[inline]
    #[must_use]
    pub fn peek(&self) -> Option<&T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        #[cfg(test)]
        {
            eprintln!("peek head={head}, tail={tail}");
        }
        if head == tail {
            return None;
        };
        // Safety: Head always in range (always moves after tail) and points to initialized memory
        let val = unsafe {
            let cur = self.buffer.add(head);
            cur.as_ref()
        };
        val
    }

    #[inline]
    #[must_use]
    pub fn pop_front(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        #[cfg(test)]
        {
            eprintln!("pop head={head}, tail={tail}");
        }
        if head == tail {
            return None;
        };
        // Safety: Head always in range (always moves after tail) and points to initialized memory
        let val = unsafe {
            let cur = self.buffer.add(head);
            cur.read()
        };
        let next_head = (head + 1) % N;
        self.head.store(next_head, Ordering::Release);
        Some(val)
    }
}

#[must_use]
pub fn new_atomic_producer_consumer<'a, T, const N: usize>(
    mem_area: &'a mut [T; N],
    head: &'a mut AtomicUsize,
    tail: &'a mut AtomicUsize,
) -> (AtomicQueueProducer<'a, T, N>, AtomicQueueConsumer<'a, T, N>) {
    let buf = mem_area.as_mut_ptr();
    head.store(0, Ordering::Release);
    tail.store(0, Ordering::Release);
    (
        AtomicQueueProducer {
            buffer: buf,
            head,
            tail,
        },
        AtomicQueueConsumer {
            buffer: buf,
            head,
            tail,
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::queue::{new_atomic_producer_consumer, Queue};
    use core::sync::atomic::AtomicUsize;

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
            assert_eq!(Some(&i), queue.peek());
            assert_eq!(Some(i), queue.pop_front());
        }
        assert!(queue.pop_front().is_none());
        for i in 0..128 {
            queue.push_back(i);
        }
        for i in 0..128 {
            assert_eq!(Some(&i), queue.peek());
            assert_eq!(Some(i), queue.pop_front());
        }
        assert!(queue.pop_front().is_none());
    }

    #[test]
    fn wrap() {
        let mut queue: Queue<u8, 128> = Queue::new();
        assert!(queue.peek().is_none());
        assert!(queue.pop_front().is_none());
        queue.push_back(1);
        queue.push_back(2);
        queue.push_back(3);
        assert_eq!(&1, queue.peek().unwrap());
        assert_eq!(1, queue.pop_front().unwrap());
        assert_eq!(&2, queue.peek().unwrap());
        assert_eq!(2, queue.pop_front().unwrap());
        assert_eq!(&3, queue.peek().unwrap());
        assert_eq!(3, queue.pop_front().unwrap());
        assert!(queue.pop_front().is_none());
        for i in 27..27 + 64 {
            queue.push_back(i);
            assert_eq!(Some(&i), queue.peek());
            assert_eq!(Some(i), queue.pop_front());
            assert!(queue.peek().is_none());
            assert!(queue.pop_front().is_none());
        }
        assert!(queue.peek().is_none());
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
    #[test]
    fn atomic_push_to_cap() {
        let mut area = [0u8; 128];
        let mut head = AtomicUsize::new(999);
        let mut tail = AtomicUsize::new(1527);
        let (producer, consumer) = new_atomic_producer_consumer(&mut area, &mut head, &mut tail);
        assert!(consumer.pop_front().is_none());
        for i in 0..u8::MAX {
            producer.push_back(i);
            let val = consumer.pop_front();
            assert_eq!(Some(i), val);
        }
        assert!(consumer.pop_front().is_none());
    }

    #[test]
    fn atomic_fill_clear() {
        let mut area = [0u8; 8];
        let mut head = AtomicUsize::new(999);
        let mut tail = AtomicUsize::new(1527);
        let (producer, consumer) = new_atomic_producer_consumer(&mut area, &mut head, &mut tail);
        assert!(consumer.pop_front().is_none());
        for i in 0..7 {
            producer.push_back(i);
        }
        for i in 0..7 {
            assert_eq!(Some(&i), consumer.peek());
            assert_eq!(Some(i), consumer.pop_front());
        }
        assert!(consumer.pop_front().is_none());
        for i in 0..7 {
            producer.push_back(i);
        }
        for i in 0..7 {
            assert_eq!(Some(&i), consumer.peek());
            assert_eq!(Some(i), consumer.pop_front());
        }
        assert!(consumer.pop_front().is_none());
    }

    #[test]
    fn atomic_wrap() {
        let mut area = [0u8; 128];
        let mut head = AtomicUsize::new(999);
        let mut tail = AtomicUsize::new(1527);
        let (producer, consumer) = new_atomic_producer_consumer(&mut area, &mut head, &mut tail);
        assert!(consumer.peek().is_none());
        assert!(consumer.pop_front().is_none());
        producer.push_back(1);
        producer.push_back(2);
        producer.push_back(3);
        assert_eq!(&1, consumer.peek().unwrap());
        assert_eq!(1, consumer.pop_front().unwrap());
        assert_eq!(&2, consumer.peek().unwrap());
        assert_eq!(2, consumer.pop_front().unwrap());
        assert_eq!(&3, consumer.peek().unwrap());
        assert_eq!(3, consumer.pop_front().unwrap());
        assert!(consumer.pop_front().is_none());
        for i in 27..27 + 64 {
            producer.push_back(i);
            assert_eq!(Some(&i), consumer.peek());
            assert_eq!(Some(i), consumer.pop_front());
            assert!(consumer.peek().is_none());
            assert!(consumer.pop_front().is_none());
        }
        assert!(consumer.peek().is_none());
        assert!(consumer.pop_front().is_none());
    }

    #[test]
    fn atomic_wrap_chunks() {
        let mut area = [0i32; 1024];
        let mut head = AtomicUsize::new(999);
        let mut tail = AtomicUsize::new(1527);
        let (producer, consumer) = new_atomic_producer_consumer(&mut area, &mut head, &mut tail);
        assert!(consumer.pop_front().is_none());
        for i in 0..1024 {
            for j in 0..i {
                producer.push_back(j);
            }
            for j in 0..i {
                let val = consumer.pop_front();
                assert_eq!(Some(j), val, "{i}");
            }
            assert!(consumer.pop_front().is_none());
        }
    }
}
