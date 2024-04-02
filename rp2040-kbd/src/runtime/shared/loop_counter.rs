use rp2040_hal::fugit::{MicrosDuration};
use rp2040_hal::timer::Instant;

#[derive(Debug, Copy, Clone)]
pub struct LoopCount {
    pub duration: MicrosDuration<u64>,
    pub count: u32,
}

pub struct LoopCounter<const N: u32> {
    start: Instant,
    count: u32,
}

impl<const N: u32> LoopCounter<N> {
    pub const fn new(instant: Instant) -> Self {
        Self { start: instant, count: 0}
    }

    #[inline]
    pub fn increment(&mut self) -> bool {
        self.count += 1;
        self.count >= N
    }

    #[inline]
    pub fn value(&self, now: Instant) -> LoopCount {
        LoopCount {
            duration: now.checked_duration_since(self.start).unwrap_or(MicrosDuration::<u64>::micros(100)),
            count: self.count,
        }
    }

    #[inline]
    pub fn reset(&mut self, start: Instant) {
        self.start = start;
        self.count = 0;
    }
}