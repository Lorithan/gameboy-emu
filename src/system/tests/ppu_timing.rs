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
    if let CycleAction::Write {
        addr: 0xFF40,
        value,
    } = trace.record.action
    {
        reference.write(0xFF40, value);
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
