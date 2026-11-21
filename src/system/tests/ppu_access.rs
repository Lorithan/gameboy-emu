//! #77 slice C at CPU level (locks refined by #73 D3). Every VRAM/OAM access
//! made by a running program is checked against independent position-based
//! locks and a shadow of the expected contents, and against a lockstep
//! reference Bus. Accesses see the position after their cycle's four ticks.

use super::super::System;
use std::collections::HashMap;

use crate::{
    cpu::CycleAction,
    memory::{Bus, Cartridge},
    step_support::expect_cycle,
};

// LD SP,FE40; LD HL,9F00;
// loop: INC A; LD (HL),A; LD A,(HL); LD (FE10),A; LD A,(FE10);
//       PUSH BC; POP DE; INC C; JR loop.
const PROGRAM: [u8; 20] = [
    0x31, 0x40, 0xFE, 0x21, 0x00, 0x9F, 0x3C, 0x77, 0x7E, 0xEA, 0x10, 0xFE, 0xFA, 0x10, 0xFE, 0xC5,
    0xD1, 0x0C, 0x18, 0xF2,
];

fn rom() -> Vec<u8> {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + PROGRAM.len()].copy_from_slice(&PROGRAM);
    rom
}

fn machine() -> System {
    System::new(Cartridge::from_bytes(&rom()).unwrap())
}

/// Physical (line, dot) after `ticks` base ticks from the post-boot seed
/// (line 153, dot 4). The program never writes LCDC, so the LCD stays on and
/// no line is a first-after-enable line.
fn position(ticks: u64) -> (u64, u64) {
    let frame_dot = (153 * 456 + 4 + ticks) % 70_224;
    (frame_dot / 456, frame_dot % 456)
}

/// Independent CPU access locks (#77, refined by #73 D3) for (read, write).
fn blocked(addr: u16, ticks: u64) -> (bool, bool) {
    let (line, dot) = position(ticks);
    let visible = line < 144;
    let drawing = visible && (80..252).contains(&dot);
    match addr {
        0x8000..=0x9FFF => (visible && (76..252).contains(&dot), drawing),
        0xFE00..=0xFE9F => (
            (visible && dot < 252) || (line <= 142 && dot >= 452),
            (visible && dot < 76) || drawing,
        ),
        _ => (false, false),
    }
}

#[test]
fn cpu_vram_and_oam_accesses_follow_the_locks_at_their_mcycle() {
    let mut system = machine();
    let mut reference = Bus::new(Cartridge::from_bytes(&rom()).unwrap());
    // [vram, oam] x [read, write] x [allowed, blocked]
    let mut counts = [[[0u32; 2]; 2]; 2];
    // Expected VRAM/OAM contents (zero at startup), updated only by stores
    // the independent locks allow.
    let mut shadow = HashMap::<u16, u8>::new();
    for _ in 0..3 * 17_556 {
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        reference.advance_mcycle();
        let tick = system.elapsed_ticks();
        match trace.record.action {
            CycleAction::Read { addr } if region(addr).is_some() => {
                let (read_lock, _) = blocked(addr, tick);
                let value = trace.record.read_value.unwrap();
                let expected = if read_lock {
                    0xFF
                } else {
                    shadow.get(&addr).copied().unwrap_or(0)
                };
                assert_eq!(value, expected, "read {addr:04X}, tick {tick}");
                assert_eq!(
                    value,
                    reference.read(addr),
                    "reference {addr:04X}, tick {tick}"
                );
                counts[region(addr).unwrap()][0][usize::from(read_lock)] += 1;
            }
            CycleAction::Write { addr, value } => {
                reference.write(addr, value);
                if let Some(region) = region(addr) {
                    let (read_lock, write_lock) = blocked(addr, tick);
                    counts[region][1][usize::from(write_lock)] += 1;
                    if !write_lock {
                        shadow.insert(addr, value);
                    }
                    let expected = if read_lock {
                        0xFF
                    } else {
                        shadow.get(&addr).copied().unwrap_or(0)
                    };
                    assert_eq!(
                        system.memory.read(addr),
                        expected,
                        "store {addr:04X}, tick {tick}"
                    );
                }
            }
            _ => {}
        }
        for addr in [0x9F00, 0xFE10, 0xFE3E, 0xFE3F] {
            assert_eq!(
                system.memory.read(addr),
                reference.read(addr),
                "{addr:04X}, tick {tick}"
            );
        }
    }
    for (region, name) in [(0, "VRAM"), (1, "OAM")] {
        for (kind, access) in [(0, "reads"), (1, "writes")] {
            let [allowed, blocked] = counts[region][kind];
            assert!(
                allowed > 0 && blocked > 0,
                "{name} {access}: {allowed} allowed, {blocked} blocked"
            );
        }
    }
    // Hidden storage: blocked stores never landed, allowed ones did.
    system.memory.write(0xFF40, 0x11);
    reference.write(0xFF40, 0x11);
    for addr in (0x8000u16..=0x9FFF).chain(0xFE00..=0xFE9F) {
        let expected = shadow.get(&addr).copied().unwrap_or(0);
        assert_eq!(system.memory.read(addr), expected, "{addr:04X}");
        assert_eq!(
            system.memory.read(addr),
            reference.read(addr),
            "reference {addr:04X}"
        );
    }
}

fn region(addr: u16) -> Option<usize> {
    match addr {
        0x8000..=0x9FFF => Some(0),
        0xFE00..=0xFE9F => Some(1),
        _ => None,
    }
}

#[test]
fn instruction_budget_pauses_do_not_change_blocked_accesses() {
    for budget in [1, 2, 3, 5] {
        let mut paused = machine();
        let mut stepped = machine();
        for _ in 0..6_000 {
            let _ = paused.step_instruction(budget).unwrap();
            while stepped.elapsed_ticks() < paused.elapsed_ticks() {
                let _ = expect_cycle(stepped.step_machine_cycle().unwrap());
            }
            assert_eq!(
                paused.cpu_snapshot(),
                stepped.cpu_snapshot(),
                "budget {budget}"
            );
        }
        paused.memory.write(0xFF40, 0x11);
        stepped.memory.write(0xFF40, 0x11);
        for addr in (0x8000u16..=0x9FFF).chain(0xFE00..=0xFE9F) {
            assert_eq!(
                paused.memory.read(addr),
                stepped.memory.read(addr),
                "budget {budget}"
            );
        }
    }
}
