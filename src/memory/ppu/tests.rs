//! #74 Ppu/Bus-level LCDC and dot-clock acceptance tests. This module is a
//! descendant of both `memory` and `ppu`, so it may inspect private Bus/Ppu
//! state directly; PpuTiming fields stay private and are compared whole.

mod interrupts;
mod status;

use super::timing::PpuTiming;
use crate::memory::{Bus, Cartridge};

const MCYCLES_PER_FRAME: u32 = 70_224 / 4;

fn fresh_bus() -> Bus {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    Bus::new(Cartridge::from_bytes(&rom).expect("valid synthetic cartridge"))
}

fn ticked(mut timing: PpuTiming, ticks: u32) -> PpuTiming {
    for _ in 0..ticks {
        timing.tick();
    }
    timing
}

fn advance(bus: &mut Bus, mcycles: u32) {
    for _ in 0..mcycles {
        bus.advance_mcycle();
    }
}

fn memory(bus: &Bus) -> Vec<u8> {
    (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect()
}

#[test]
fn lcdc_startup_and_every_byte_read_back_exactly_without_read_effects() {
    let mut bus = fresh_bus();
    assert_eq!(bus.read(0xFF40), 0x91);
    assert_eq!(bus.ppu.timing, PpuTiming::dmg_post_boot());
    for start in [0x11u8, 0x91] {
        for value in 0u8..=u8::MAX {
            bus.write(0xFF40, start);
            bus.write(0xFF40, value);
            let timing = bus.ppu.timing;
            for _ in 0..3 {
                assert_eq!(bus.read(0xFF40), value, "start {start:02X}");
            }
            assert_eq!(bus.ppu.timing, timing, "read changed timing");
        }
    }
}

#[test]
fn lcdc_writes_apply_only_enable_edges_for_every_byte() {
    // Advance far enough that the live state differs from every constructor.
    const SETUP_MCYCLES: u32 = 1_234;
    for old in [0x00u8, 0x11, 0x7F, 0x80, 0x91, 0xFF] {
        for new in 0u8..=u8::MAX {
            let mut bus = fresh_bus();
            bus.write(0xFF40, old);
            advance(&mut bus, SETUP_MCYCLES);
            let before = bus.ppu.timing;
            let old_on = old & 0x80 != 0;
            if old_on {
                // Same-enabled post-boot writes preserve the seeded phase.
                let expected = ticked(PpuTiming::dmg_post_boot(), 4 * SETUP_MCYCLES);
                assert_eq!(before, expected, "setup {old:02X}");
                assert_ne!(before, PpuTiming::lcd_on());
                assert_ne!(before, PpuTiming::lcd_off());
            } else {
                assert_eq!(before, PpuTiming::lcd_off(), "frozen {old:02X}");
            }
            bus.write(0xFF40, new);
            let expected = match (old_on, new & 0x80 != 0) {
                (true, false) => PpuTiming::lcd_off(),
                (false, true) => PpuTiming::lcd_on(),
                _ => before,
            };
            assert_eq!(bus.ppu.timing, expected, "{old:02X} -> {new:02X}");
            assert_eq!(bus.read(0xFF40), new);
        }
    }
}

#[test]
fn disabled_lcd_freezes_and_repeated_disabled_writes_never_restart_it() {
    let mut off = fresh_bus();
    let mut on = fresh_bus();
    for bus in [&mut off, &mut on] {
        bus.write(0xFFFF, 0x1F);
        bus.write(0xFF06, 0xF0);
        bus.write(0xFF07, 5); // Fast timer: overflows/IRQs occur while off.
    }
    advance(&mut off, 10);
    advance(&mut on, 10);
    off.write(0xFF40, 0x11);
    assert_eq!(off.ppu.timing, PpuTiming::lcd_off());
    for value in 0u8..0x80 {
        advance(&mut off, 300);
        advance(&mut on, 300);
        off.write(0xFF40, value);
        assert_eq!(off.ppu.timing, PpuTiming::lcd_off(), "write {value:02X}");
        // Timer and its interrupt requests continue identically while off.
        assert_eq!(off.timer, on.timer);
        assert_eq!(off.read(0xFF0F), on.read(0xFF0F));
    }
    assert_ne!(on.ppu.timing, PpuTiming::lcd_off());
    // Restart counts from the enable write, not from the time spent off.
    off.write(0xFF40, 0x91);
    assert_eq!(off.ppu.timing, PpuTiming::lcd_on());
    let mut reference = PpuTiming::lcd_on();
    for mcycle in 1..=2 * MCYCLES_PER_FRAME {
        off.advance_mcycle();
        reference = ticked(reference, 4);
        assert_eq!(off.ppu.timing, reference, "mcycle {mcycle}");
    }
}

#[test]
fn each_bus_mcycle_advances_four_dots_and_a_frame_returns_to_the_seed() {
    let mut bus = fresh_bus();
    let mut reference = PpuTiming::dmg_post_boot();
    for mcycle in 1..=2 * MCYCLES_PER_FRAME + 7 {
        bus.advance_mcycle();
        reference = ticked(reference, 4);
        assert_eq!(bus.ppu.timing, reference, "mcycle {mcycle}");
        if mcycle % MCYCLES_PER_FRAME == 0 {
            assert_eq!(bus.ppu.timing, PpuTiming::dmg_post_boot(), "frame {mcycle}");
        }
    }
}

#[test]
fn lcdc_writes_preserve_memory_devices_interrupts_and_other_instances() {
    let mut bus = fresh_bus();
    let other = fresh_bus();
    for (address, value) in [
        (0x8000, 0x35),
        (0x9FFF, 0x76),
        (0xFE00, 0x91),
        (0xFE9F, 0x28),
        (0xC000, 0xA5),
        (0xDFFF, 0x5A),
        (0xFF80, 0x19),
        (0xFFFE, 0xE2),
        (0xFF0F, 0x11),
        (0xFFFF, 0x1F),
        (0xFF05, 0x80),
        (0xFF06, 0x42),
        (0xFF07, 5),
    ] {
        bus.write(address, value);
    }
    for address in [
        0xFF42, 0xFF43, 0xFF45, 0xFF47, 0xFF48, 0xFF49, 0xFF4A, 0xFF4B,
    ] {
        bus.write(address, address as u8 ^ 0x5A);
    }
    advance(&mut bus, 77);
    let timer = bus.timer;
    let mut expected = memory(&bus);
    let untouched = memory(&other);
    let other_timing = other.ppu.timing;
    // Cover every edge class: on->on, on->off, off->off, off->on.
    let mut enabled = true;
    for value in [0x93u8, 0x13, 0x00, 0x7F, 0x80, 0xFF, 0x01, 0x91] {
        bus.write(0xFF40, value);
        expected[0xFF40] = value;
        if enabled != (value & 0x80 != 0) {
            // Either edge restarts timing at line 0 / HBlank: LY 0, mode 0.
            // STAT's select bits and the held LY=LYC flag are unchanged.
            expected[0xFF44] = 0;
            expected[0xFF41] &= !0x03;
            enabled = !enabled;
        }
        assert_eq!(memory(&bus), expected, "after LCDC {value:02X}");
        assert_eq!(bus.timer, timer);
        assert_eq!(memory(&other), untouched);
        assert_eq!(other.ppu.timing, other_timing);
    }
}

#[test]
fn stop_entry_preserves_ppu_phase() {
    let mut bus = fresh_bus();
    advance(&mut bus, 500);
    let timing = bus.ppu.timing;
    bus.on_stop_entered();
    assert_eq!(bus.ppu.timing, timing);
    bus.advance_mcycle();
    assert_eq!(bus.ppu.timing, ticked(timing, 4));
}
