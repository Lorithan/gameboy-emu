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
