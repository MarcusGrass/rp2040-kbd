use rp2040_hal::Timer;

pub fn wait_nanos(timer: Timer, nanos: u64) {
    let start = timer.get_counter();
    loop {
        let Some(dur) = timer.get_counter().checked_duration_since(start) else {
            continue;
        };
        if dur.to_nanos() >= nanos {
            return;
        }
    }
}
