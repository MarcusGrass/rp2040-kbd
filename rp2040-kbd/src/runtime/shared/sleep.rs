use rp2040_hal::timer::Instant;

const SLEEP_AFTER_SECONDS: u64 = 60;

pub struct SleepCountdown {
    is_asleep: bool,
    touched_last: Instant,
}

impl SleepCountdown {
    pub const fn new() -> Self {
        Self {
            is_asleep: false,
            touched_last: Instant::from_ticks(0),
        }
    }

    #[inline]
    pub fn touch(&mut self, now: Instant) {
        self.touched_last = now;
        self.is_asleep = false;
    }

    #[inline]
    pub fn should_sleep(&mut self, now: Instant) -> bool {
        !self.is_asleep
            && now
                .checked_duration_since(self.touched_last)
                .is_some_and(|dur| dur.to_secs() > SLEEP_AFTER_SECONDS)
    }

    #[inline]
    pub fn set_sleeping(&mut self) {
        self.is_asleep = true;
    }

    #[inline]
    pub fn is_awake(&self) -> bool {
        !self.is_asleep
    }
}
