//! Frame statistics count only the time a session runs, not paused time.

use std::time::{Duration, Instant};

use super::Stopwatch;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn stopped() -> Stopwatch {
    Stopwatch {
        total: Duration::ZERO,
        running_since: None,
    }
}

#[test]
fn a_stopwatch_never_started_reads_zero() {
    let t = Instant::now();
    let mut watch = stopped();
    assert_eq!(watch.elapsed(t + ms(500)), Duration::ZERO);
    watch.stop(t + ms(600));
    assert_eq!(watch.elapsed(t + ms(700)), Duration::ZERO);
}

#[test]
fn it_counts_while_running_and_holds_while_stopped() {
    let t = Instant::now();
    let mut watch = stopped();
    watch.start(t);
    assert_eq!(watch.elapsed(t + ms(250)), ms(250), "running");
    watch.stop(t + ms(300));
    assert_eq!(watch.elapsed(t + ms(300)), ms(300));
    assert_eq!(
        watch.elapsed(t + ms(10_000)),
        ms(300),
        "paused time not counted"
    );
}

#[test]
fn running_stretches_add_up() {
    let t = Instant::now();
    let mut watch = stopped();
    watch.start(t);
    watch.stop(t + ms(100)); // 100 running
    watch.start(t + ms(1_100)); // 1000 paused
    watch.stop(t + ms(1_400)); // 300 running
    watch.start(t + ms(5_000)); // 3600 paused
    assert_eq!(watch.elapsed(t + ms(5_050)), ms(450), "ends while running");
    watch.stop(t + ms(5_050));
    assert_eq!(watch.elapsed(t + ms(9_000)), ms(450), "ends while paused");
}

#[test]
fn starting_or_stopping_twice_changes_nothing() {
    let t = Instant::now();
    let mut watch = stopped();
    watch.start(t);
    watch.start(t + ms(400)); // already running: keeps the first start
    assert_eq!(watch.elapsed(t + ms(500)), ms(500));
    watch.stop(t + ms(500));
    watch.stop(t + ms(900)); // already stopped: adds nothing
    assert_eq!(watch.elapsed(t + ms(1_000)), ms(500));
}
