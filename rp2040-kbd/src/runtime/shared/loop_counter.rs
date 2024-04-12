use core::fmt::Write;
use heapless::String;
use rp2040_hal::fugit::MicrosDuration;
use rp2040_hal::rom_data::float_funcs::{fdiv, uint64_to_float, uint_to_float};
use rp2040_hal::timer::Instant;

#[derive(Debug, Copy, Clone)]
pub struct LoopCount {
    pub duration: MicrosDuration<u64>,
    pub count: u32,
}

impl LoopCount {
    pub fn as_display(&self) -> Option<(String<5>, String<5>)> {
        let count = uint_to_float(self.count);
        let micros = uint64_to_float(self.duration.to_micros());
        let res = fdiv(micros, count);
        if res <= 999.9 {
            let mut header = String::new();
            let _ = header.push_str("my");
            let mut body = String::new();
            let _ = body.write_fmt(format_args!("{res:.1}"));
            Some((header, body))
        } else {
            None
        }
    }
}

pub struct LoopCounter<const N: u32> {
    start: Instant,
    count: u32,
}

impl<const N: u32> LoopCounter<N> {
    pub const fn new(instant: Instant) -> Self {
        Self {
            start: instant,
            count: 0,
        }
    }

    #[inline]
    pub fn increment(&mut self) -> bool {
        self.count += 1;
        self.count >= N
    }

    #[inline]
    pub fn value(&self, now: Instant) -> LoopCount {
        LoopCount {
            duration: now
                .checked_duration_since(self.start)
                .unwrap_or(MicrosDuration::<u64>::micros(100)),
            count: self.count,
        }
    }

    #[inline]
    pub fn reset(&mut self, start: Instant) {
        self.start = start;
        self.count = 0;
    }
}
