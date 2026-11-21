use super::{PpuMode, PpuTiming};

const DOTS_PER_LINE: u32 = 456;
const LINES_PER_FRAME: u32 = 154;
const DOTS_PER_FRAME: u32 = DOTS_PER_LINE * LINES_PER_FRAME;

/// Independent oracle for the provisional #74 schedule, written from the
/// contract's range table rather than the production boundary transitions.
/// `elapsed` counts dots since line 0 / dot 0 of the reference frame.
fn model(elapsed: u32, starts_with_first_enabled_line: bool) -> PpuTiming {
    let frame_dot = elapsed % DOTS_PER_FRAME;
    let line = (frame_dot / DOTS_PER_LINE) as u8;
    let dot = (frame_dot % DOTS_PER_LINE) as u16;
    let first_enabled_line = starts_with_first_enabled_line && elapsed < DOTS_PER_LINE;
    let mode = match (line, dot) {
        (144..=153, _) => PpuMode::VBlank,
        (_, 0..=79) if first_enabled_line => PpuMode::HBlank,
        (_, 0..=79) => PpuMode::OamScan,
        (_, 80..=251) => PpuMode::Drawing,
        (_, 252..=455) => PpuMode::HBlank,
        _ => unreachable!("dot outside line"),
    };
    PpuTiming {
        line,
        dot,
        mode,
        first_enabled_line,
    }
}

fn state(line: u8, dot: u16, mode: PpuMode, first_enabled_line: bool) -> PpuTiming {
    PpuTiming {
        line,
        dot,
        mode,
        first_enabled_line,
    }
}

#[test]
fn constructors_match_the_contract_states() {
    assert_eq!(
        PpuTiming::dmg_post_boot(),
        state(153, 4, PpuMode::VBlank, false)
    );
    assert_eq!(PpuTiming::lcd_on(), state(0, 0, PpuMode::HBlank, true));
    assert_eq!(PpuTiming::lcd_off(), state(0, 0, PpuMode::HBlank, false));
    // Enabling starts exactly at the model's first-line origin.
    assert_eq!(PpuTiming::lcd_on(), model(0, true));
}

#[test]
fn oracle_spot_checks_match_the_contract_table() {
    // Guards the oracle itself against sharing an off-by-one with production.
    let line = |n: u32| n * DOTS_PER_LINE;
    for (elapsed, first, expected) in [
        (79, true, state(0, 79, PpuMode::HBlank, true)),
        (80, true, state(0, 80, PpuMode::Drawing, true)),
        (251, true, state(0, 251, PpuMode::Drawing, true)),
        (252, true, state(0, 252, PpuMode::HBlank, true)),
        (455, true, state(0, 455, PpuMode::HBlank, true)),
        (line(1), true, state(1, 0, PpuMode::OamScan, false)),
        (line(1) + 79, false, state(1, 79, PpuMode::OamScan, false)),
        (line(1) + 80, false, state(1, 80, PpuMode::Drawing, false)),
        (
            line(143) + 455,
            false,
            state(143, 455, PpuMode::HBlank, false),
        ),
        (line(144), false, state(144, 0, PpuMode::VBlank, false)),
        (
            line(144) + 80,
            false,
            state(144, 80, PpuMode::VBlank, false),
        ),
        (
            line(153) + 455,
            false,
            state(153, 455, PpuMode::VBlank, false),
        ),
        (DOTS_PER_FRAME, true, state(0, 0, PpuMode::OamScan, false)),
    ] {
        assert_eq!(model(elapsed, first), expected, "elapsed {elapsed}");
    }
}

#[test]
fn every_dot_after_enable_matches_the_model_for_three_frames() {
    let mut timing = PpuTiming::lcd_on();
    for elapsed in 1..=3 * DOTS_PER_FRAME {
        timing.tick();
        assert_eq!(timing, model(elapsed, true), "elapsed {elapsed}");
    }
    // A full frame later, the first-line identity is gone: ordinary line 0.
    assert_eq!(timing, state(0, 0, PpuMode::OamScan, false));
}

#[test]
fn ordinary_frames_repeat_every_70224_dots_exactly() {
    let mut timing = PpuTiming::lcd_on();
    for _ in 0..DOTS_PER_FRAME {
        timing.tick();
    }
    let frame_start = timing;
    for elapsed in 1..DOTS_PER_FRAME {
        timing.tick();
        assert_ne!(timing, frame_start, "returned early at {elapsed}");
    }
    timing.tick();
    assert_eq!(timing, frame_start);
}

#[test]
fn post_boot_seed_reaches_ordinary_line_zero_after_exactly_452_ticks() {
    let mut timing = PpuTiming::dmg_post_boot();
    for tick in 1..452u16 {
        timing.tick();
        assert_eq!(
            timing,
            state(153, 4 + tick, PpuMode::VBlank, false),
            "tick {tick}"
        );
    }
    timing.tick();
    assert_eq!(timing, state(0, 0, PpuMode::OamScan, false));
    for elapsed in 1..=2 * DOTS_PER_FRAME {
        timing.tick();
        assert_eq!(timing, model(elapsed, false), "after seed {elapsed}");
    }
}

/// #75 visible LY: physical line, except line 153 shows 153 only at dots 0-3.
fn model_visible_ly(elapsed: u32) -> u8 {
    let frame_dot = elapsed % DOTS_PER_FRAME;
    let (line, dot) = (frame_dot / DOTS_PER_LINE, frame_dot % DOTS_PER_LINE);
    match (line, dot) {
        (153, 4..) => 0,
        // #73 D3: the last four dots of lines 0-142 already show the next line.
        (0..=142, 452..) => line as u8 + 1,
        _ => line as u8,
    }
}

#[test]
fn stat_bits_match_the_documented_mode_numbers() {
    for (mode, bits) in [
        (PpuMode::HBlank, 0),
        (PpuMode::VBlank, 1),
        (PpuMode::OamScan, 2),
        (PpuMode::Drawing, 3),
    ] {
        assert_eq!(mode.stat_bits(), bits, "{mode:?}");
    }
}

#[test]
fn visible_ly_and_mode_match_the_model_at_every_dot() {
    // From enable (first-line state) and from the post-boot seed.
    let seed_elapsed = 153 * DOTS_PER_LINE + 4;
    for (mut timing, start, first) in [
        (PpuTiming::lcd_on(), 0, true),
        (PpuTiming::dmg_post_boot(), seed_elapsed, false),
    ] {
        for step in 0..=2 * DOTS_PER_FRAME {
            let elapsed = start + step;
            let expected = model(elapsed, first);
            assert_eq!(timing.mode(), expected.mode, "elapsed {elapsed}");
            assert_eq!(
                timing.visible_ly(),
                model_visible_ly(elapsed),
                "elapsed {elapsed}"
            );
            timing.tick();
        }
    }
}

#[test]
fn line_153_shows_153_for_exactly_four_dots_then_zero() {
    let mut timing = PpuTiming::lcd_on();
    for _ in 0..153 * DOTS_PER_LINE {
        timing.tick();
    }
    assert_eq!((timing.line, timing.dot), (153, 0));
    for dot in 0..DOTS_PER_LINE {
        let expected = if dot < 4 { 153 } else { 0 };
        assert_eq!(timing.visible_ly(), expected, "dot {dot}");
        assert_eq!(timing.line, 153, "physical line stays 153");
        timing.tick();
    }
    assert_eq!((timing.line, timing.visible_ly()), (0, 0));
}

#[test]
fn lcd_off_state_reads_ly_0_and_hblank() {
    let off = PpuTiming::lcd_off();
    assert_eq!((off.visible_ly(), off.mode()), (0, PpuMode::HBlank));
}
