//! #80 OAM DMA at CPU level (docs/tasks/OAM_DMA.md). Written ahead of
//! implementation. Programs disable the LCD first so the PPU-mode OAM
//! locks cannot interfere, and WRAM sources are filled through the Bus.

use super::super::System;
use crate::{memory::Cartridge, step_support::expect_cycle};

const OAM: u16 = 0xFE00;

fn machine(program: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + program.len()].copy_from_slice(program);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn wram_byte(addr: u16) -> u8 {
    (addr as u8).wrapping_mul(11) ^ (addr >> 8) as u8 ^ 0xC3
}

fn fill_wram(system: &mut System) {
    for addr in 0xC000..=0xDFFF {
        system.memory.write(addr, wram_byte(addr));
    }
}

/// Step whole instructions until PC reaches `pc`.
fn run_to(system: &mut System, pc: u16) {
    for _ in 0..10_000 {
        if system.cpu_snapshot().pc == pc {
            return;
        }
        let _ = system.step_instruction(100).unwrap();
    }
    panic!("PC {pc:04X} never reached");
}

/// oam_dma_timing, run from ROM (allowed: no bus conflicts). Each round
/// starts a transfer and spins `DEC B / JR NZ` 40 times; round 1 then reads
/// OAM at M161 (blocked, FF), round 2 adds a NOP and reads at M162 (data).
const TIMING: [u8; 35] = [
    0xAF, // XOR A
    0xE0, 0x40, // LDH (40),A: LCD off
    0x3E, 0x01, // LD A,01
    0xEA, 0x00, 0xC0, // LD (C000),A
    0x21, 0x00, 0xFE, // LD HL,FE00
    0x06, 0x28, // LD B,40
    0x3E, 0xC0, // LD A,C0
    0xE0, 0x46, // LDH (46),A
    0x05, // DEC B
    0x20, 0xFD, // JR NZ,-3
    0x7E, // LD A,(HL)
    0x4F, // LD C,A
    0x06, 0x28, // LD B,40
    0x3E, 0xC0, // LD A,C0
    0xE0, 0x46, // LDH (46),A
    0x05, // DEC B
    0x20, 0xFD, // JR NZ,-3
    0x00, // NOP
    0x7E, // LD A,(HL)
    0x57, // LD D,A
    0x18, // JR -2 (offset below)
];
const TIMING_END: u16 = 0x0122;

fn timing_machine() -> System {
    let mut program = TIMING.to_vec();
    program.push(0xFE);
    machine(&program)
}

#[test]
fn oam_dma_timing_reproduction_reads_ff_at_m161_and_data_at_m162() {
    let mut system = timing_machine();
    run_to(&mut system, TIMING_END);
    let snapshot = system.cpu_snapshot();
    assert_eq!(
        snapshot.bc & 0xFF,
        0xFF,
        "C: read one M-cycle before the end"
    );
    assert_eq!(snapshot.de >> 8, 0x01, "D: read exactly at the end");
    assert_eq!(system.memory.read(0xFF46), 0xC0);
}

#[test]
fn instruction_budget_pauses_do_not_change_the_transfer() {
    for budget in [1, 2, 3, 5] {
        let mut paused = timing_machine();
        let mut stepped = timing_machine();
        while paused.cpu_snapshot().pc != TIMING_END {
            let _ = paused.step_instruction(budget).unwrap();
            while stepped.elapsed_ticks() < paused.elapsed_ticks() {
                let _ = expect_cycle(stepped.step_machine_cycle().unwrap());
            }
            assert_eq!(
                paused.cpu_snapshot(),
                stepped.cpu_snapshot(),
                "budget {budget}"
            );
            for addr in [OAM, OAM + 0x9F, 0xFF46] {
                assert_eq!(
                    paused.memory.read(addr),
                    stepped.memory.read(addr),
                    "budget {budget}, {addr:04X}"
                );
            }
        }
        assert_eq!(paused.cpu_snapshot().bc & 0xFF, 0xFF, "budget {budget}");
    }
}

#[test]
fn a_transfer_continues_while_the_cpu_is_halted() {
    // Rule 8. IE = 0 and IME = 0, so HALT never wakes.
    const HALT_AT: u16 = 0x0109;
    let mut system = machine(&[
        0xAF, // XOR A
        0xE0, 0x40, // LDH (40),A: LCD off
        0xE0, 0xFF, // LDH (FF),A: IE = 0
        0x3E, 0xD0, // LD A,D0
        0xE0, 0x46, // LDH (46),A
        0x76, // HALT
    ]);
    fill_wram(&mut system);
    run_to(&mut system, HALT_AT + 1);
    for _ in 0..170 {
        let _ = expect_cycle(system.step_machine_cycle().unwrap());
    }
    assert_eq!(system.cpu_snapshot().pc, HALT_AT + 1, "still halted");
    for index in 0..0xA0u16 {
        assert_eq!(
            system.memory.read(OAM + index),
            wram_byte(0xD000 + index),
            "OAM {index:02X}"
        );
    }
}

#[test]
fn an_hram_routine_like_mooneye_dma_proc_completes_the_copy() {
    // oam_dma/basic: the transfer is started and awaited from HRAM, then
    // the program returns to ROM and compares.
    const ROUTINE: [u8; 8] = [
        0xE0, 0x46, // LDH (46),A
        0x06, 0x28, // LD B,40
        0x05, // DEC B
        0x20, 0xFD, // JR NZ,-3
        0xC9, // RET
    ];
    const END: u16 = 0x010B;
    let mut system = machine(&[
        0x31, 0xFE, 0xFF, // LD SP,FFFE
        0xAF, // XOR A
        0xE0, 0x40, // LDH (40),A: LCD off
        0x3E, 0xC1, // LD A,C1
        0xCD, 0x80, 0xFF, // CALL FF80
        0x18, 0xFE, // JR -2
    ]);
    fill_wram(&mut system);
    for (offset, &byte) in ROUTINE.iter().enumerate() {
        system.memory.write(0xFF80 + offset as u16, byte);
    }
    run_to(&mut system, END);
    for index in 0..0xA0u16 {
        assert_eq!(
            system.memory.read(OAM + index),
            wram_byte(0xC100 + index),
            "OAM {index:02X}"
        );
    }
}
