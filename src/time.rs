use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_time() -> u64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    #[cfg(test)]
    let dur = {
        if TEST_TIME.get() == 0 {
            dur
        } else {
            TEST_TIME.set(TEST_TIME.get() + 100);
            core::time::Duration::from_millis(TEST_TIME.get())
        }
    };
    dur.as_nanos() as u64
}

#[cfg(test)]
thread_local! {
    static TEST_TIME: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub fn set_time(time: u64) {
    TEST_TIME.set(time);
}
