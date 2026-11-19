use super::super::{InstructionStep, MachineStep, System};
use crate::{
    cpu::{CycleAction, CycleOutcome, CycleRequest},
    memory::{Button, Cartridge},
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
fn conditional_stop_without_selected_input_or_pending_interrupt_enters_two_byte_stop() {
    check_stop_entry(false, false, 2, true, false);
}

#[test]
fn conditional_stop_without_selected_input_with_pending_interrupt_enters_one_byte_stop() {
    check_stop_entry(false, true, 1, true, false);
}

#[test]
fn conditional_stop_with_selected_input_without_pending_interrupt_enters_two_byte_halt() {
    check_stop_entry(true, false, 2, false, true);
}

#[test]
fn conditional_stop_with_selected_input_and_pending_interrupt_continues_after_one_byte() {
    check_stop_entry(true, true, 1, false, false);
}

fn check_stop_entry(wake: bool, pending: bool, length: u16, stopped: bool, halted: bool) {
    for selection in [0u8, 0x10, 0x20, 0x30] {
        if wake && selection == 0x30 {
            continue;
        }
        for delayed in [false, true] {
            let program: &[u8] = if delayed {
                &[0xFB, 0x10, 0, 0]
            } else {
                &[0x10, 0, 0]
            };
            let mut s = machine(program);
            if delayed {
                s.step_instruction(1).unwrap();
            }
            s.memory.write(0xFF00, selection);
            // An unselected held button must not assert wake.
            if wake {
                let button = if selection == 0x20 {
                    Button::Right
                } else {
                    Button::A
                };
                s.set_button(button, true);
            } else if selection == 0x10 || selection == 0x30 {
                s.set_button(Button::Right, true);
            } else if selection == 0x20 {
                s.set_button(Button::A, true);
            }
            // Overwrite generated joypad IF. Pending VBlank and upper IE
            // bits verify that wake does not depend on joypad IE/IF.
            s.memory.write(0xFFFF, 0xE1);
            s.memory.write(0xFF0F, if pending { 1 } else { 0 });
            let mut expected = s.cpu_snapshot();
            let before_pc = expected.pc;
            let ticks = s.elapsed_ticks();
            let before_debug = format!("{:?}", s.cpu);
            assert!(matches!(
                s.step_instruction(0).unwrap(),
                InstructionStep::BudgetExhausted {
                    mcycles: 0,
                    ticks: 0
                }
            ));
            assert_eq!(format!("{:?}", s.cpu), before_debug);
            let mut expected_memory = memory(&s);
            if stopped {
                expected_memory[0xFF04] = 0;
            }
            let trace = match s.step_machine_cycle().unwrap() {
                MachineStep::Cycle(trace) => trace,
                other => panic!("STOP fetch did not execute: {other:?}"),
            };
            assert_eq!(trace.record.action, CycleAction::Read { addr: before_pc });
            assert_eq!(trace.record.read_value, Some(0x10));
            assert_eq!(
                (trace.record.tick_before, trace.record.tick_after),
                (ticks, ticks + 4)
            );
            assert_eq!(
                trace.outcome,
                if stopped {
                    CycleOutcome::StopEntered
                } else {
                    CycleOutcome::InstructionComplete
                }
            );
            expected.pc = before_pc.wrapping_add(length);
            assert_eq!(
                s.cpu_snapshot(),
                expected,
                "wake {wake}, pending {pending}, selection {selection:02X}"
            );
            assert_eq!(s.cpu.test_interrupt_status(), (false, delayed));
            assert_eq!(memory(&s), expected_memory);
            assert_eq!(
                s.cpu.request_cycle().unwrap(),
                if stopped {
                    CycleRequest::Stopped
                } else if halted {
                    CycleRequest::Execute(CycleAction::Idle)
                } else {
                    CycleRequest::Execute(CycleAction::Read { addr: expected.pc })
                }
            );
            // With arbitration neutralized, HALT advances real clocks;
            // STOP keeps DIV reset and produces no cycle at all.
            s.memory.write(0xFFFF, 0);
            if stopped {
                for _ in 0..3 {
                    assert!(matches!(
                        s.step_machine_cycle().unwrap(),
                        MachineStep::Stopped
                    ));
                }
                assert_eq!(s.elapsed_ticks(), ticks + 4);
                assert_eq!(s.memory.read(0xFF04), 0);
            } else if halted {
                for _ in 0..13 {
                    let idle = match s.step_machine_cycle().unwrap() {
                        MachineStep::Cycle(t) => t,
                        other => panic!("HALT lost its clock: {other:?}"),
                    };
                    assert_eq!(idle.record.action, CycleAction::Idle);
                    assert_eq!(idle.outcome, CycleOutcome::InProgress);
                }
                assert_eq!(s.cpu_snapshot(), expected);
                assert_eq!(s.elapsed_ticks(), ticks + 56);
                assert_eq!(s.memory.read(0xFF04), 0xAC);
            } else {
                assert!(matches!(
                    s.step_instruction(1).unwrap(),
                    InstructionStep::Complete {
                        mcycles: 1,
                        ticks: 4
                    }
                ));
                expected.pc = expected.pc.wrapping_add(1);
                assert_eq!(s.cpu_snapshot(), expected);
            }
        }
    }
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
