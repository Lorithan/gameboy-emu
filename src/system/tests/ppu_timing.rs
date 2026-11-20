//! #74 CPU-level PPU progression. System tests cannot name private PPU types,
//! so they compare opaque timing probes against a bare reference Bus that is
//! advanced one M-cycle per executed cycle, with LCDC stores applied after
//! that cycle's four ticks (the agreed device-then-access order).

use super::super::{InstructionStep, MachineStep, System};
use crate::{
    cpu::{CycleAction, CycleOutcome},
    memory::{Bus, Button, Cartridge},
    step_support::expect_cycle,
};

fn rom(bytes: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    rom[0x50] = 0xD9; // Timer vector: RETI
    rom
}

fn machine(bytes: &[u8]) -> System {
    System::new(Cartridge::from_bytes(&rom(bytes)).unwrap())
}

fn reference_bus() -> Bus {
    Bus::new(Cartridge::from_bytes(&rom(&[])).unwrap())
}

/// Steps one machine cycle and mirrors it on `reference`.
fn step_mirrored(system: &mut System, reference: &mut Bus) -> CycleAction {
    let trace = expect_cycle(system.step_machine_cycle().unwrap());
    reference.advance_mcycle();
    match trace.record.action {
        // LCD register stores apply after the cycle's four ticks.
        CycleAction::Write { addr, value } if (0xFF40..=0xFF4B).contains(&addr) => {
            reference.write(addr, value);
        }
        // Live LY/STAT reads must return the reference's value at this cycle.
        CycleAction::Read {
            addr: addr @ (0xFF41 | 0xFF44),
        } => {
            assert_eq!(
                trace.record.read_value,
                Some(reference.read(addr)),
                "read {addr:04X} at tick {}",
                system.elapsed_ticks()
            );
        }
        _ => {}
    }
    assert_eq!(
        system.memory.ppu_timing_probe(),
        reference.ppu_timing_probe(),
        "tick {}",
        system.elapsed_ticks()
    );
    trace.record.action
}

// LD A,11; LDH (40),A (on->off); LD A,91; LDH (40),A (off->on);
// LD A,13; LDH (40),A (on->on); then NOP / JR -3 forever.
const LCDC_PROGRAM: [u8; 15] = [
    0x3E, 0x11, 0xE0, 0x40, 0x3E, 0x91, 0xE0, 0x40, 0x3E, 0x13, 0xE0, 0x40, 0x00, 0x18, 0xFD,
];

#[test]
fn cpu_lcdc_stores_take_effect_after_their_mcycle_ticks() {
    let mut system = machine(&LCDC_PROGRAM);
    let mut reference = reference_bus();
    let mut just_enabled = reference_bus();
    just_enabled.write(0xFF40, 0x11);
    just_enabled.write(0xFF40, 0x91);
    let mut writes = Vec::new();
    for _ in 0..2 * 17_556 {
        let action = step_mirrored(&mut system, &mut reference);
        if let CycleAction::Write {
            addr: 0xFF40,
            value,
        } = action
        {
            writes.push(value);
            if value == 0x91 {
                // The enabling store's own four ticks ran while still off:
                // no dots have elapsed since the restart.
                assert_eq!(
                    system.memory.ppu_timing_probe(),
                    just_enabled.ppu_timing_probe()
                );
            }
        }
    }
    assert_eq!(writes, [0x11, 0x91, 0x13]);
    assert_eq!(system.memory.read(0xFF40), 0x13);
}

#[test]
fn instruction_budget_pauses_do_not_change_ppu_progression() {
    for budget in [1, 2, 3, 5, 17] {
        let mut paused = machine(&LCDC_PROGRAM);
        let mut stepped = machine(&LCDC_PROGRAM);
        for _ in 0..400 {
            let _ = paused.step_instruction(budget).unwrap();
            while stepped.elapsed_ticks() < paused.elapsed_ticks() {
                let _ = expect_cycle(stepped.step_machine_cycle().unwrap());
            }
            assert_eq!(stepped.elapsed_ticks(), paused.elapsed_ticks());
            assert_eq!(
                paused.memory.ppu_timing_probe(),
                stepped.memory.ppu_timing_probe(),
                "budget {budget}, tick {}",
                paused.elapsed_ticks()
            );
        }
    }
}

#[test]
fn halt_idles_and_interrupt_entry_advance_ppu_every_mcycle() {
    // EI; HALT; NOP; JR -4 (back to HALT). Timer IRQs wake it repeatedly.
    let mut system = machine(&[0xFB, 0x76, 0x00, 0x18, 0xFC]);
    system.memory.write(0xFF0F, 0);
    system.memory.write(0xFFFF, 0x04);
    system.memory.write(0xFF05, 0xF0);
    system.memory.write(0xFF06, 0xF0);
    system.memory.write(0xFF07, 5);
    let mut reference = reference_bus();
    let (mut idles, mut entries) = (0, 0);
    for cycle in 1..=3 * 17_556u64 {
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        reference.advance_mcycle();
        idles += usize::from(trace.record.action == CycleAction::Idle);
        entries += usize::from(matches!(
            trace.outcome,
            CycleOutcome::InterruptEntryComplete { .. }
        ));
        assert_eq!(system.elapsed_ticks(), 4 * cycle);
        assert_eq!(
            system.memory.ppu_timing_probe(),
            reference.ppu_timing_probe(),
            "cycle {cycle}"
        );
    }
    // Prove the run actually exercised HALT idling and interrupt entry.
    assert!(idles > 10_000, "idles {idles}");
    assert!(entries > 500, "entries {entries}");
}

#[test]
fn stop_preserves_ppu_phase_and_stopped_polls_add_no_dots() {
    // NOP; STOP 00; NOP...
    let mut system = machine(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00]);
    system.memory.write(0xFF0F, 0);
    system.memory.write(0xFFFF, 0);
    let mut reference = reference_bus();
    let _ = step_mirrored(&mut system, &mut reference);
    let entry = expect_cycle(system.step_machine_cycle().unwrap());
    reference.advance_mcycle();
    assert_eq!(entry.outcome, CycleOutcome::StopEntered);
    let stopped = system.memory.ppu_timing_probe();
    assert_eq!(stopped, reference.ppu_timing_probe());
    for budget in [1, 16, 1000] {
        assert!(matches!(
            system.step_machine_cycle().unwrap(),
            MachineStep::Stopped
        ));
        assert!(matches!(
            system.step_instruction(budget).unwrap(),
            InstructionStep::Stopped { ticks: 0, .. }
        ));
        assert_eq!(system.memory.ppu_timing_probe(), stopped);
    }
    system.set_button(Button::A, true);
    system.set_button(Button::A, false);
    // Resumes from the preserved phase, not a reset one.
    for _ in 0..3 {
        let _ = step_mirrored(&mut system, &mut reference);
    }
}

#[test]
fn cpu_ldh_reads_of_ly_and_stat_return_the_value_at_their_exact_mcycle() {
    // LD A,78; LDH (41),A (all selects); LD A,02; LDH (45),A (LYC 2);
    // loop: LDH A,(44); LDH A,(41); JR -6.
    let mut system = machine(&[
        0x3E, 0x78, 0xE0, 0x41, 0x3E, 0x02, 0xE0, 0x45, 0xF0, 0x44, 0xF0, 0x41, 0x18, 0xFA,
    ]);
    system.memory.write(0xFF0F, 0);
    let mut reference = reference_bus();
    let mut seen_ly = [false; 154];
    let (mut flag_reads, mut stat_reads) = (0, 0);
    for _ in 0..3 * 17_556 {
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        // Re-run the cycle's mirror bookkeeping on the reference by hand,
        // since the trace is consumed here for value coverage.
        reference.advance_mcycle();
        match trace.record.action {
            CycleAction::Write { addr, value } => reference.write(addr, value),
            CycleAction::Read { addr: 0xFF44 } => {
                let ly = trace.record.read_value.unwrap();
                assert_eq!(ly, reference.read(0xFF44));
                seen_ly[usize::from(ly)] = true;
            }
            CycleAction::Read { addr: 0xFF41 } => {
                let stat = trace.record.read_value.unwrap();
                assert_eq!(stat, reference.read(0xFF41));
                assert_eq!(stat & 0xF8, 0xF8, "bit 7 and selects");
                stat_reads += 1;
                flag_reads += usize::from(stat & 0x04 != 0);
            }
            _ => {}
        }
        assert_eq!(system.memory.read(0xFF0F), 0xE0, "no PPU interrupts");
    }
    // LY 153 is visible at one 4-dot sample per frame; this 9-M-cycle poll
    // loop reaches only 3 of 9 phases, so the Bus-level status tests own that
    // case. Every other LY must be read.
    assert!(
        seen_ly[..153].iter().all(|&seen| seen),
        "every LY 0-152 observed"
    );
    assert!(flag_reads > 0 && flag_reads < stat_reads);
}

#[test]
fn budget_pauses_do_not_change_ly_or_stat_reads() {
    let program = [0xF0, 0x44, 0x47, 0xF0, 0x41, 0x4F, 0x18, 0xF8]; // B=LY, C=STAT
    for budget in [1, 2, 3, 7] {
        let mut paused = machine(&program);
        let mut stepped = machine(&program);
        for _ in 0..3_000 {
            let _ = paused.step_instruction(budget).unwrap();
            while stepped.elapsed_ticks() < paused.elapsed_ticks() {
                let _ = expect_cycle(stepped.step_machine_cycle().unwrap());
            }
            assert_eq!(
                paused.cpu_snapshot(),
                stepped.cpu_snapshot(),
                "budget {budget}"
            );
            assert_eq!(paused.memory.read(0xFF41), stepped.memory.read(0xFF41));
        }
    }
}
