use super::super::{InstructionStep, System};
use super::divider_oracle::clocked_byte;
use crate::step_support::expect_cycle;
use crate::{
    cpu::{CycleAction, CycleOutcome},
    memory::Cartridge,
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    for vector in [0x40, 0x48, 0x50] {
        rom[vector..vector + 3].copy_from_slice(&[0x06, 0x42, 0xD9]);
    }
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn pending(s: &mut System, mask: u8) {
    s.memory.write(0xFF0F, mask);
    s.memory.write(0xFFFF, mask);
}

#[test]
fn halt_completes_once_then_bounded_idles_preserve_state_and_memory() {
    for enabled in [false, true] {
        let mut s = machine(if enabled {
            &[0xFB, 0x76, 0x04]
        } else {
            &[0x76, 0x04]
        });
        if enabled {
            s.step_instruction(1).unwrap();
        }
        let start = s.elapsed_ticks();
        assert!(matches!(
            s.step_instruction(100).unwrap(),
            InstructionStep::Complete {
                mcycles: 1,
                ticks: 4
            }
        ));
        let before = s.cpu_snapshot();
        let memory: Vec<_> = (0u16..=65535).map(|a| s.memory.read(a)).collect();
        assert_eq!(s.cpu.test_interrupt_status(), (false, enabled));
        for budget in [0, 1, 3, 7] {
            let ticks = s.elapsed_ticks();
            assert!(
                matches!(s.step_instruction(budget).unwrap(),InstructionStep::BudgetExhausted{mcycles,ticks:t} if mcycles==budget && t==budget*4)
            );
            assert_eq!(s.elapsed_ticks(), ticks + budget * 4);
            assert_eq!(s.cpu_snapshot(), before);
        }
        let t = s.step_machine_cycle().map(expect_cycle).unwrap();
        assert_eq!(t.record.action, CycleAction::Idle);
        assert_eq!(t.record.read_value, None);
        assert_eq!(t.outcome, CycleOutcome::InProgress);
        assert_eq!(s.elapsed_ticks() - start, 52);
        for addr in 0u16..=65535 {
            assert_eq!(
                s.memory.read(addr),
                clocked_byte(memory[usize::from(addr)], addr, s.elapsed_ticks())
            );
        }
    }
}

#[test]
fn halt_wakes_only_for_enabled_requested_sources_with_no_extra_wake_cycle() {
    for enabled in [false, true] {
        let mut s = machine(if enabled {
            &[0xFB, 0x76, 0x04]
        } else {
            &[0x76, 0x04]
        });
        if enabled {
            s.step_instruction(1).unwrap();
        }
        s.step_instruction(1).unwrap();
        for (flags, enable) in [(4, 0), (0, 4), (0xE0, 0xE0)] {
            s.memory.write(0xFF0F, flags);
            s.memory.write(0xFFFF, enable);
            assert_eq!(
                s.step_machine_cycle().map(expect_cycle).unwrap().outcome,
                CycleOutcome::InProgress
            );
        }
        let before = s.cpu_snapshot();
        let ticks = s.elapsed_ticks();
        pending(&mut s, 4);
        s.step_instruction(0).unwrap();
        assert_eq!(s.cpu_snapshot(), before);
        assert_eq!(s.elapsed_ticks(), ticks);
        if enabled {
            assert!(matches!(
                s.step_instruction(100).unwrap(),
                InstructionStep::InterruptEntered {
                    vector: 0x50,
                    mcycles: 5,
                    ticks: 20
                }
            ));
            assert_eq!(s.memory.read(0xFF0F), 0xE0);
            assert_eq!(s.memory.read(0xFFFC), before.pc as u8);
            assert_eq!(s.memory.read(0xFFFD), (before.pc / 256) as u8);
        } else {
            assert!(matches!(
                s.step_instruction(100).unwrap(),
                InstructionStep::Complete {
                    mcycles: 1,
                    ticks: 4
                }
            ));
            assert_eq!(s.cpu_snapshot().pc, before.pc + 1);
            assert_eq!(s.cpu_snapshot().bc, before.bc + 0x100);
            assert_eq!(s.memory.read(0xFF0F), 0xE4);
            assert_eq!(s.cpu_snapshot().sp, before.sp);
        }
    }
}

#[test]
fn halt_bug_duplicates_immediate_single_byte_cb_and_rst_fetches() {
    let mut s = machine(&[0x76, 0x3E, 0x12]);
    pending(&mut s, 4);
    s.step_instruction(1).unwrap();
    let before = s.cpu_snapshot();
    s.step_instruction(0).unwrap();
    assert_eq!(s.cpu_snapshot(), before);
    let t = s.step_machine_cycle().map(expect_cycle).unwrap();
    assert_eq!(t.record.action, CycleAction::Read { addr: 0x101 });
    assert_eq!(s.cpu_snapshot().pc, 0x101);
    let t = s.step_machine_cycle().map(expect_cycle).unwrap();
    assert_eq!(t.record.action, CycleAction::Read { addr: 0x101 });
    assert_eq!(s.cpu_snapshot().af >> 8, 0x3E);
    assert_eq!(s.cpu_snapshot().pc, 0x102);

    let mut s = machine(&[0x76, 0x04, 0]);
    pending(&mut s, 4);
    s.step_instruction(1).unwrap();
    for expected_pc in [0x101, 0x102] {
        s.step_instruction(1).unwrap();
        assert_eq!(s.cpu_snapshot().pc, expected_pc);
    }
    assert_eq!(s.cpu_snapshot().bc >> 8, 2);

    let mut s = machine(&[0x76, 0xCB, 0x11]);
    pending(&mut s, 4);
    s.step_instruction(1).unwrap();
    for _ in 0..2 {
        assert_eq!(
            s.step_machine_cycle()
                .map(expect_cycle)
                .unwrap()
                .record
                .action,
            CycleAction::Read { addr: 0x101 }
        );
    }
    assert_eq!(s.cpu_snapshot().de & 255, 0xDA); // CB CB = SET 1,E
    assert_eq!(s.cpu_snapshot().pc, 0x102);

    let mut s = machine(&[0x76, 0xFF]);
    pending(&mut s, 4);
    s.step_instruction(1).unwrap();
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::Complete {
            mcycles: 4,
            ticks: 16
        }
    ));
    assert_eq!(s.cpu_snapshot().pc, 0x38);
    assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (1, 1));
}

#[test]
fn ei_halt_pending_handlers_return_into_halt_then_later_request_returns_after_it() {
    let mut s = machine(&[0xFB, 0x76, 0x04]);
    pending(&mut s, 3);
    s.step_instruction(1).unwrap();
    s.step_instruction(1).unwrap();
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    assert_eq!(s.cpu.test_interrupt_status(), (false, true));
    for vector in [0x40, 0x48] {
        assert!(
            matches!(s.step_instruction(100).unwrap(),InstructionStep::InterruptEntered{vector:v,mcycles:5,ticks:20} if v==vector)
        );
        assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (1, 1));
        let t = s.step_machine_cycle().map(expect_cycle).unwrap();
        assert_eq!(t.record.action, CycleAction::Read { addr: vector });
        assert_eq!(s.cpu_snapshot().pc, vector + 1); // No leaked suppression in handler.
        s.step_instruction(1).unwrap();
        assert_eq!(s.cpu_snapshot().bc >> 8, 0x42);
        s.step_instruction(4).unwrap();
        assert_eq!(s.cpu_snapshot().pc, 0x101);
    }
    s.step_instruction(1).unwrap(); // Re-execute HALT, now with no pending requests.
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    assert!(matches!(
        s.step_instruction(3).unwrap(),
        InstructionStep::BudgetExhausted {
            mcycles: 3,
            ticks: 12
        }
    ));
    pending(&mut s, 4);
    s.step_instruction(5).unwrap();
    assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (2, 1));
    s.step_instruction(2).unwrap();
    s.step_instruction(4).unwrap();
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    s.step_instruction(1).unwrap();
    assert_eq!(s.cpu_snapshot().bc >> 8, 0x43);
}

#[test]
fn repeated_halt_and_di_cancelled_ei_keep_budgets_bounded() {
    let mut s = machine(&[0xFB, 0xF3, 0x76, 0x76, 0]);
    pending(&mut s, 4);
    s.step_instruction(1).unwrap();
    s.step_instruction(1).unwrap();
    s.step_instruction(1).unwrap();
    assert_eq!(s.cpu.test_interrupt_status(), (false, false));
    for _ in 0..4 {
        assert!(matches!(
            s.step_instruction(1).unwrap(),
            InstructionStep::Complete { .. }
        ));
        assert_eq!(s.cpu_snapshot().pc, 0x103);
    }
    s.memory.write(0xFF0F, 0);
    s.step_instruction(1).unwrap();
    assert!(matches!(
        s.step_instruction(7).unwrap(),
        InstructionStep::BudgetExhausted {
            mcycles: 7,
            ticks: 28
        }
    ));
    assert_eq!(s.cpu_snapshot().pc, 0x103);
}

/// Program prefix that arms a Timer request at a fixed M-cycle: IE=04,
/// TIMA=FE, TAC=05 (16-tick increments), then EI.
const TIMER_PREFIX: [u8; 13] = [
    0x3E, 0x04, 0xE0, 0xFF, 0x3E, 0xFE, 0xE0, 0x05, 0x3E, 0x05, 0xE0, 0x07, 0xFB,
];

/// Steps (from 1) at which the first Timer interrupt is selected and its
/// entry completes, after `TIMER_PREFIX` followed by `tail`.
fn timer_entry_steps(tail: &[u8]) -> (u64, u64) {
    let mut program = TIMER_PREFIX.to_vec();
    program.extend_from_slice(tail);
    let mut s = machine(&program);
    let (mut selected, mut complete) = (None, None);
    for step in 1..=200 {
        match s.step_machine_cycle().map(expect_cycle).unwrap().outcome {
            CycleOutcome::InterruptSelected { .. } if selected.is_none() => selected = Some(step),
            CycleOutcome::InterruptEntryComplete { vector: 0x50 } if complete.is_none() => {
                complete = Some(step);
            }
            _ => {}
        }
    }
    (selected.expect("selected"), complete.expect("entered"))
}

#[test]
fn halt_with_ime_services_a_device_raised_request_like_a_nop_wait() {
    // Mooneye halt_ime1_timing2-GS (DMG-verified): with IME=1, HALT services
    // the interrupt "with exactly same timing as if a long series of NOP
    // instructions were used to wait for the interrupt". The request here is
    // raised by the Timer during an M-cycle's device advance, not by an IF
    // write between steps.
    let nop_wait = timer_entry_steps(&[0; 60]);
    let halt_wait = timer_entry_steps(&[0x76, 0x00, 0x00, 0x00]);
    assert_eq!(halt_wait, nop_wait, "HALT adds no M-cycle before entry");
}

#[test]
fn halt_without_ime_fetches_in_the_same_step_as_a_device_raised_request() {
    // Mooneye halt_ime0_nointr_timing (all models): HALT with IME=0 continues
    // "with exactly same timing as if a long series of NOP instructions were
    // used", and its six NOPs equal entry (5) + JP HL (1) in the IME=1 case.
    // So, like IME=1 entry (#79), the next opcode fetch is the bus access of
    // the very M-cycle whose device advance raised the request.
    // DI; IE=04, TIMA=FE, TAC=05; HALT; INC B.
    let mut program = vec![0xF3];
    program.extend_from_slice(&TIMER_PREFIX[..12]);
    program.extend_from_slice(&[0x76, 0x04]);
    let mut s = machine(&program);
    s.memory.write(0xFF0F, 0);
    let after_halt = 0x100 + program.len() as u16 - 1;
    let mut halted_idles = 0;
    for _ in 0..200 {
        let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
        if s.memory.read(0xFF0F) & 0x04 == 0 {
            if s.cpu_snapshot().pc == after_halt && trace.record.action == CycleAction::Idle {
                halted_idles += 1;
            }
            continue;
        }
        assert!(halted_idles > 0, "the request arrived while halted");
        assert_eq!(
            trace.record.action,
            CycleAction::Read { addr: after_halt },
            "INC B fetched in the request's own M-cycle"
        );
        let _ = s.step_machine_cycle().map(expect_cycle).unwrap();
        assert_eq!(s.cpu_snapshot().bc >> 8, 0x01, "INC B executed once");
        assert_eq!(s.memory.read(0xFF0F), 0xE4, "IME=0: request stays pending");
        return;
    }
    panic!("Timer request never observed");
}
