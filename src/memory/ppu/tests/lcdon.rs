//! #73 D3: the DMG-verified expectation tables of Mooneye lcdon_timing-GS
//! and lcdon_write_timing-GS (revision 31510e12, blobs 3bcf2b2c and
//! 4ba40c7a; MIT), reproduced through Bus.
//!
//! In both ROMs each sample is `ldh (LCDC),a` (enable, $81), then N NOPs,
//! then a 2-M-cycle `ld a,(de)` or `ld (de),a`. Our enable store applies
//! after its own M-cycle's ticks, and the access happens after the access
//! cycle's ticks, so the access follows N + 2 Bus M-cycles. That is
//! dot 4N + 8 of the enabled timing, always a multiple of 4.

use super::fresh_bus;
use crate::memory::Bus;

/// Reads: pass 1, 2 and 3 use N + 0, N + 1 and N + 2.
const READ_NOPS: [u32; 8] = [0, 17, 60, 110, 130, 174, 224, 244];
const EXPECT_LY: [[u8; 8]; 3] = [
    [0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x02],
    [0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x02],
    [0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x02],
];
const EXPECT_STAT_LYC0: [[u8; 8]; 3] = [
    [0x84, 0x84, 0x87, 0x84, 0x82, 0x83, 0x80, 0x82],
    [0x84, 0x87, 0x84, 0x80, 0x82, 0x80, 0x80, 0x82],
    [0x84, 0x87, 0x84, 0x82, 0x83, 0x80, 0x82, 0x83],
];
const EXPECT_STAT_LYC1: [[u8; 8]; 3] = [
    [0x80, 0x80, 0x83, 0x80, 0x86, 0x87, 0x84, 0x82],
    [0x80, 0x83, 0x80, 0x80, 0x86, 0x84, 0x80, 0x82],
    [0x80, 0x83, 0x80, 0x86, 0x87, 0x84, 0x82, 0x83],
];
const EXPECT_OAM_READ: [[u8; 8]; 3] = [
    [0x00, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF],
    [0x00, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF],
    [0x00, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF],
];
const EXPECT_VRAM_READ: [[u8; 8]; 3] = [
    [0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00],
    [0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF],
    [0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF],
];

const WRITE_NOPS: [u32; 19] = [
    0, 17, 18, 60, 61, 110, 111, 112, 130, 131, 132, 174, 175, 224, 225, 226, 244, 245, 246,
];
/// True where the $81 store lands (the ROM reads back $81, else $00).
const EXPECT_OAM_WRITE: [bool; 19] = [
    true, true, false, false, true, true, true, false, false, true, false, false, true, true, true,
    false, false, true, false,
];
const EXPECT_VRAM_WRITE: [bool; 19] = [
    true, true, false, false, true, true, true, true, true, true, false, false, true, true, true,
    true, true, true, false,
];

/// Bus with the LCD freshly enabled ($81) after a disable, VRAM/OAM cleared.
fn enabled_bus(lyc: u8) -> Bus {
    let mut bus = fresh_bus();
    bus.write(0xFF40, 0x00);
    bus.write(0x8000, 0x00);
    bus.write(0xFE00, 0x00);
    bus.write(0xFF45, lyc);
    bus.write(0xFF40, 0x81);
    bus
}

fn read_after(nops: u32, addr: u16, lyc: u8) -> u8 {
    let mut bus = enabled_bus(lyc);
    for _ in 0..nops + 2 {
        bus.advance_mcycle();
    }
    bus.read(addr)
}

fn write_lands(nops: u32, addr: u16) -> bool {
    let mut bus = enabled_bus(0);
    for _ in 0..nops + 2 {
        bus.advance_mcycle();
    }
    bus.write(addr, 0x81);
    bus.write(0xFF40, 0x00); // Read the stored byte back with the LCD off.
    bus.read(addr) == 0x81
}

fn describe(nops: u32) -> String {
    let dot = 4 * nops + 8;
    format!("N={nops} (line {} dot {})", dot / 456, dot % 456)
}

#[test]
fn lcdon_timing_gs_read_table_matches() {
    let mut mismatches = Vec::new();
    for pass in 0..3u32 {
        for (column, &base) in READ_NOPS.iter().enumerate() {
            let nops = base + pass;
            let pass = pass as usize;
            for (name, addr, lyc, expected) in [
                ("LY", 0xFF44, 0, EXPECT_LY[pass][column]),
                ("STAT LYC=0", 0xFF41, 0, EXPECT_STAT_LYC0[pass][column]),
                ("STAT LYC=1", 0xFF41, 1, EXPECT_STAT_LYC1[pass][column]),
                ("OAM", 0xFE00, 0, EXPECT_OAM_READ[pass][column]),
                ("VRAM", 0x8000, 0, EXPECT_VRAM_READ[pass][column]),
            ] {
                let actual = read_after(nops, addr, lyc);
                if actual != expected {
                    mismatches.push(format!(
                        "{name} {}: expected {expected:02X}, got {actual:02X}",
                        describe(nops)
                    ));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn lcdon_write_timing_gs_write_table_matches() {
    let mut mismatches = Vec::new();
    for (index, &nops) in WRITE_NOPS.iter().enumerate() {
        for (name, addr, expected) in [
            ("OAM", 0xFE00, EXPECT_OAM_WRITE[index]),
            ("VRAM", 0x8000, EXPECT_VRAM_WRITE[index]),
        ] {
            let actual = write_lands(nops, addr);
            if actual != expected {
                mismatches.push(format!(
                    "{name} write {}: expected {}, got {}",
                    describe(nops),
                    if expected { "lands" } else { "ignored" },
                    if actual { "lands" } else { "ignored" }
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
