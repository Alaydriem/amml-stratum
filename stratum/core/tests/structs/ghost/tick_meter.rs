use stratum_core::structs::ghost::GhostTickMeter;

#[test]
fn a_window_gives_the_average_and_the_slowest_tick() {
    let mut meter = GhostTickMeter::new();
    meter.observe(10);
    meter.observe(50);
    meter.observe(30);

    let window = meter.take();

    assert_eq!(window.avg_us, 30);
    assert_eq!(window.max_us, 50);
}

#[test]
fn taking_a_window_starts_the_next_one_empty() {
    let mut meter = GhostTickMeter::new();
    meter.observe(40);
    let _ = meter.take();

    let window = meter.take();

    assert_eq!((window.avg_us, window.max_us), (0, 0));
}

#[test]
fn a_huge_sample_does_not_overflow_the_sum() {
    let mut meter = GhostTickMeter::new();
    meter.observe(u64::MAX);
    meter.observe(u64::MAX);

    assert_eq!(meter.take().max_us, u64::MAX);
}
