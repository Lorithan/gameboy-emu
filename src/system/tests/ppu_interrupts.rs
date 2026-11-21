//! #76 slice B at CPU level. PPU requests are checked against control
//! programs whose request source (Timer, or a direct IF store) already has
//! verified M3 entry timing, so these tests pin "a PPU request behaves like
//! any other request raised at the same point", not new CPU timing.

use super::super::System;
use crate::{cpu::CycleOutcome, memory::Cartridge, step_support::expect_cycle};

/// Ticks from the post-boot seed to physical line 144 / dot 0, and to the
/// first HBlank (line 0 / dot 252), as Bus M-cycles.
const SEED_TO_VBLANK_MCYCLES: u64 = (452 + 144 * 456) / 4;
const SEED_TO_FIRST_HBLANK_MCYCLES: u64 = (452 + 252) / 4;

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    for vector in [0x40, 0x48, 0x50] {
        rom[vector] = 0xD9; // RETI
    }
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

/// EI; HALT; JR -3 (back to HALT).
const HALT_LOOP: [u8; 4] = [0xFB, 0x76, 0x18, 0xFD];

/// Steps until IF & `mask` is observed set (returning that step, counted
/// from 1), then until interrupt entry completes; returns
/// (request step, steps from request to entry completion, vector).
fn wake(system: &mut System, mask: u8) -> (u64, u64, u16) {
    let mut step = 0;
    let requested = loop {
        step += 1;
        assert!(step < 3 * 17_556, "no request");
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        assert!(
            !matches!(trace.outcome, CycleOutcome::InterruptEntryComplete { .. }),
            "entry before request"
        );
        if system.memory.read(0xFF0F) & mask != 0 {
            break step;
        }
    };
    loop {
        step += 1;
        assert!(step < requested + 20, "no entry");
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        if let CycleOutcome::InterruptEntryComplete { vector } = trace.outcome {
            return (requested, step - requested, vector);
        }
    }
}

fn halted_machine(enable: u8, stat: u8) -> System {
    let mut system = machine(&HALT_LOOP);
    system.memory.write(0xFF41, stat); // May request via the quirk.
    system.memory.write(0xFF0F, 0);
    system.memory.write(0xFFFF, enable);
    system
}

#[test]
fn ppu_requests_wake_halt_and_dispatch_like_a_timer_request() {
    // Control: Timer from HALT (TIMA FE, TAC 05: 16-tick increments).
    let mut timer = halted_machine(0x04, 0);
    timer.memory.write(0xFF05, 0xFE);
    timer.memory.write(0xFF07, 0x05);
    let (_, timer_delay, vector) = wake(&mut timer, 0x04);
    assert_eq!(vector, 0x50);

    let mut vblank = halted_machine(0x01, 0);
    assert_eq!(
        wake(&mut vblank, 0x01),
        (SEED_TO_VBLANK_MCYCLES, timer_delay, 0x40),
        "VBlank"
    );

    let mut hblank = halted_machine(0x02, 0x08);
    assert_eq!(
        wake(&mut hblank, 0x02),
        (SEED_TO_FIRST_HBLANK_MCYCLES, timer_delay, 0x48),
        "STAT HBlank"
    );

    // Same-tick VBlank and line-144 OAM pulse: VBlank (higher priority)
    // dispatches first, STAT stays pending for the next entry.
    let mut both = halted_machine(0x03, 0x20);
    let mut entries = Vec::new();
    for _ in 0..SEED_TO_VBLANK_MCYCLES + 40 {
        let trace = expect_cycle(both.step_machine_cycle().unwrap());
        if let CycleOutcome::InterruptEntryComplete { vector } = trace.outcome {
            entries.push(vector);
        }
    }
    // Lines 1-143 OAM each request STAT before VBlank; line 0 follows the
    // seed's VBlank with only the OAM select, so it requests too.
    assert_eq!(entries.len(), 144 + 2, "{entries:X?}");
    assert_eq!(&entries[144..], [0x40, 0x48]);
}

#[test]
fn with_ime_off_ppu_requests_only_set_if() {
    // DI; loop: NOP; JR -3.
    let mut system = machine(&[0xF3, 0x00, 0x18, 0xFD]);
    system.memory.write(0xFF41, 0x08);
    system.memory.write(0xFF0F, 0);
    system.memory.write(0xFFFF, 0x03);
    for _ in 0..17_556 {
        let trace = expect_cycle(system.step_machine_cycle().unwrap());
        assert!(!matches!(
            trace.outcome,
            CycleOutcome::InterruptEntryComplete { .. }
        ));
        assert!((0x101..=0x104).contains(&system.cpu_snapshot().pc));
    }
    assert_eq!(system.memory.read(0xFF0F), 0xE3);
}

/// Runs `program` with IE = STAT and returns the steps (from 1) of every
/// completed interrupt entry within 40 M-cycles.
fn entries(program: &[u8]) -> Vec<(u64, u16)> {
    let mut system = machine(program);
    system.memory.write(0xFF0F, 0);
    system.memory.write(0xFFFF, 0x02);
    (1..=40)
        .filter_map(|step| {
            let trace = expect_cycle(system.step_machine_cycle().unwrap());
            match trace.outcome {
                CycleOutcome::InterruptEntryComplete { vector } => Some((step, vector)),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn write_raised_stat_requests_are_sampled_like_a_direct_if_store() {
    // From the seed (VBlank, selects 0): EI; LD A,10; LDH (41),A; NOPs.
    // The STAT write requests via the VBlank source / quirk after its access.
    let stat_write = entries(&[0xFB, 0x3E, 0x10, 0xE0, 0x41, 0, 0, 0, 0, 0, 0, 0, 0]);
    // Control: EI; LD A,02; LDH (0F),A; NOPs, requesting at the same cycle.
    let if_store = entries(&[0xFB, 0x3E, 0x02, 0xE0, 0x0F, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(stat_write.len(), 1, "{stat_write:?}");
    assert_eq!(stat_write, if_store);
    assert_eq!(stat_write[0].1, 0x48);
}
