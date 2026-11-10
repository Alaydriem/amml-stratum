use std::time::{Duration, Instant};

use alaydriem_stratum::services::RenderMeter;

#[test]
fn a_window_times_the_intervals_between_takes() {
    let meter = RenderMeter::new();
    meter.set_enabled(true);
    let start = Instant::now();

    meter.take_at(start);
    meter.take_at(start + Duration::from_micros(10_000));
    meter.take_at(start + Duration::from_micros(30_000));

    let window = meter.take();
    assert_eq!(window.frames, 2);
    assert_eq!(window.avg_us, 15_000);
    assert_eq!(window.max_us, 20_000);
    assert_eq!(meter.take().frames, 0);
}

#[test]
fn the_p99_is_the_bucket_of_the_99th_percentile_frame() {
    let meter = RenderMeter::new();
    for _ in 0..99 {
        meter.record(950);
    }
    meter.record(40_000);

    let window = meter.take();
    assert_eq!(window.p99_us, 1_000);
    assert_eq!(window.max_us, 40_000);

    for _ in 0..98 {
        meter.record(950);
    }
    meter.record(16_950);
    meter.record(40_000);
    assert_eq!(meter.take().p99_us, 17_000);
}

#[test]
fn a_frame_past_the_last_bucket_reads_as_the_slowest_frame() {
    let meter = RenderMeter::new();
    meter.record(90_000);

    let window = meter.take();
    assert_eq!(window.p99_us, 90_000);
}

#[test]
fn a_gap_longer_than_a_second_is_not_a_frame() {
    let meter = RenderMeter::new();
    meter.set_enabled(true);
    let start = Instant::now();

    meter.take_at(start);
    meter.take_at(start + Duration::from_secs(5));
    meter.take_at(start + Duration::from_secs(5) + Duration::from_micros(16_000));

    let window = meter.take();
    assert_eq!(window.frames, 1);
    assert_eq!(window.max_us, 16_000);
}

#[test]
fn time_while_switched_off_is_not_counted() {
    let meter = RenderMeter::new();
    let start = Instant::now();
    meter.take_at(start);
    meter.take_at(start + Duration::from_micros(5_000));
    assert_eq!(meter.take().frames, 0);

    meter.set_enabled(true);
    meter.take_at(start + Duration::from_micros(500_000));
    meter.take_at(start + Duration::from_micros(508_000));

    let window = meter.take();
    assert_eq!(window.frames, 1);
    assert_eq!(window.max_us, 8_000);
}
