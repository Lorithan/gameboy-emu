//! #86 F2: timer pacing (docs/tasks/FRONTEND.md). Written ahead of
//! implementation. Time is simulated with `Instant` arithmetic, so the tests
//! never sleep.
//!
//! Frame n (n = 0, 1, 2, ...) is due at `start + deadline(n)`, where
//! `deadline(n)` is n * 70,224 dots at 4,194,304 Hz, truncated to whole
//! nanoseconds from the exact value (never accumulated). Frame 0 is due at
//! the start.

use std::time::{Duration, Instant};

use super::{MAX_CATCH_UP, MCYCLES_PER_FRAME, Pacer};

fn deadline(n: u64) -> Duration {
    Duration::from_nanos((u128::from(n) * 70_224 * 1_000_000_000 / 4_194_304) as u64)
}

const NS: Duration = Duration::from_nanos(1);

#[test]
fn a_frame_is_one_lcd_frame_of_m_cycles() {
    assert_eq!(MCYCLES_PER_FRAME, 17_556);
    assert_eq!(MCYCLES_PER_FRAME * 4, 154 * 456);
}

#[test]
fn the_catch_up_cap_allows_at_least_one_frame() {
    const { assert!(MAX_CATCH_UP >= 1) };
}

#[test]
fn the_first_frame_is_due_at_the_start() {
    let start = Instant::now();
    let mut pacer = Pacer::new(start);
    assert_eq!(pacer.next_due(), start);
    assert_eq!(pacer.frames_due(start), 1);
    assert_eq!(pacer.frames_due(start), 0, "already returned");
    assert_eq!(pacer.next_due(), start + deadline(1));
}

#[test]
fn each_frame_becomes_due_exactly_at_its_deadline() {
    let start = Instant::now();
    let mut pacer = Pacer::new(start);
    assert_eq!(pacer.frames_due(start), 1);
    for n in 1..=600 {
        let due = start + deadline(n);
        assert_eq!(pacer.next_due(), due, "frame {n}");
        assert_eq!(pacer.frames_due(due - NS), 0, "1 ns before frame {n}");
        assert_eq!(pacer.frames_due(due), 1, "frame {n}");
    }
}

#[test]
fn an_hour_of_polling_runs_the_exact_frame_count_with_no_drift() {
    // Polling every 5 ms never leaves more than one frame due, so the cap
    // never applies. 3600 s * 4194304 / 70224 = 215019.002, so frames 0 to
    // 215,019 are due.
    let start = Instant::now();
    let end = start + Duration::from_secs(3600);
    let mut pacer = Pacer::new(start);
    let mut total = 0u64;
    let mut now = start;
    while now <= end {
        total += u64::from(pacer.frames_due(now));
        assert!(pacer.next_due() > now, "next frame not in the future");
        now += Duration::from_millis(5);
    }
    let expected = (0..)
        .take_while(|&n| deadline(n) <= Duration::from_secs(3600))
        .count() as u64;
    assert_eq!(expected, 215_020);
    assert_eq!(total, expected);
}

#[test]
fn a_stall_up_to_the_cap_runs_every_missed_frame() {
    let start = Instant::now();
    let mut pacer = Pacer::new(start);
    assert_eq!(pacer.frames_due(start), 1);
    let late = start + deadline(u64::from(MAX_CATCH_UP));
    assert_eq!(pacer.frames_due(late), MAX_CATCH_UP);
    assert_eq!(
        pacer.next_due(),
        start + deadline(u64::from(MAX_CATCH_UP) + 1),
        "no resynchronisation within the cap"
    );
}

#[test]
fn a_longer_stall_runs_the_cap_and_resynchronises_one_frame_later() {
    // More than MAX_CATCH_UP frames due: run the cap, drop the rest, and
    // count the next frame from `now` instead of the original start.
    let start = Instant::now();
    for stall in [
        deadline(u64::from(MAX_CATCH_UP) + 1),
        Duration::from_secs(1),
        Duration::from_secs(600),
    ] {
        let mut pacer = Pacer::new(start);
        assert_eq!(pacer.frames_due(start), 1);
        let now = start + stall;
        assert_eq!(pacer.frames_due(now), MAX_CATCH_UP, "stall {stall:?}");
        assert_eq!(pacer.frames_due(now), 0, "stall {stall:?}: backlog dropped");
        assert_eq!(pacer.next_due(), now + deadline(1), "stall {stall:?}");
        assert_eq!(pacer.frames_due(now + deadline(1) - NS), 0);
        assert_eq!(pacer.frames_due(now + deadline(1)), 1);
        assert_eq!(pacer.next_due(), now + deadline(2), "stall {stall:?}");
    }
}
