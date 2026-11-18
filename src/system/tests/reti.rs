use super::super::{InstructionStep, System};
use crate::response_support::expect_execute;
use crate::step_support::expect_cycle;
use crate::{
    cpu::{CycleAction, CycleOutcome},
    memory::Cartridge,
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    rom[0x40..0x42].copy_from_slice(&[0x14, 0xD9]); // INC D; RETI
    rom[0x58..0x5A].copy_from_slice(&[0x1C, 0xD9]); // INC E; RETI
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

#[test]
fn reti_bus_reads_exact_traces_and_all_budget_resumes_preserve_memory() {
    for (sp, target) in [
        (0xC123u16, 0x1234u16),
        (0xE123, 0x1234),
        (0xFFFC, 0x1234),
        (0xFFFE, 0x1234),
        (0xFFFF, 0x0034),
        (0xFF0F, 0xFFF4),
        (0x7FFF, 0xFF00),
        (0xBFFF, 0x12FF),
    ] {
        for budget in [0, 1, 2, 3, 4, 100] {
            let mut s = machine(&[0x31, sp as u8, (sp / 256) as u8, 0xD9, 0]);
            s.step_instruction(3).unwrap();
            s.memory.write(sp, 0x34);
            s.memory.write(sp.wrapping_add(1), 0x12);
            let original: Vec<_> = (0u16..=65535).map(|a| s.memory.read(a)).collect();
            let before = s.cpu_snapshot();
            let first = s.step_instruction(budget).unwrap();
            if budget < 4 {
                assert!(
                    matches!(first,InstructionStep::BudgetExhausted{mcycles,ticks} if mcycles==budget && ticks==budget*4)
                );
            } else {
                assert!(matches!(
                    first,
                    InstructionStep::Complete {
                        mcycles: 4,
                        ticks: 16
                    }
                ));
            }
            let phase = budget.min(4);
            let mut expected = before;
            if phase > 0 {
                expected.pc = 0x104;
            }
            if phase >= 2 {
                expected.sp = sp.wrapping_add(if phase == 2 { 1 } else { 2 });
            }
            if phase == 4 {
                expected.pc = target;
            }
            assert_eq!(s.cpu_snapshot(), expected);
            assert_eq!(s.cpu.test_interrupt_status(), (false, phase == 4));
            s.step_instruction(0).unwrap();
            assert_eq!(s.cpu_snapshot(), expected);
            assert_eq!(s.elapsed_ticks(), 12 + phase * 4);
            for cycle in phase..4 {
                let t = s.step_machine_cycle().map(expect_cycle).unwrap();
                let action = match cycle {
                    0 => CycleAction::Read { addr: 0x103 },
                    1 => CycleAction::Read { addr: sp },
                    2 => CycleAction::Read {
                        addr: sp.wrapping_add(1),
                    },
                    _ => CycleAction::Idle,
                };
                assert_eq!(t.record.action, action);
                assert_eq!(
                    t.record.read_value,
                    match cycle {
                        0 => Some(0xD9),
                        1 => Some(target as u8),
                        2 => Some((target / 256) as u8),
                        _ => None,
                    }
                );
                assert_eq!(
                    (t.record.tick_before, t.record.tick_after),
                    (12 + cycle * 4, 16 + cycle * 4)
                );
                assert_eq!(
                    t.outcome,
                    if cycle == 3 {
                        CycleOutcome::InstructionComplete
                    } else {
                        CycleOutcome::InProgress
                    }
                );
                assert_eq!(s.cpu.test_interrupt_status(), (false, cycle == 3));
            }
            let mut expected = before;
            expected.pc = target;
            expected.sp = sp.wrapping_add(2);
            assert_eq!(s.cpu_snapshot(), expected);
            assert_eq!(
                s.cpu.request_cycle().map(expect_execute).unwrap(),
                CycleAction::Read { addr: target }
            );
            assert_eq!(s.elapsed_ticks(), 28);
            for addr in 0u16..=65535 {
                assert_eq!(s.memory.read(addr), original[usize::from(addr)]);
            }
        }
    }
}

#[test]
fn reti_enables_pending_interrupt_before_returned_to_opcode() {
    for delayed in [false, true] {
        let mut bytes = vec![0x31, 0xFC, 0xFF];
        if delayed {
            bytes.push(0xFB);
        }
        bytes.push(0xD9);
        for pause in 0..=4 {
            let mut s = machine(&bytes);
            s.step_instruction(3).unwrap();
            if delayed {
                s.step_instruction(1).unwrap();
            }
            s.memory.write(0xFFFC, 0x34);
            s.memory.write(0xFFFD, 0x12);
            s.memory.write(0xFF0F, 4);
            s.memory.write(0xFFFF, 4);
            s.step_instruction(pause).unwrap();
            if pause < 4 {
                assert_eq!(s.cpu.test_interrupt_status(), (delayed, false));
                assert!(
                    matches!(s.step_instruction(100).unwrap(),InstructionStep::Complete{mcycles,ticks} if mcycles==4-pause && ticks==(4-pause)*4)
                );
            }
            assert_eq!(s.cpu_snapshot().pc, 0x1234);
            assert_eq!(s.cpu_snapshot().sp, 0xFFFE);
            assert_eq!(s.memory.read(0xFF0F), 0xE4);
            assert_eq!(s.cpu.test_interrupt_status(), (false, true));
            let t = s.step_machine_cycle().map(expect_cycle).unwrap();
            assert_eq!(t.record.action, CycleAction::Idle); // No returned-to opcode fetch.
            assert_eq!(t.outcome, CycleOutcome::InProgress);
            assert!(matches!(
                s.step_instruction(100).unwrap(),
                InstructionStep::InterruptEntered {
                    vector: 0x50,
                    mcycles: 4,
                    ticks: 16
                }
            ));
            assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (0x34, 0x12));
            assert_eq!(s.memory.read(0xFF0F), 0xE0);
        }
    }
}

#[test]
fn reti_chains_pending_handlers_then_returns_to_mainline_once() {
    let mut s = machine(&[0xFB, 0x04, 0x04, 0]); // EI; INC B; INC B
    s.memory.write(0xFF0F, 9);
    s.memory.write(0xFFFF, 9);
    for _ in 0..2 {
        s.step_instruction(1).unwrap();
    }
    assert_eq!(s.cpu_snapshot().bc >> 8, 1);
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::InterruptEntered {
            vector: 0x40,
            mcycles: 5,
            ticks: 20
        }
    ));
    s.step_instruction(1).unwrap(); // INC D
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::Complete {
            mcycles: 4,
            ticks: 16
        }
    ));
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    assert_eq!(s.cpu_snapshot().sp, 0xFFFE);
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::InterruptEntered {
            vector: 0x58,
            mcycles: 5,
            ticks: 20
        }
    ));
    assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (2, 1));
    assert_eq!(s.cpu_snapshot().bc >> 8, 1); // Returned-to INC B has not executed.
    s.step_instruction(1).unwrap(); // INC E
    s.step_instruction(4).unwrap(); // RETI with no remaining requests
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    assert_eq!(s.cpu_snapshot().sp, 0xFFFE);
    assert_eq!(s.memory.read(0xFF0F), 0xE0);
    s.step_instruction(1).unwrap();
    assert_eq!(s.cpu_snapshot().bc >> 8, 2);
    assert_eq!(s.cpu_snapshot().de, 0x01D9); // D/E each incremented once.
    assert_eq!(s.cpu_snapshot().pc, 0x103);
    assert_eq!(s.elapsed_ticks(), 92);
}
