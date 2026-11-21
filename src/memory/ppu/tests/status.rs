//! #75 slice A: live LY (FF44) and STAT (FF41) readback, black-box through Bus.
//! Written ahead of implementation; wire with `mod status;` in ppu/tests.rs
//! once #75 lands. The model below is independent of production timing code.

use super::{advance, fresh_bus};
use crate::memory::Bus;

const DOTS_PER_LINE: u32 = 456;
const DOTS_PER_FRAME: u32 = DOTS_PER_LINE * 154;
const MCYCLES_PER_FRAME: u32 = DOTS_PER_FRAME / 4;
/// Post-boot seed: physical line 153, dot 4, as dots since line 0 / dot 0.
const SEED: u32 = 153 * DOTS_PER_LINE + 4;

/// (visible LY, STAT mode bits) at `frame_dot` of an ordinary frame.
fn model(frame_dot: u32) -> (u8, u8) {
    let line = frame_dot / DOTS_PER_LINE;
    let dot = frame_dot % DOTS_PER_LINE;
    let visible_ly = if line == 153 && dot >= 4 {
        0
    } else {
        line as u8
    };
    let mode = match (line, dot) {
        (144..=153, _) => 1,
        (_, 0..=79) => 2,
        (_, 80..=251) => 3,
        _ => 0,
    };
    (visible_ly, mode)
}

fn expected_stat(select: u8, flag: bool, mode: u8) -> u8 {
    0x80 | (select & 0x78) | (u8::from(flag) << 2) | mode
}

fn memory(bus: &Bus) -> Vec<u8> {
    (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect()
}

#[test]
fn startup_reads_ly_00_and_stat_85_without_read_effects() {
    let bus = fresh_bus();
    let before = memory(&bus);
    for _ in 0..3 {
        assert_eq!(bus.read(0xFF44), 0x00);
        assert_eq!(bus.read(0xFF41), 0x85);
    }
    assert_eq!(memory(&bus), before);
}

#[test]
fn stat_writes_store_only_select_bits_for_every_byte() {
    let mut bus = fresh_bus();
    for value in 0u8..=u8::MAX {
        bus.write(0xFF41, value);
        // Startup: flag set (LY 0 == LYC 0), VBlank.
        assert_eq!(
            bus.read(0xFF41),
            expected_stat(value, true, 1),
            "{value:02X}"
        );
        assert_eq!(bus.read(0xFF44), 0);
    }
}

#[test]
fn stat_bit_7_reads_set_and_select_bits_persist_while_lcd_is_off() {
    let mut bus = fresh_bus();
    bus.write(0xFF40, 0x11); // Off with the startup flag (true) held.
    for value in 0u8..=u8::MAX {
        bus.write(0xFF41, value);
        advance(&mut bus, 3);
        assert_eq!(
            bus.read(0xFF41),
            expected_stat(value, true, 0),
            "off, {value:02X}"
        );
    }
    // Select bits written while off survive re-enable.
    bus.write(0xFF41, 0x28);
    bus.write(0xFF40, 0x91);
    assert_eq!(bus.read(0xFF41) & 0x78, 0x28);
}

#[test]
fn ly_writes_are_ignored_for_every_byte() {
    let mut bus = fresh_bus();
    let mut reference = fresh_bus();
    advance(&mut bus, 1_000);
    advance(&mut reference, 1_000);
    for value in 0u8..=u8::MAX {
        bus.write(0xFF44, value);
        assert_eq!(memory(&bus), memory(&reference), "write {value:02X}");
        bus.advance_mcycle();
        reference.advance_mcycle();
        assert_eq!(bus.ppu.timing, reference.ppu.timing);
    }
}

#[test]
fn ly_and_stat_match_the_model_at_every_mcycle() {
    // Readback only; the interrupt requests these selects cause since #76
    // are checked in `interrupts`.
    for lyc in [0u8, 1, 143, 144, 153, 0xFF] {
        let mut bus = fresh_bus();
        bus.write(0xFF45, lyc);
        bus.write(0xFF41, 0x78);
        let mut ly_153_samples = 0;
        for mcycle in 1..=2 * MCYCLES_PER_FRAME {
            bus.advance_mcycle();
            let (ly, mode) = model((SEED + 4 * mcycle) % DOTS_PER_FRAME);
            ly_153_samples += u32::from(ly == 153);
            assert_eq!(bus.read(0xFF44), ly, "LYC {lyc}, mcycle {mcycle}");
            assert_eq!(
                bus.read(0xFF41),
                expected_stat(0x78, ly == lyc, mode),
                "LYC {lyc}, mcycle {mcycle}"
            );
        }
        assert_eq!(ly_153_samples, 2, "one LY 153 sample per frame");
    }
}

/// Advance until visible LY reads `ly`, returning the bus at that sample.
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
fn lcd_off_holds_the_flag_across_lyc_writes_and_enable_recompares_ly_0() {
    // The four stat_lyc_onoff states: retained flag x comparison after enable.
    for (retained, after_enable) in [(true, false), (true, true), (false, false), (false, true)] {
        let mut bus = fresh_bus();
        bus.write(0xFF45, 0x50);
        run_to_ly(&mut bus, if retained { 0x50 } else { 0x51 });
        assert_eq!(bus.read(0xFF41) & 0x04 != 0, retained);
        bus.write(0xFF40, 0x11);
        for _ in 0..3 {
            for lyc in [0u8, 0x50, 0x51, 0xFF] {
                bus.write(0xFF45, lyc);
                advance(&mut bus, 1_000);
                assert_eq!(bus.read(0xFF44), 0, "LY while off");
                assert_eq!(
                    bus.read(0xFF41),
                    expected_stat(0, retained, 0),
                    "held flag, LYC {lyc:02X}"
                );
            }
        }
        bus.write(0xFF45, if after_enable { 0 } else { 0x50 });
        bus.write(0xFF40, 0x91);
        // Holds until the first enabled tick, then compares visible LY 0.
        assert_eq!(bus.read(0xFF41), expected_stat(0, retained, 0));
        bus.advance_mcycle();
        assert_eq!(bus.read(0xFF44), 0);
        // First line after enable reports HBlank (mode 0) before Drawing.
        assert_eq!(bus.read(0xFF41), expected_stat(0, after_enable, 0));
    }
}
