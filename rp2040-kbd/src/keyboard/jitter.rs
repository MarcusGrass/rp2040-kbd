use rp2040_hal::timer::Instant;

#[derive(Copy, Clone, Debug, Default)]
pub struct JitterRegulator {
    last_touch: Option<Instant>,
    quarantined: Option<bool>,
}

impl JitterRegulator {
    pub const fn new() -> Self {
        Self {
            last_touch: None,
            quarantined: None,
        }
    }
    pub fn try_submit(&mut self, now: Instant, state: bool) -> bool {
        let Some(earlier) = self.last_touch else {
            self.last_touch = Some(now);
            return true;
        };
        self.last_touch = Some(now);
        let Some(diff) = now.checked_duration_since(earlier) else {
            return true;
        };
        // Effectively, maximum presses per single key becomes is 1 per 10 millis, that's a 1200 WPM with a single finger
        if diff.to_micros() < 10_000 {
            self.quarantined = Some(state);
            return false;
        }

        self.quarantined.take();
        true
    }

    pub fn try_release_quarantined(&mut self, now: Instant) -> Option<bool> {
        let _quarantined = self.quarantined?;
        let last = self.last_touch?;
        let diff = now.checked_duration_since(last)?;
        if diff.to_millis() < 10_000 {
            return None;
        }
        self.quarantined.take()
    }
}
