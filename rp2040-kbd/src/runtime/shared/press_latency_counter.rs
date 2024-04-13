use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::rom_data::float_funcs::{fadd, fdiv, fsub, uint64_to_float, uint_to_float};

pub struct PressLatencyCounter {
    count: u32,
    avg: f32,
}

impl PressLatencyCounter {
    pub fn increment_get_avg(&mut self, duration: MicrosDurationU64) -> f32 {
        let f_val = uint64_to_float(duration.to_micros());
        self.count = self.count.wrapping_add(1);
        if self.count == 0 {
            self.count = 1;
            self.avg = 0.0;
        }
        let count_f = uint_to_float(self.count);
        self.avg = fadd(self.avg, fdiv(fsub(f_val, self.avg), count_f));
        self.avg
    }
    pub const fn new() -> Self {
        Self { count: 0, avg: 0.0 }
    }
}
