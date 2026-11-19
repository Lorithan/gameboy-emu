use super::super::{InstructionStep, MachineStep, System};
use crate::{
    cpu::{CycleAction, CycleOutcome, CycleRequest},
    memory::{Button, Cartridge},
    step_support::expect_cycle,
};

#[test]
fn stop_freezes_new_overflow_until_selected_input_and_the_first_resumed_cycle() {
    let mut s = machine(&[0, 0x10, 0, 0, 0]);
    s.memory.write(0xFF0F, 0);
    s.memory.write(0xFFFF, 4);
    s.memory.write(0xFF05, 255);
    s.memory.write(0xFF06, 0x42);
    s.memory.write(0xFF07, 5);
    let _ = expect_cycle(s.step_machine_cycle().unwrap());
    assert_eq!(s.memory.read(0xFF05), 255);
    let entry = expect_cycle(s.step_machine_cycle().unwrap());
    assert_eq!(entry.outcome, CycleOutcome::StopEntered);
    assert_eq!(entry.record.tick_after, 8);
    assert_eq!(s.cpu_snapshot().pc, 0x103);
    assert_eq!(
        (
            s.memory.read(0xFF04),
            s.memory.read(0xFF05),
            s.memory.read(0xFF0F)
        ),
        (0, 0, 0xE0)
    );
    let stopped_cpu = s.cpu_snapshot();
    let stopped_memory = memory(&s);
    for budget in [1, 16, 1000] {
        assert!(matches!(
            s.step_machine_cycle().unwrap(),
            MachineStep::Stopped
        ));
        assert!(matches!(
            s.step_instruction(budget).unwrap(),
            InstructionStep::Stopped {
                mcycles: 0,
                ticks: 0
            }
        ));
        assert_eq!(s.cpu_snapshot(), stopped_cpu);
        assert_eq!(memory(&s), stopped_memory);
        assert_eq!(s.elapsed_ticks(), 8);
    }
    // Inject a flagged eligible IRQ while stopped: it cannot replace JOYP wake.
    s.memory.write(0xFF0F, 4);
    assert!(matches!(
        s.step_machine_cycle().unwrap(),
        MachineStep::Stopped
    ));
    assert_eq!(
        (
            s.memory.read(0xFF05),
            s.memory.read(0xFF0F),
            s.elapsed_ticks()
        ),
        (0, 0xE4, 8)
    );
    s.memory.write(0xFF0F, 0);
    s.set_button(Button::A, true);
    s.set_button(Button::A, false);
    assert_eq!(s.cpu_snapshot(), stopped_cpu);
    assert_eq!(
        (
            s.memory.read(0xFF04),
            s.memory.read(0xFF05),
            s.memory.read(0xFF0F),
            s.elapsed_ticks()
        ),
        (0, 0, 0xF0, 8)
    );
    let resumed = expect_cycle(s.step_machine_cycle().unwrap());
    assert_eq!(resumed.record.action, CycleAction::Read { addr: 0x103 });
    assert_eq!(resumed.outcome, CycleOutcome::InstructionComplete);
    assert_eq!(resumed.record.tick_after, 12);
    assert_eq!((s.memory.read(0xFF05), s.memory.read(0xFF0F)), (0x42, 0xF4));
    let mut expected = stopped_cpu;
    expected.pc = 0x104;
    assert_eq!(s.cpu_snapshot(), expected);
}

#[test]
fn stop_retains_reload_irq_from_its_fetch_cycle_until_wake_and_late_acknowledgment() {
    let mut s = machine(&[0xFB, 0, 0x10, 0, 0]);
    s.memory.write(0xFF0F, 0);
    s.memory.write(0xFFFF, 4);
    s.memory.write(0xFF05, 255);
    s.memory.write(0xFF06, 0x42);
    s.memory.write(0xFF07, 5);
    for _ in 0..2 {
        let _ = expect_cycle(s.step_machine_cycle().unwrap());
    }
    let entry = expect_cycle(s.step_machine_cycle().unwrap());
    assert_eq!(entry.outcome, CycleOutcome::StopEntered);
    assert_eq!(entry.record.tick_after, 12);
    assert_eq!((s.memory.read(0xFF05), s.memory.read(0xFF0F)), (0x42, 0xE4));
    let stopped = s.cpu_snapshot();
    assert_eq!(stopped.pc, 0x103); // pending mask selects the one-byte branch
    assert_eq!(s.cpu.test_interrupt_status(), (false, true));
    for _ in 0..3 {
        assert!(matches!(
            s.step_machine_cycle().unwrap(),
            MachineStep::Stopped
        ));
        assert_eq!(
            (
                s.memory.read(0xFF05),
                s.memory.read(0xFF0F),
                s.elapsed_ticks()
            ),
            (0x42, 0xE4, 12)
        );
    }
    s.set_button(Button::A, true);
    s.set_button(Button::A, false);
    assert_eq!(s.elapsed_ticks(), 12);
    for cycle in 1..=5 {
        let trace = expect_cycle(s.step_machine_cycle().unwrap());
        assert_eq!(trace.record.tick_after, 12 + cycle * 4);
        assert_eq!(
            trace.record.action,
            match cycle {
                3 => CycleAction::Write {
                    addr: 0xFFFD,
                    value: 1
                },
                4 => CycleAction::Write {
                    addr: 0xFFFC,
                    value: 3
                },
                _ => CycleAction::Idle,
            }
        );
        assert_eq!(s.memory.read(0xFF0F), if cycle < 4 { 0xF4 } else { 0xF0 });
        if cycle == 5 {
            assert_eq!(
                trace.outcome,
                CycleOutcome::InterruptEntryComplete { vector: 0x50 }
            );
        }
    }
    let mut expected = stopped;
    expected.pc = 0x50;
    expected.sp = 0xFFFC;
    assert_eq!(s.cpu_snapshot(), expected);
    assert_eq!(s.memory.read(0xFF05), 0x43);
}

#[test]
fn stop_halt_like_and_continue_branches_keep_tima_clocking_without_div_reset() {
    for pending in [false, true] {
        let mut s = machine(&[0x10, 0, 0]);
        s.set_button(Button::A, true);
        s.memory.write(0xFFFF, 4);
        s.memory.write(0xFF0F, if pending { 4 } else { 0 });
        s.memory.write(0xFF05, 0x20);
        s.memory.write(0xFF07, 5);
        let first = expect_cycle(s.step_machine_cycle().unwrap());
        assert_eq!(first.outcome, CycleOutcome::InstructionComplete);
        assert_eq!((s.memory.read(0xFF04), s.memory.read(0xFF05)), (0xAB, 0x20));
        assert_eq!(s.cpu_snapshot().pc, if pending { 0x101 } else { 0x102 });
        let next = expect_cycle(s.step_machine_cycle().unwrap());
        assert_eq!(
            next.record.action,
            if pending {
                CycleAction::Read { addr: 0x101 }
            } else {
                CycleAction::Idle
            }
        );
        assert_eq!(
            next.outcome,
            if pending {
                CycleOutcome::InstructionComplete
            } else {
                CycleOutcome::InProgress
            }
        );
        assert_eq!(
            (
                s.memory.read(0xFF04),
                s.memory.read(0xFF05),
                s.elapsed_ticks()
            ),
            (0xAB, 0x21, 8)
        );
    }
}

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
fn stop_wake_uses_selected_buttons_and_retains_pulses_without_time_or_divider_effects() {
    for selection in [0, 0x10, 0x20, 0x30] {
        for (button, row) in [
            (Button::Right, 0x10),
            (Button::Left, 0x10),
            (Button::Up, 0x10),
            (Button::Down, 0x10),
            (Button::A, 0x20),
            (Button::B, 0x20),
            (Button::Select, 0x20),
            (Button::Start, 0x20),
        ] {
            let mut s = machine(&[0x10, 0, 0]);
            s.memory.write(0xFF00, selection);
            s.memory.write(0xFFFF, 0); // Wake must work with no eligible IRQ.
            s.memory.write(0xFF0F, 0);
            s.step_instruction(1).unwrap();
            let before = s.cpu_snapshot();
            let before_memory = memory(&s);
            let stopped_debug = format!("{:?}", s.cpu);
            let selected = selection & row == 0;
            s.set_button(button, true);
            s.set_button(button, false); // Entire pulse between execution calls.
            assert_eq!(s.cpu_snapshot(), before);
            assert_eq!(s.cpu.test_interrupt_status(), (false, false));
            assert_eq!(s.elapsed_ticks(), 4);
            assert_eq!(s.memory.read(0xFF04), 0);
            for addr in 0u16..=65535 {
                let expected = if addr == 0xFF0F && selected {
                    0xF0
                } else {
                    before_memory[usize::from(addr)]
                };
                assert_eq!(
                    s.memory.read(addr),
                    expected,
                    "selection {selection:02X}, {button:?}, address {addr:04X}"
                );
            }
            assert!(matches!(
                s.step_instruction(0).unwrap(),
                InstructionStep::BudgetExhausted {
                    mcycles: 0,
                    ticks: 0
                }
            ));
            if selected {
                assert_eq!(
                    s.cpu.request_cycle().unwrap(),
                    CycleRequest::Execute(CycleAction::Read { addr: 0x102 })
                );
                for cycle in 1..=64 {
                    assert!(matches!(
                        s.step_instruction(1).unwrap(),
                        InstructionStep::Complete {
                            mcycles: 1,
                            ticks: 4
                        }
                    ));
                    assert_eq!(s.memory.read(0xFF04), if cycle < 64 { 0 } else { 1 });
                }
                assert_eq!(s.elapsed_ticks(), 260);
                assert_eq!(s.memory.read(0xFF0F), 0xF0); // Wake never acknowledges IF.
            } else {
                assert_eq!(format!("{:?}", s.cpu), stopped_debug);
                assert!(matches!(
                    s.step_machine_cycle().unwrap(),
                    MachineStep::Stopped
                ));
                assert_eq!(s.elapsed_ticks(), 4);
            }
        }
    }
}

#[test]
fn stop_wake_interrupt_arbitration_occurs_on_next_step_and_keeps_the_resume_address() {
    for enabled in [false, true] {
        for eligible in [false, true] {
            for pending_at_entry in [false, true] {
                // EI;STOP matures IME at completion without accepting an
                // already pending interrupt before STOP's fetch.
                let program: &[u8] = if enabled {
                    &[0xFB, 0x10, 0, 0]
                } else {
                    &[0x10, 0, 0]
                };
                let mut s = machine(program);
                if enabled {
                    s.step_instruction(1).unwrap();
                }
                s.memory.write(0xFFFF, if pending_at_entry { 1 } else { 0 });
                s.memory.write(0xFF0F, if pending_at_entry { 1 } else { 0 });
                s.step_instruction(1).unwrap();
                let before = s.cpu_snapshot();
                let ticks = s.elapsed_ticks();
                let expected_pc = 0x100 + u16::from(enabled) + if pending_at_entry { 1 } else { 2 };
                assert_eq!(before.pc, expected_pc);
                s.memory.write(0xFFFF, if eligible { 16 } else { 0 });
                s.memory.write(0xFF0F, 0);
                s.set_button(Button::A, true);
                s.set_button(Button::A, false);
                assert_eq!(s.cpu_snapshot(), before);
                assert_eq!(s.cpu.test_interrupt_status(), (false, enabled));
                assert_eq!(s.elapsed_ticks(), ticks);
                assert_eq!(s.memory.read(0xFF0F), 0xF0);
                assert_eq!(s.memory.read(0xFF04), 0);
                assert_eq!((s.memory.read(0xFFFC), s.memory.read(0xFFFD)), (0, 0));
                if enabled && eligible {
                    for cycle in 0..5 {
                        let trace = match s.step_machine_cycle().unwrap() {
                            MachineStep::Cycle(t) => t,
                            other => panic!("wake failed: {other:?}"),
                        };
                        assert_eq!(
                            trace.record.action,
                            match cycle {
                                2 => CycleAction::Write {
                                    addr: 0xFFFD,
                                    value: (expected_pc >> 8) as u8
                                },
                                3 => CycleAction::Write {
                                    addr: 0xFFFC,
                                    value: expected_pc as u8
                                },
                                _ => CycleAction::Idle,
                            }
                        );
                        assert_eq!(s.memory.read(0xFF0F), if cycle < 3 { 0xF0 } else { 0xE0 });
                        if cycle == 4 {
                            assert_eq!(
                                trace.outcome,
                                CycleOutcome::InterruptEntryComplete { vector: 0x60 }
                            );
                        }
                    }
                    assert_eq!(s.cpu_snapshot().pc, 0x60);
                    assert_eq!(s.elapsed_ticks(), ticks + 20);
                } else {
                    assert!(matches!(
                        s.step_instruction(1).unwrap(),
                        InstructionStep::Complete {
                            mcycles: 1,
                            ticks: 4
                        }
                    ));
                    assert_eq!(s.cpu_snapshot().pc, expected_pc + 1);
                    assert_eq!(s.memory.read(0xFF0F), 0xF0);
                }
            }
        }
    }
}

#[test]
fn stop_wake_does_not_remember_input_before_entry_or_infer_wake_from_stale_if() {
    let mut s = machine(&[0x10, 0, 0]);
    s.set_button(Button::A, true);
    s.set_button(Button::A, false);
    s.memory.write(0xFFFF, 16); // Persistent eligible IF yields one-byte STOP.
    s.step_instruction(1).unwrap();
    assert_eq!(s.cpu_snapshot().pc, 0x101);
    let before = format!("{:?}", s.cpu);
    for _ in 0..3 {
        s.set_button(Button::A, false);
        assert!(matches!(
            s.step_machine_cycle().unwrap(),
            MachineStep::Stopped
        ));
    }
    assert_eq!(format!("{:?}", s.cpu), before);
    assert_eq!(s.elapsed_ticks(), 4);
    s.set_button(Button::A, true);
    assert_eq!(
        s.cpu.request_cycle().unwrap(),
        CycleRequest::Execute(CycleAction::Read { addr: 0x101 })
    );
    assert_eq!(s.elapsed_ticks(), 4);
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
