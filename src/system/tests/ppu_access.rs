//! #77 slice C at CPU level. Every VRAM/OAM access made by a running program
//! is checked against STAT's mode at that M-cycle on a reference Bus advanced
//! in lockstep (stores applied after the cycle's ticks), so blocking uses the
//! mode after the access cycle's four ticks.

use super::super::System;
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

fn blocked(addr: u16, mode: u8) -> bool {
    match addr {
        0x8000..=0x9FFF => mode == 3,
        0xFE00..=0xFE9F => mode >= 2,
        _ => false,
    }
}

#[test]
fn cpu_vram_and_oam_accesses_follow_the_mode_at_their_mcycle() {
    let mut system = machine();
    let mut reference = Bus::new(Cartridge::from_bytes(&rom()).unwrap());
    // [vram, oam] x [read, write] x [allowed, blocked]
    let mut counts = [[[0u32; 2]; 2]; 2];
    for _ in 0..3 * 17_556 {
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        reference.advance_mcycle();
        let mode = reference.read(0xFF41) & 0x03;
        let tick = system.elapsed_ticks();
        match trace.record.action {
            CycleAction::Read { addr } if region(addr).is_some() => {
                let value = trace.record.read_value.unwrap();
                assert_eq!(value, reference.read(addr), "read {addr:04X}, tick {tick}");
                if blocked(addr, mode) {
                    assert_eq!(value, 0xFF, "blocked read {addr:04X}, tick {tick}");
                }
                if let Some(region) = region(addr) {
                    counts[region][0][usize::from(blocked(addr, mode))] += 1;
                }
            }
            CycleAction::Write { addr, value } => {
                let was_blocked = blocked(addr, mode);
                reference.write(addr, value);
                if let Some(region) = region(addr) {
                    counts[region][1][usize::from(was_blocked)] += 1;
                    let expected = if was_blocked { 0xFF } else { value };
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
    // Hidden storage agrees too: blocked stores never landed in either.
    system.memory.write(0xFF40, 0x11);
    reference.write(0xFF40, 0x11);
    for addr in (0x8000u16..=0x9FFF).chain(0xFE00..=0xFE9F) {
        assert_eq!(system.memory.read(addr), reference.read(addr), "{addr:04X}");
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
