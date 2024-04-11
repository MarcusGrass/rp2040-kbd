use rp2040_hal::timer::Instant;

#[derive(Copy, Clone, Debug, Default)]
pub struct PinDebouncer {
    last_touch: Option<Instant>,
    quarantined: Option<bool>,
}

// Effectively, maximum presses per single key becomes is 1 per 20 millis, that's a 600 WPM with a single finger
const QUARANTINE_MICROS: u64 = 10_000;

impl PinDebouncer {
    pub const fn new() -> Self {
        Self {
            last_touch: None,
            quarantined: None,
        }
    }

    #[cfg(all(feature = "serial", feature = "right"))]
    pub fn diff_last(&self, now: Instant) -> Option<u64> {
        self.last_touch
            .and_then(|last| now.checked_duration_since(last))
            .map(|diff| diff.to_micros())
    }

    pub fn try_submit(&mut self, now: Instant, state: bool) -> bool {
        let Some(earlier) = self.last_touch else {
            self.last_touch = Some(now);
            return true;
        };
        let Some(diff) = now.checked_duration_since(earlier) else {
            self.last_touch = Some(now);
            return true;
        };
        if diff.to_micros() < QUARANTINE_MICROS {
            return if self.quarantined == Some(state) {
                false
            } else {
                self.quarantined = Some(state);
                self.last_touch = Some(now);
                false
            };
        }

        self.last_touch = Some(now);
        self.quarantined.take();
        true
    }
}
