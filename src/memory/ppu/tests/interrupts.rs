//! #76 slice B: STAT and VBlank interrupt requests. Ppu-level tests compare
//! `Ppu::tick` effects with an independent line/edge model; Bus-level tests
//! observe requests only through IF (FF0F). The position model (`model`)
//! shares no code with production timing or status logic.

use super::{
    advance, fresh_bus,
    model::{DOTS_PER_FRAME, DOTS_PER_LINE, MCYCLES_PER_FRAME, Pos, SEED_DOT},
};
use crate::memory::{
    Bus,
    ppu::{Ppu, PpuEffects},
};

/// Ticks from the seed to physical line 144 / dot 0.
const SEED_TO_VBLANK: u32 = DOTS_PER_FRAME - SEED_DOT + 144 * DOTS_PER_LINE;
const LYCS: [u8; 6] = [0, 1, 143, 144, 153, 0xFF];

/// A Ppu with `selects`/`lyc` installed, plus the model's position and stored
/// line level. From the seed the writes happen while on (write effects are
/// discarded; the stored level becomes the real line). From enable they
/// happen while off, so the stored level keeps its startup value (low).
fn prepared(from_enable: bool, selects: u8, lyc: u8) -> (Ppu, Pos, bool) {
    let mut ppu = Ppu::dmg_post_boot();
    if from_enable {
        ppu.write_lcdc(0x11);
    }
    let _ = ppu.write_lyc(lyc);
    let _ = ppu.write_stat(selects);
    if from_enable {
        ppu.write_lcdc(0x91);
        (ppu, Pos::ENABLED, false)
    } else {
        (ppu, Pos::SEED, Pos::SEED.line_level(selects, lyc))
    }
}

/// (VBlank effects, STAT effects) over `ticks` ticks from the seed.
fn counts_from_seed(selects: u8, lyc: u8, ticks: u32) -> (u32, u32) {
    let (mut ppu, _, _) = prepared(false, selects, lyc);
    let (mut vblank, mut stat) = (0, 0);
    for _ in 0..ticks {
        let effects = ppu.tick();
        vblank += u32::from(effects.vblank);
        stat += u32::from(effects.lcd_stat);
    }
    (vblank, stat)
}

#[test]
fn tick_effects_match_the_line_model_for_every_select_and_lyc() {
    for from_enable in [false, true] {
        for selects in (0u8..16).map(|bits| bits << 3) {
            for lyc in LYCS {
                let (mut ppu, mut pos, mut stored) = prepared(from_enable, selects, lyc);
                let mut requests = 0;
                for tick in 1..=2 * DOTS_PER_FRAME {
                    pos = pos.next();
                    let level = pos.line_level(selects, lyc);
                    let expected = PpuEffects {
                        vblank: pos.pulse(),
                        lcd_stat: !stored && level,
                    };
                    stored = level;
                    requests += u32::from(expected.lcd_stat);
                    assert_eq!(
                        ppu.tick(),
                        expected,
                        "enable {from_enable}, STAT {selects:02X}, LYC {lyc:02X}, tick {tick}, {pos:?}"
                    );
                }
                // Guard against a vacuous model: any select except a lone
                // LYC FF must produce requests.
                if selects & 0x38 != 0 || (selects & 0x40 != 0 && lyc != 0xFF) {
                    assert!(requests > 0, "STAT {selects:02X}, LYC {lyc:02X}");
                }
            }
        }
    }
}

#[test]
fn per_frame_request_counts_match_hand_derived_values() {
    // One full frame from the seed (LYC FF unless the case selects LYC).
    let cases = [
        (0x00, 0xFF, 0, "no selects"),
        (0x08, 0xFF, 144, "HBlank: one per visible line"),
        (0x20, 0xFF, 145, "OAM: 144 lines plus the line-144 pulse"),
        (0x10, 0xFF, 1, "VBlank: once"),
        (0x40, 0x00, 1, "LYC 0 (line 153 dot 4 through line 0)"),
        (0x40, 0x50, 1, "LYC 80"),
        (0x40, 0x99, 1, "LYC 153 (line 153 dots 0-3)"),
        (0x40, 0xFF, 0, "LYC FF never matches"),
        (
            0x18,
            0xFF,
            144,
            "HBlank+VBlank: VBlank blocked behind HBlank",
        ),
        // The pulse rises at line 144, VBlank keeps the line high through
        // line 153, and line 0's OAM scan follows without a low gap: lines
        // 1-143 plus the pulse.
        (
            0x30,
            0xFF,
            144,
            "OAM+VBlank: line 0 OAM blocked behind VBlank",
        ),
    ];
    for (selects, lyc, expected, label) in cases {
        let (vblank, stat) = counts_from_seed(selects, lyc, DOTS_PER_FRAME);
        assert_eq!(stat, expected, "{label}");
        assert_eq!(vblank, 1, "{label}");
    }
}

#[test]
fn vblank_effect_occurs_once_per_frame_at_line_144_dot_0_for_any_selects() {
    for selects in (0u8..16).map(|bits| bits << 3) {
        let (mut ppu, _, _) = prepared(false, selects, 0);
        let mut at = Vec::new();
        for tick in 1..=3 * DOTS_PER_FRAME {
            if ppu.tick().vblank {
                at.push(tick);
            }
        }
        let expected: Vec<u32> = (0..3)
            .map(|frame| SEED_TO_VBLANK + frame * DOTS_PER_FRAME)
            .collect();
        assert_eq!(at, expected, "STAT {selects:02X}");
    }
}

// ---- Bus level: requests observed through IF only ----

fn stat_requested(bus: &Bus) -> bool {
    bus.read(0xFF0F) & 0x02 != 0
}

fn clear_if(bus: &mut Bus) {
    bus.write(0xFF0F, 0);
}

/// Bus advanced from the seed to physical (line, dot), with IF cleared.
fn bus_at(line: u32, dot: u32) -> Bus {
    let target = line * DOTS_PER_LINE + dot;
    let dots = (target + DOTS_PER_FRAME - SEED_DOT) % DOTS_PER_FRAME;
    assert_eq!(dots % 4, 0, "reachable at an M-cycle boundary");
    let mut bus = fresh_bus();
    advance(&mut bus, dots / 4);
    clear_if(&mut bus);
    bus
}

#[test]
fn startup_requests_nothing_until_the_first_vblank() {
    let mut bus = fresh_bus();
    // Post-boot IF already has VBlank set; construction adds no STAT request.
    assert_eq!(bus.read(0xFF0F), 0xE1, "post-boot IF");
    clear_if(&mut bus);
    let vblank_mcycle = SEED_TO_VBLANK / 4;
    for mcycle in 1..vblank_mcycle {
        bus.advance_mcycle();
        assert_eq!(bus.read(0xFF0F), 0xE0, "mcycle {mcycle}");
    }
    bus.advance_mcycle();
    assert_eq!(bus.read(0xFF0F), 0xE1, "VBlank only, at line 144");
}

#[test]
fn bus_requests_match_the_model_through_if_at_every_mcycle() {
    // VBlank and STAT, through advance_mcycle's effect handling.
    for (selects, lyc) in [
        (0x08, 0xFF),
        (0x20, 0xFF),
        (0x10, 0xFF),
        (0x40, 0x50),
        (0x78, 0x00),
    ] {
        let mut bus = fresh_bus();
        let _ = bus.ppu.write_lyc(lyc);
        let _ = bus.ppu.write_stat(selects);
        clear_if(&mut bus);
        let (mut pos, mut stored) = (Pos::SEED, Pos::SEED.line_level(selects, lyc));
        for mcycle in 1..=2 * MCYCLES_PER_FRAME {
            let (mut vblank, mut stat) = (false, false);
            for _ in 0..4 {
                pos = pos.next();
                let level = pos.line_level(selects, lyc);
                stat |= !stored && level;
                vblank |= pos.pulse();
                stored = level;
            }
            bus.advance_mcycle();
            let expected = 0xE0 | u8::from(vblank) | (u8::from(stat) << 1);
            assert_eq!(
                bus.read(0xFF0F),
                expected,
                "STAT {selects:02X}, LYC {lyc:02X}, mcycle {mcycle}"
            );
            clear_if(&mut bus);
        }
    }
}

#[test]
fn lyc_writes_while_on_compare_immediately_and_request_only_on_rising_edges() {
    let mut bus = bus_at(10, 300); // HBlank of line 10, LYC 0: flag clear.
    bus.write(0xFF41, 0x40);
    clear_if(&mut bus); // The STAT-write quirk requested here (HBlank).
    assert_eq!(bus.read(0xFF41) & 0x04, 0);

    bus.write(0xFF45, 10);
    assert_eq!(bus.read(0xFF41) & 0x04, 0x04, "flag set by the write");
    assert!(stat_requested(&bus), "rising edge from the write");
    clear_if(&mut bus);

    bus.write(0xFF45, 10);
    assert!(!stat_requested(&bus), "already high: no edge");

    bus.write(0xFF45, 11);
    assert_eq!(bus.read(0xFF41) & 0x04, 0, "flag cleared by the write");
    assert!(!stat_requested(&bus), "falling edge");

    bus.write(0xFF45, 10);
    assert!(stat_requested(&bus), "rises again");
    clear_if(&mut bus);

    // Without the LYC select the flag still follows writes, with no request.
    bus.write(0xFF41, 0x00);
    clear_if(&mut bus);
    for lyc in [11, 10, 11, 10] {
        bus.write(0xFF45, lyc);
        assert_eq!(bus.read(0xFF41) & 0x04 != 0, lyc == 10);
        assert!(!stat_requested(&bus), "LYC {lyc}, no select");
    }
}

#[test]
fn stat_write_quirk_requests_once_when_any_source_is_active() {
    // (line, dot, LYC, label, expect request): all with the line low before
    // the write (selects 0).
    let cases = [
        (10, 40, 0xFF, "OAM scan", true),
        (10, 160, 0xFF, "Drawing, flag clear", false),
        (10, 300, 0xFF, "HBlank", true),
        (150, 100, 0xFF, "VBlank", true),
        (10, 160, 10, "Drawing, flag set", true),
    ];
    for (line, dot, lyc, label, expected) in cases {
        let mut bus = bus_at(line, dot);
        bus.write(0xFF45, lyc); // Selects 0: compares, cannot request.
        assert_eq!(bus.read(0xFF41) & 0x04 != 0, lyc == 10, "{label}");
        assert!(!stat_requested(&bus), "{label}: LYC write");
        let pos = Pos {
            line,
            dot,
            first: false,
        };
        assert_eq!(bus.read(0xFF41) & 0x03, pos.mode(), "{label}: positioned");
        for write in 0..2 {
            bus.write(0xFF41, 0x00);
            assert_eq!(stat_requested(&bus), expected, "{label}, write {write}");
            assert_eq!(bus.read(0xFF41) & 0x78, 0, "real selects stored");
            clear_if(&mut bus);
        }
        // The re-evaluated line is low with selects 0: nothing follows.
        advance(&mut bus, 2);
        assert!(!stat_requested(&bus), "{label}: no later request");
    }

    // Line already high from the written source: no new edge.
    let mut bus = bus_at(10, 300);
    bus.write(0xFF41, 0x08);
    clear_if(&mut bus);
    bus.write(0xFF41, 0x08);
    assert!(!stat_requested(&bus), "HBlank select rewritten in HBlank");
}

/// Advance one M-cycle at a time until `done`, asserting no STAT request.
fn poll_without_stat(bus: &mut Bus, mut done: impl FnMut(u8) -> bool, seen_mode_3: &mut bool) {
    for _ in 0..MCYCLES_PER_FRAME {
        let stat = bus.read(0xFF41);
        *seen_mode_3 |= stat & 0x03 == 3;
        if done(stat) {
            return;
        }
        bus.advance_mcycle();
        assert!(!stat_requested(bus), "STAT requested while polling");
    }
    panic!("poll condition never met");
}

#[test]
fn stat_irq_blocking_rounds_reproduced_through_bus() {
    // Round 1: already in VBlank, writing STAT $10 requests.
    let mut bus = bus_at(144, 8);
    bus.write(0xFF41, 0x10);
    assert!(stat_requested(&bus), "round 1");
    clear_if(&mut bus);

    // Round 2: STAT $78 (line already high in VBlank), then for LYC 0-143
    // wait for the flag, then for mode 0. Only mode 3 can lower the line,
    // and the flag holds it high through mode 3, so nothing is requested.
    bus.write(0xFF41, 0x78);
    assert!(!stat_requested(&bus), "round 2 STAT write");
    for lyc in 0u8..0x90 {
        bus.write(0xFF45, lyc);
        assert!(!stat_requested(&bus), "LYC write {lyc}");
        let mut seen_mode_3 = false;
        poll_without_stat(&mut bus, |stat| stat & 0x04 != 0, &mut seen_mode_3);
        poll_without_stat(&mut bus, |stat| stat & 0x03 == 0, &mut seen_mode_3);
        assert!(seen_mode_3, "LYC {lyc}: passed through mode 3");
        assert_eq!(bus.read(0xFF44), lyc);
    }
}

#[test]
fn lcd_off_writes_and_time_request_nothing() {
    let mut bus = bus_at(10, 300);
    bus.write(0xFF40, 0x11);
    clear_if(&mut bus);
    for value in 0u8..=u8::MAX {
        bus.write(0xFF41, value);
        bus.write(0xFF45, value);
        bus.write(0xFF45, 0);
        advance(&mut bus, 300);
        assert_eq!(bus.read(0xFF0F), 0xE0, "off, value {value:02X}");
    }
    // Disabling and enabling writes request nothing at the write either.
    bus.write(0xFF41, 0x00);
    bus.write(0xFF40, 0x91);
    assert_eq!(bus.read(0xFF0F), 0xE0, "enable write");
    let mut on = bus_at(10, 300);
    on.write(0xFF41, 0x78);
    clear_if(&mut on);
    on.write(0xFF40, 0x11);
    assert_eq!(on.read(0xFF0F), 0xE0, "disable write");
}

/// Advance until visible LY reads `ly`.
fn run_to_ly(bus: &mut Bus, ly: u8) {
    for _ in 0..MCYCLES_PER_FRAME {
        if bus.read(0xFF44) == ly {
            return;
        }
        bus.advance_mcycle();
    }
    panic!("LY {ly} never sampled");
}

#[test]
fn stat_lyc_onoff_rounds_request_only_on_a_held_false_to_true_flag() {
    for (retained, after_enable) in [(true, false), (true, true), (false, false), (false, true)] {
        let label = format!("retained {retained}, after enable {after_enable}");
        let mut bus = fresh_bus();
        bus.write(0xFF45, 0x50);
        bus.write(0xFF41, 0x40); // LYC select only.
        run_to_ly(&mut bus, if retained { 0x50 } else { 0x51 });
        // Since #73 D3, LY first shows a line during the previous line's
        // last four dots, when the flag reads 0; step into the line proper.
        advance(&mut bus, 2);
        assert_eq!(bus.read(0xFF41) & 0x04 != 0, retained, "{label}");
        bus.write(0xFF40, 0x11);
        clear_if(&mut bus);
        for lyc in [0u8, 0x50, 0x51, 0xFF] {
            bus.write(0xFF45, lyc);
            advance(&mut bus, 1_000);
        }
        bus.write(0xFF45, if after_enable { 0 } else { 0x50 });
        bus.write(0xFF40, 0x91);
        assert!(
            !stat_requested(&bus),
            "{label}: no request at the enable write"
        );
        bus.advance_mcycle();
        assert_eq!(
            stat_requested(&bus),
            !retained && after_enable,
            "{label}: first enabled M-cycle"
        );
        assert_eq!(bus.read(0xFF41) & 0x04 != 0, after_enable, "{label}");
    }
}
