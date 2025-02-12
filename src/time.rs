pub fn current_time() -> u64 {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    #[cfg(test)]
    let dur = {
        if *TEST_TIME.lock().unwrap() == 0 {
            dur
        } else {
            *TEST_TIME.lock().unwrap() += 100;
            core::time::Duration::from_millis(*TEST_TIME.lock().unwrap())
        }
    };
    dur.as_nanos() as u64
}

#[cfg(test)]
static TEST_TIME: std::sync::LazyLock<std::sync::Mutex<u64>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(0));

#[cfg(test)]
pub fn set_time(time: u64) {
    *TEST_TIME.lock().unwrap() = time;
}
