//! #80 OAM DMA (FF46), black-box through Bus, against the contract in
//! docs/tasks/OAM_DMA.md. Written ahead of implementation.
//!
//! Each test treats `bus.write(0xFF46, ..)` as the CPU access of M0. Before
//! the CPU access of Mk the test calls `advance_mcycle` k times, as System
//! does. The expected values come from the independent models below, never
//! from production routing.

use super::{Bus, Cartridge};

const OAM: u16 = 0xFE00;
const OAM_LEN: u16 = 0xA0;

/// ROM byte at `addr`. The header (0100-014F) stays zero apart from the
/// fixed checksum E7 at 014D, which is valid for an all-zero header.
fn rom_byte(addr: u16) -> u8 {
    if addr == 0x014D {
        0xE7
    } else if (0x0100..0x0150).contains(&addr) {
        0
    } else {
        (addr as u8).wrapping_mul(13) ^ (addr >> 8) as u8 ^ 0x5A
    }
}

fn vram_byte(addr: u16) -> u8 {
    (addr as u8).wrapping_mul(7) ^ (addr >> 8) as u8 ^ 0x3C
}

fn wram_byte(addr: u16) -> u8 {
    (addr as u8).wrapping_mul(11) ^ (addr >> 8) as u8 ^ 0xC3
}

/// Byte `index` of a transfer from `page` (contract rule 4).
fn source_byte(page: u8, index: u16) -> u8 {
    let addr = (u16::from(page) << 8) + index;
    match page {
        0x00..=0x7F => rom_byte(addr),
        0x80..=0x9F => vram_byte(addr),
        0xA0..=0xBF => 0xFF, // ROM-only cartridge: no external RAM.
        0xC0..=0xDF => wram_byte(addr),
        0xE0..=0xFF => wram_byte(addr - 0x2000),
    }
}

/// Bus with patterned ROM, VRAM and WRAM, OAM filled with `oam_fill`,
/// and the LCD off so the PPU-mode locks cannot interfere.
fn bus_lcd_off(oam_fill: u8) -> Bus {
    let mut rom = vec![0; 32_768];
    for (addr, byte) in rom.iter_mut().enumerate() {
        *byte = rom_byte(addr as u16);
    }
    rom[0x014D] = 0xE7;
    let mut bus = Bus::new(Cartridge::from_bytes(&rom).unwrap());
    bus.write(0xFF40, 0x11);
    for addr in 0x8000..=0x9FFF {
        bus.write(addr, vram_byte(addr));
    }
    for addr in 0xC000..=0xDFFF {
        bus.write(addr, wram_byte(addr));
    }
    for addr in OAM..OAM + OAM_LEN {
        bus.write(addr, oam_fill);
    }
    bus
}

fn advance(bus: &mut Bus, mcycles: u32) {
    for _ in 0..mcycles {
        bus.advance_mcycle();
    }
}

fn oam(bus: &Bus) -> Vec<u8> {
    (OAM..OAM + OAM_LEN).map(|addr| bus.read(addr)).collect()
}

fn page(page: u8) -> Vec<u8> {
    (0..OAM_LEN).map(|index| source_byte(page, index)).collect()
}

/// Rule 2: a transfer written at M0 blocks CPU OAM access at M2..=M161.
fn blocked_single(k: u32) -> bool {
    (2..=161).contains(&k)
}

/// Rules 2-3 for a first write at M0 and a second at M`second`: the first
/// transfer runs until the second starts at M(second+2); a second write
/// before the first has started replaces it.
fn blocked_restart(second: u32, k: u32) -> bool {
    let first = if second >= 2 {
        (2..=161.min(second + 1)).contains(&k)
    } else {
        false
    };
    first || (second + 2..=second + 161).contains(&k)
}

#[test]
fn ff46_starts_at_ff_and_every_write_reads_back_at_every_phase() {
    // Rule 1 (decision 2: startup value FF, provisional).
    let bus = bus_lcd_off(0);
    assert_eq!(bus.read(0xFF46), 0xFF);
    for value in 0u8..=u8::MAX {
        let mut bus = bus_lcd_off(0);
        bus.write(0xFF46, value);
        for k in 0..=163 {
            if k > 0 {
                bus.advance_mcycle();
            }
            assert_eq!(bus.read(0xFF46), value, "value {value:02X}, M{k}");
        }
    }
}

#[test]
fn cpu_oam_is_blocked_exactly_from_m2_to_m161_and_then_holds_the_source() {
    // oam_dma_start (M1 accessible, M2 blocked) and oam_dma_timing (M161
    // blocked, M162 readable), at every M-cycle and every OAM address.
    let mut bus = bus_lcd_off(0x5A);
    bus.write(0xFF46, 0xC0);
    for k in 0..=170 {
        if k > 0 {
            bus.advance_mcycle();
        }
        let expected = if blocked_single(k) {
            vec![0xFF; usize::from(OAM_LEN)]
        } else if k < 2 {
            vec![0x5A; usize::from(OAM_LEN)]
        } else {
            page(0xC0)
        };
        assert_eq!(oam(&bus), expected, "M{k}");
    }
}

#[test]
fn every_source_page_copies_all_160_bytes() {
    // Rule 4 and sources-GS: ROM, VRAM, absent external RAM, WRAM and the
    // E0-FF mapping to WRAM minus 0x2000 (FE -> DE00, FF -> DF00).
    for source in 0u8..=u8::MAX {
        let mut bus = bus_lcd_off(0x5A);
        bus.write(0xFF46, source);
        advance(&mut bus, 162);
        assert_eq!(oam(&bus), page(source), "page {source:02X}");
    }
}

#[test]
fn a_restart_while_active_keeps_blocking_until_the_new_transfer_ends() {
    // Rule 3 and oam_dma_start round 2 / oam_dma_restart: the old transfer
    // runs during the new write's M0 and M1; the new one ends 161 M-cycles
    // after its write. A restart at M161 leaves a one-cycle gap at M162.
    for second in [2u32, 3, 4, 7, 8, 80, 159, 160, 161] {
        let mut bus = bus_lcd_off(0x5A);
        bus.write(0xFF46, 0xC0);
        for k in 1..=second + 170 {
            bus.advance_mcycle();
            if k == second {
                bus.write(0xFF46, 0x80);
            }
            let read = bus.read(OAM + 0x9F);
            assert_eq!(
                read == 0xFF,
                blocked_restart(second, k),
                "second write at M{second}, M{k}: read {read:02X}"
            );
        }
        assert_eq!(oam(&bus), page(0x80), "second write at M{second}");
        assert_eq!(bus.read(0xFF46), 0x80);
    }
}

#[test]
fn a_second_write_before_the_start_replaces_the_pending_transfer() {
    // Rule 3, provisional: no pinned ROM writes twice within two M-cycles.
    let mut bus = bus_lcd_off(0x5A);
    bus.write(0xFF46, 0xC0);
    for k in 1..=170 {
        bus.advance_mcycle();
        if k == 1 {
            bus.write(0xFF46, 0x80);
        }
        let read = bus.read(OAM);
        assert_eq!(read == 0xFF, blocked_restart(1, k), "M{k}: read {read:02X}");
    }
    assert_eq!(oam(&bus), page(0x80));
}

#[test]
fn cpu_oam_writes_are_ignored_while_active_and_land_afterwards() {
    // Rule 6. Writes before M2 land but are then overwritten by the copy.
    for addr in [OAM, OAM + 0x50, OAM + 0x9F] {
        for k in 0..=164u32 {
            let mut bus = bus_lcd_off(0x5A);
            bus.write(0xFF46, 0xC0);
            advance(&mut bus, k);
            bus.write(addr, 0xA7);
            advance(&mut bus, 170);
            let mut expected = page(0xC0);
            if k >= 162 {
                expected[usize::from(addr - OAM)] = 0xA7;
            }
            assert_eq!(oam(&bus), expected, "write {addr:04X} at M{k}");
        }
    }
}

#[test]
fn the_copy_ignores_ppu_mode_locks_with_the_lcd_on() {
    // Rules 4-5: VRAM sources read and OAM written during OamScan and
    // Drawing. Start offsets cover a whole line (114 M-cycles) after enable.
    for start in (0..114).step_by(5) {
        let mut bus = bus_lcd_off(0x5A);
        bus.write(0xFF40, 0x91);
        advance(&mut bus, start);
        bus.write(0xFF46, 0x80);
        advance(&mut bus, 162);
        bus.write(0xFF40, 0x11);
        assert_eq!(oam(&bus), page(0x80), "start M{start} after enable");
    }
}

#[test]
fn no_bus_conflicts_and_no_other_state_changes_while_active() {
    // Rules 7-8 (decision 1: no bus conflicts). With the LCD on and off,
    // every address except OAM and FF46 reads as on a reference Bus that
    // never started a transfer, at every M-cycle, while the CPU also writes
    // WRAM, VRAM and HRAM. FEA0-FEFF keeps its fallback.
    for lcdc in [0x11u8, 0x91] {
        let mut bus = bus_lcd_off(0x5A);
        let mut reference = bus_lcd_off(0x5A);
        bus.write(0xFF40, lcdc);
        reference.write(0xFF40, lcdc);
        bus.write(0xFF46, 0xC0);
        for k in 1..=170u32 {
            bus.advance_mcycle();
            reference.advance_mcycle();
            let (addr, value) = match k % 3 {
                0 => (0xC000 + k as u16, k as u8),
                1 => (0x8000 + k as u16, !(k as u8)),
                _ => (0xFF80 + (k as u16 % 0x7F), k as u8 ^ 0x55),
            };
            bus.write(addr, value);
            reference.write(addr, value);
            for probe in 0u16..=u16::MAX {
                if (OAM..OAM + OAM_LEN).contains(&probe) || probe == 0xFF46 {
                    continue;
                }
                assert_eq!(
                    bus.read(probe),
                    reference.read(probe),
                    "LCDC {lcdc:02X}, M{k}, {probe:04X}"
                );
            }
            assert_eq!(bus.pending_interrupts(), reference.pending_interrupts());
        }
        // LY/STAT are compared through FF44/FF41 above.
        assert_eq!(bus.timer, reference.timer, "LCDC {lcdc:02X}");
    }
}
