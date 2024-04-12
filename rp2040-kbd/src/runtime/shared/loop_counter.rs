use core::fmt::Write;
use heapless::String;
use rp2040_hal::fugit::MicrosDuration;
use rp2040_hal::rom_data::float_funcs::fdiv;
use rp2040_hal::timer::Instant;

#[derive(Debug, Copy, Clone)]
pub struct LoopCount {
    pub duration: MicrosDuration<u64>,
    pub count: u32,
}

impl LoopCount {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    pub fn as_display(&self) -> Option<(String<5>, String<5>)> {
        if self.count > f32::MAX as u32 {
            return None;
        }
        let count = self.count as f32;
        let micros = self.duration.to_micros();
        if micros < f32::MAX as u64 {
            let dur = micros as f32;
            let res = fdiv(dur, count);
            if res <= 9999.9 {
                let mut header = String::new();
                let _ = header.push_str("my");
                let mut body = String::new();
                let _ = body.write_fmt(format_args!("{res:.1}"));
                return Some((header, body));
            }
        }
        let millis = self.duration.to_millis();
        if millis >= f32::MAX as u64 {
            return None;
        }
        let dur = millis as f32;
        let res = fdiv(dur, count);
        if res > 9999.9 {
            return None;
        }
        let mut header = String::new();
        let _ = header.push_str("ms");
        let mut body = String::new();
        let _ = body.write_fmt(format_args!("{res:.1}"));
        Some((header, body))
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
