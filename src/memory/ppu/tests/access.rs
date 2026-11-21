//! #77 slice C: CPU VRAM/OAM access restrictions, black-box through Bus
//! reads and writes, against the independent `model` positions. Since #73 D3
//! reads and writes have separate locks (`Pos::*_blocked`).

use super::{
    advance, fresh_bus, memory,
    model::{DOTS_PER_FRAME, DOTS_PER_LINE, MCYCLES_PER_FRAME, Pos, SEED_DOT},
};
use crate::memory::Bus;

const VRAM: [u16; 5] = [0x8000, 0x8001, 0x8FFF, 0x9800, 0x9FFF];
const OAM: [u16; 5] = [0xFE00, 0xFE01, 0xFE4F, 0xFE9E, 0xFE9F];

/// Bus at physical (line, dot), advanced from the seed with no accesses.
fn bus_at(line: u32, dot: u32) -> Bus {
    let dots = (line * DOTS_PER_LINE + dot + DOTS_PER_FRAME - SEED_DOT) % DOTS_PER_FRAME;
    assert_eq!(dots % 4, 0, "reachable at an M-cycle boundary");
    let mut bus = fresh_bus();
    advance(&mut bus, dots / 4);
    bus
}

/// A value never equal to FF, so a stored byte cannot pass for a blocked read.
fn value(step: u32, index: usize) -> u8 {
    ((step * 7 + index as u32 * 31) % 0xFF) as u8
}

/// Observable PPU and interrupt state that blocked accesses must not touch.
fn observable(bus: &Bus) -> (String, u8, u8, u8) {
    (
        bus.ppu_timing_probe(),
        bus.read(0xFF41),
        bus.read(0xFF44),
        bus.read(0xFF0F),
    )
}

#[test]
fn reads_and_writes_follow_the_mode_model_at_every_mcycle() {
    for from_enable in [false, true] {
        let mut bus = fresh_bus();
        let mut reference = fresh_bus();
        if from_enable {
            bus.write(0xFF40, 0x11);
            reference.write(0xFF40, 0x11);
        }
        // Seed in VBlank, or LCD off: every address is writable here.
        let addresses: Vec<u16> = VRAM.iter().chain(&OAM).copied().collect();
        let mut stored: Vec<u8> = (0..addresses.len()).map(|i| value(0, i)).collect();
        for (&addr, &byte) in addresses.iter().zip(&stored) {
            bus.write(addr, byte);
        }
        let mut pos = if from_enable {
            bus.write(0xFF40, 0x91);
            reference.write(0xFF40, 0x91);
            Pos::ENABLED
        } else {
            Pos::SEED
        };
        let mut counts = [[[0u32; 2]; 2]; 2]; // [region][read, write][blocked]
        let mut split = [0u32; 2]; // [region]: read and write locks differ
        let mut first_line_oam_allowed = 0;
        for mcycle in 1..=2 * MCYCLES_PER_FRAME {
            for _ in 0..4 {
                pos = pos.next();
            }
            bus.advance_mcycle();
            reference.advance_mcycle();
            let label = format!("enable {from_enable}, mcycle {mcycle}, {pos:?}");
            for (index, &addr) in addresses.iter().enumerate() {
                let is_oam = addr >= 0xFE00;
                let (read_blocked, write_blocked) = if is_oam {
                    (pos.oam_read_blocked(), pos.oam_write_blocked())
                } else {
                    (pos.vram_read_blocked(), pos.vram_write_blocked())
                };
                let region = usize::from(is_oam);
                counts[region][0][usize::from(read_blocked)] += 1;
                counts[region][1][usize::from(write_blocked)] += 1;
                split[region] += u32::from(read_blocked != write_blocked);
                if is_oam && !read_blocked && pos.first && pos.dot < 80 {
                    first_line_oam_allowed += 1;
                }
                let expected_read = if read_blocked { 0xFF } else { stored[index] };
                assert_eq!(bus.read(addr), expected_read, "read {addr:04X}, {label}");
                let new = value(mcycle, index);
                bus.write(addr, new);
                if !write_blocked {
                    stored[index] = new;
                }
                let expected_read = if read_blocked { 0xFF } else { stored[index] };
                assert_eq!(bus.read(addr), expected_read, "reread {addr:04X}, {label}");
            }
            assert_eq!(observable(&bus), observable(&reference), "{label}");
        }
        for (region, name) in [(0, "VRAM"), (1, "OAM")] {
            // #73 D3 windows (line end, mode 2 tail) were exercised.
            assert!(split[region] > 0, "{name}: no read/write lock split");
            for kind in 0..2 {
                let [allowed, blocked] = counts[region][kind];
                assert!(allowed > 0 && blocked > 0, "{name} {counts:?}");
            }
        }
        if from_enable {
            // Dots 4-76 of the first line report HBlank: 19 M-cycles x 5 addresses.
            assert_eq!(first_line_oam_allowed, 19 * OAM.len(), "first line OAM");
        }
        // Blocked writes never landed: with the LCD off every byte reads back.
        bus.write(0xFF40, 0x11);
        for (&addr, &byte) in addresses.iter().zip(&stored) {
            assert_eq!(
                bus.read(addr),
                byte,
                "final {addr:04X}, enable {from_enable}"
            );
        }
    }
}

#[test]
fn lcd_off_never_blocks_and_disabling_mid_drawing_unblocks_the_next_access() {
    let mut bus = bus_at(10, 160);
    assert_eq!(bus.read(0xFF41) & 0x03, 3, "positioned in Drawing");
    assert_eq!(bus.read(0x8000), 0xFF);
    assert_eq!(bus.read(0xFE00), 0xFF);
    bus.write(0xFF40, 0x11);
    // Zero-initialized storage is visible immediately.
    assert_eq!(bus.read(0x8000), 0x00);
    assert_eq!(bus.read(0xFE00), 0x00);
    for mcycle in 0..MCYCLES_PER_FRAME + 100 {
        for (index, &addr) in VRAM.iter().chain(&OAM).enumerate() {
            let byte = value(mcycle, index);
            bus.write(addr, byte);
            assert_eq!(bus.read(addr), byte, "off, {addr:04X}, mcycle {mcycle}");
        }
        bus.advance_mcycle();
    }
}

#[test]
fn unusable_fea0_feff_keeps_its_fallback_in_every_mode() {
    let mut bus = fresh_bus();
    for (index, addr) in (0xFE00u16..=0xFE9F).enumerate() {
        bus.write(addr, value(1, index));
    }
    let oam_before: Vec<u8> = (0xFE00u16..=0xFE9F).map(|addr| bus.read(addr)).collect();
    let mut modes = [false; 4];
    for mcycle in 0..MCYCLES_PER_FRAME {
        modes[usize::from(bus.read(0xFF41) & 0x03)] = true;
        for addr in 0xFEA0u16..=0xFEFF {
            bus.write(addr, value(mcycle, 0));
            assert_eq!(bus.read(addr), 0xFF, "{addr:04X}, mcycle {mcycle}");
        }
        bus.advance_mcycle();
    }
    assert_eq!(modes, [true; 4], "every mode visited");
    bus.write(0xFF40, 0x11);
    for addr in 0xFEA0u16..=0xFEFF {
        bus.write(addr, 0x12);
        assert_eq!(bus.read(addr), 0xFF, "off, {addr:04X}");
    }
    let oam_after: Vec<u8> = (0xFE00u16..=0xFE9F).map(|addr| bus.read(addr)).collect();
    assert_eq!(oam_after, oam_before, "the tail does not alias OAM");
}

#[test]
fn blocked_accesses_change_no_other_state() {
    // Drawing blocks both regions.
    let mut bus = bus_at(10, 160);
    let mut reference = bus_at(10, 160);
    let before = memory(&bus);
    for addr in (0x8000u16..=0x9FFF).chain(0xFE00..=0xFE9F) {
        bus.write(addr, 0x5A);
        assert_eq!(bus.read(addr), 0xFF, "{addr:04X}");
    }
    assert_eq!(memory(&bus), before, "LCD on, after blocked writes");
    assert_eq!(observable(&bus), observable(&reference));
    // With the LCD off, the hidden storage is identical too.
    bus.write(0xFF40, 0x11);
    reference.write(0xFF40, 0x11);
    assert_eq!(memory(&bus), memory(&reference), "LCD off");
}
