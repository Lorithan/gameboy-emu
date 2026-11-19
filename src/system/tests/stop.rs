use super::super::{InstructionStep, MachineStep, System};
use crate::{
    cpu::{CycleAction, CycleOutcome, CycleRequest},
    memory::Cartridge,
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    rom[0x40] = 0xD9; // RETI
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn memory(s: &System) -> Vec<u8> {
    (0u16..=65535).map(|addr| s.memory.read(addr)).collect()
}

#[test]
fn stop_entry_has_one_read_then_no_cycle_polls_and_preserves_all_memory() {
    // Nonzero padding values check the declared ignored-padding fallback;
    // they do not claim hardware conditional STOP behavior.
    for padding in [0, 4, 0xD3, 0xFF] {
        let mut s = machine(&[0x10, padding, 0x04]);
        let mut expected = s.cpu_snapshot();
        let mut original_memory = memory(&s);
        original_memory[0xFF04] = 0; // STOP resets DIV; other memory is preserved.
        let trace = match s.step_machine_cycle().unwrap() {
            MachineStep::Cycle(trace) => trace,
            other => panic!("STOP entry must execute a cycle, got {other:?}"),
        };
        assert_eq!((trace.record.tick_before, trace.record.tick_after), (0, 4));
        assert_eq!(trace.record.action, CycleAction::Read { addr: 0x100 });
        assert_eq!(trace.record.read_value, Some(0x10));
        assert_eq!(trace.outcome, CycleOutcome::StopEntered);
        expected.pc = 0x102;
        assert_eq!(s.cpu_snapshot(), expected);
        for _ in 0..32 {
            assert!(matches!(
                s.step_machine_cycle().unwrap(),
                MachineStep::Stopped
            ));
            assert_eq!(s.cpu_snapshot(), expected);
            assert_eq!(s.elapsed_ticks(), 4);
        }
        assert_eq!(memory(&s), original_memory);
    }
}

#[test]
fn stop_budgets_count_only_real_cycles_and_zero_budget_never_enters() {
    for budget in [0, 1, 2, 100] {
        let mut s = machine(&[0x10, 0, 0xD3]);
        let before = s.cpu_snapshot();
        let result = s.step_instruction(budget).unwrap();
        if budget == 0 {
            assert!(matches!(
                result,
                InstructionStep::BudgetExhausted {
                    mcycles: 0,
                    ticks: 0
                }
            ));
            assert_eq!(s.cpu_snapshot(), before);
            assert_eq!(s.elapsed_ticks(), 0);
            assert_eq!(
                s.cpu.request_cycle().unwrap(),
                CycleRequest::Execute(CycleAction::Read { addr: 0x100 })
            );
            assert!(matches!(
                s.step_instruction(100).unwrap(),
                InstructionStep::Stopped {
                    mcycles: 1,
                    ticks: 4
                }
            ));
        } else {
            assert!(matches!(
                result,
                InstructionStep::Stopped {
                    mcycles: 1,
                    ticks: 4
                }
            ));
        }
        let mut expected = before;
        expected.pc = 0x102;
        assert_eq!(s.cpu_snapshot(), expected);
        for later_budget in [0, 1, 3, u64::MAX] {
            let result = s.step_instruction(later_budget).unwrap();
            if later_budget == 0 {
                assert!(matches!(
                    result,
                    InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    }
                ));
            } else {
                assert!(matches!(
                    result,
                    InstructionStep::Stopped {
                        mcycles: 0,
                        ticks: 0
                    }
                ));
            }
            assert_eq!(s.cpu_snapshot(), expected);
            assert_eq!(s.elapsed_ticks(), 4);
        }
    }
}

#[test]
fn ei_di_stop_bookkeeping_and_pending_interrupts_cannot_resume_standby() {
    for (program, prefix, enabled) in [
        (&[0x10, 0][..], 0, false),
        (&[0xFB, 0x10, 0][..], 1, true),
        (&[0xFB, 0xF3, 0x10, 0][..], 2, false),
        (&[0xFB, 0, 0x10, 0][..], 2, true),
        (&[0xFB, 0xFB, 0x10, 0][..], 2, true),
    ] {
        let mut s = machine(program);
        for _ in 0..prefix {
            assert!(matches!(
                s.step_instruction(1).unwrap(),
                InstructionStep::Complete {
                    mcycles: 1,
                    ticks: 4
                }
            ));
        }
        let start_ticks = s.elapsed_ticks();
        assert!(matches!(
            s.step_instruction(100).unwrap(),
            InstructionStep::Stopped {
                mcycles: 1,
                ticks: 4
            }
        ));
        assert_eq!(s.cpu.test_interrupt_status(), (false, enabled));
        let before = s.cpu_snapshot();
        for mask in [0, 1, 2, 4, 8, 16, 31, 0xE0, 0xFF] {
            s.memory.write(0xFFFF, mask);
            s.memory.write(0xFF0F, mask);
            let before_memory = memory(&s);
            assert!(matches!(
                s.step_machine_cycle().unwrap(),
                MachineStep::Stopped
            ));
            assert!(matches!(
                s.step_instruction(100).unwrap(),
                InstructionStep::Stopped {
                    mcycles: 0,
                    ticks: 0
                }
            ));
            assert_eq!(s.cpu_snapshot(), before);
            assert_eq!(s.elapsed_ticks(), start_ticks + 4);
            assert_eq!(s.cpu.test_interrupt_status(), (false, enabled));
            assert_eq!(memory(&s), before_memory, "standby changed IF/IE or memory");
        }
    }
}

#[test]
fn pending_interrupt_entry_precedes_stop_fetch_and_reti_returns_to_stop() {
    let mut s = machine(&[0xFB, 0, 0x10, 0]);
    for _ in 0..2 {
        assert!(matches!(
            s.step_instruction(1).unwrap(),
            InstructionStep::Complete { .. }
        ));
    }
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    s.memory.write(0xFFFF, 1);
    s.memory.write(0xFF0F, 1);
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::InterruptEntered {
            vector: 0x40,
            mcycles: 5,
            ticks: 20
        }
    ));
    assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (2, 1));
    assert_eq!(s.memory.read(0xFF0F), 0xE0);
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::Complete {
            mcycles: 4,
            ticks: 16
        }
    ));
    assert_eq!(s.cpu_snapshot().pc, 0x102);
    assert!(matches!(
        s.step_instruction(100).unwrap(),
        InstructionStep::Stopped {
            mcycles: 1,
            ticks: 4
        }
    ));
    assert_eq!(s.cpu_snapshot().pc, 0x104);
    assert_eq!(s.cpu.test_interrupt_status(), (false, true));
    assert_eq!(s.elapsed_ticks(), 48);
}

#[test]
fn stop_opcode_and_padding_pc_wrap_across_hram_ie_and_rom() {
    for pc in [0xFFFEu16, 0xFFFF] {
        let [low, high] = pc.to_le_bytes();
        let mut s = machine(&[0xC3, low, high]);
        s.memory.write(pc, 0x10);
        if pc == 0xFFFE {
            s.memory.write(0xFFFF, 0);
        }
        assert!(matches!(
            s.step_instruction(100).unwrap(),
            InstructionStep::Complete {
                mcycles: 4,
                ticks: 16
            }
        ));
        let mut expected = s.cpu_snapshot();
        let mut before_memory = memory(&s);
        before_memory[0xFF04] = 0;
        assert_eq!(expected.pc, pc);
        assert!(matches!(
            s.step_instruction(100).unwrap(),
            InstructionStep::Stopped {
                mcycles: 1,
                ticks: 4
            }
        ));
        expected.pc = if pc == 0xFFFE { 0 } else { 1 };
        assert_eq!(s.cpu_snapshot(), expected);
        assert_eq!(s.elapsed_ticks(), 20);
        assert_eq!(memory(&s), before_memory);
    }
}
