use super::super::{InstructionStep, MachineStep, System, SystemError};
use crate::{
    cpu::{CpuError, CycleAction, CycleOutcome},
    memory::Cartridge,
    step_support::expect_cycle,
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

#[test]
fn divider_system_boot_phase_reads_observe_the_completing_cycle() {
    // Original synthetic program reproducing the pinned boot_div sample
    // spacing. This is not a copied external ROM or an external-suite pass.
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x104].copy_from_slice(&[0, 0xC3, 0x50, 1]);
    let mut body = Vec::new();
    for nops in [6, 57, 56, 57, 57, 58] {
        body.resize(body.len() + nops, 0);
        body.extend_from_slice(&[0xF0, 4, 0xF5]); // LDH A,(DIV); PUSH AF
    }
    rom[0x150..0x150 + body.len()].copy_from_slice(&body);
    let mut s = System::new(Cartridge::from_bytes(&rom).unwrap());
    let expected = [
        (14, 0xAC),
        (78, 0xAD),
        (141, 0xAD),
        (205, 0xAE),
        (269, 0xAF),
        (334, 0xB1),
    ];
    let mut sample = 0;
    assert_eq!(s.memory.read(0xFF04), 0xAB);
    for mcycles in 1..=334 {
        let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
        assert_eq!(
            (trace.record.tick_before, trace.record.tick_after),
            ((mcycles - 1) * 4, mcycles * 4)
        );
        if trace.record.action == (CycleAction::Read { addr: 0xFF04 }) {
            assert_eq!(mcycles, expected[sample].0);
            assert_eq!(trace.record.read_value, Some(expected[sample].1));
            assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
            assert_eq!(s.cpu_snapshot().af / 256, u16::from(expected[sample].1));
            assert_eq!(s.memory.read(0xFF04), expected[sample].1);
            sample += 1;
        }
    }
    assert_eq!(sample, 6);
    assert_eq!(s.elapsed_ticks(), 1336);
}

#[test]
fn divider_system_write_resets_after_its_cycle_and_restarts_hidden_phase() {
    for value in [0, 0x5A, 0xFF] {
        for pause in 0..=3 {
            let mut s = machine(&[0x3E, value, 0xE0, 4]);
            s.step_instruction(2).unwrap();
            assert_eq!(s.elapsed_ticks(), 8);
            let before = s.cpu_snapshot();
            let result = s.step_instruction(pause).unwrap();
            if pause < 3 {
                assert!(
                    matches!(result, InstructionStep::BudgetExhausted { mcycles, ticks }
                    if mcycles == pause && ticks == pause * 4)
                );
                assert_eq!(s.memory.read(0xFF04), 0xAB, "reset before write cycle");
                if pause == 0 {
                    assert_eq!(s.cpu_snapshot(), before);
                }
                for phase in pause..3 {
                    let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
                    assert_eq!(
                        (trace.record.tick_before, trace.record.tick_after),
                        (8 + phase * 4, 12 + phase * 4)
                    );
                    if phase == 2 {
                        assert_eq!(
                            trace.record.action,
                            CycleAction::Write {
                                addr: 0xFF04,
                                value
                            }
                        );
                        assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
                    }
                }
            } else {
                assert!(matches!(
                    result,
                    InstructionStep::Complete {
                        mcycles: 3,
                        ticks: 12
                    }
                ));
            }
            assert_eq!(s.elapsed_ticks(), 20);
            assert_eq!(s.memory.read(0xFF04), 0);
            for later in 1..=64 {
                let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
                assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
                assert_eq!(s.memory.read(0xFF04), u8::from(later == 64));
                assert_eq!(s.elapsed_ticks(), 20 + later * 4);
            }
        }
    }
}

#[test]
fn divider_halt_and_every_interrupt_entry_phase_continue_clocking() {
    let mut halted = machine(&[0x76]);
    halted.step_instruction(1).unwrap();
    for wait in 1..=300 {
        let trace = halted.step_machine_cycle().map(expect_cycle).unwrap();
        assert_eq!(trace.record.action, CycleAction::Idle);
        assert_eq!(trace.outcome, CycleOutcome::InProgress);
        let expected_ticks = (1 + wait) * 4;
        assert_eq!(halted.elapsed_ticks(), expected_ticks);
        assert_eq!(
            halted.memory.read(0xFF04),
            ((43_976 + expected_ticks) / 256) as u8
        );
    }

    // Position a DIV transition in each of entry's five phases, proving that
    // none of its idles or stack writes can omit device advancement.
    for crossing_phase in 1..=5 {
        let mut s = machine(&[0xFB, 0x76]);
        s.step_instruction(1).unwrap();
        s.step_instruction(1).unwrap();
        for _ in 0..12 - crossing_phase {
            s.step_machine_cycle().map(expect_cycle).unwrap();
        }
        let start = s.elapsed_ticks();
        assert_eq!(s.memory.read(0xFF04), 0xAB);
        s.memory.write(0xFFFF, 1);
        s.memory.write(0xFF0F, 1);
        for phase in 1..=5 {
            let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
            assert_eq!(
                (trace.record.tick_before, trace.record.tick_after),
                (start + (phase - 1) * 4, start + phase * 4)
            );
            assert_eq!(
                s.memory.read(0xFF04),
                if phase < crossing_phase { 0xAB } else { 0xAC }
            );
            if phase == 5 {
                assert_eq!(
                    trace.outcome,
                    CycleOutcome::InterruptEntryComplete { vector: 0x40 }
                );
            }
        }
    }
}

#[test]
fn divider_stop_entry_resets_once_then_no_cycle_polls_freeze_devices() {
    let mut s = machine(&[0x10, 0, 0x04]);
    let before = s.cpu_snapshot();
    assert!(matches!(
        s.step_instruction(0).unwrap(),
        InstructionStep::BudgetExhausted {
            mcycles: 0,
            ticks: 0
        }
    ));
    assert_eq!(s.cpu_snapshot(), before);
    assert_eq!(s.elapsed_ticks(), 0);
    assert_eq!(s.memory.read(0xFF04), 0xAB);
    let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
    assert_eq!(trace.outcome, CycleOutcome::StopEntered);
    assert_eq!((trace.record.tick_before, trace.record.tick_after), (0, 4));
    assert_eq!(s.memory.read(0xFF04), 0);
    let stopped = s.cpu_snapshot();
    for budget in [1, 64, u64::MAX] {
        assert!(matches!(
            s.step_instruction(budget).unwrap(),
            InstructionStep::Stopped {
                mcycles: 0,
                ticks: 0
            }
        ));
        assert!(matches!(
            s.step_machine_cycle().unwrap(),
            MachineStep::Stopped
        ));
        assert_eq!(s.memory.read(0xFF04), 0);
        assert_eq!(s.elapsed_ticks(), 4);
        assert_eq!(s.cpu_snapshot(), stopped);
    }
    // Test-only counter-state injection through Bus, while CPU remains stopped:
    // this reveals an accidental repeated STOP reset on no-cycle polling.
    for _ in 0..63 {
        s.memory.advance_mcycle();
    }
    assert_eq!(s.memory.read(0xFF04), 0);
    assert!(matches!(
        s.step_machine_cycle().unwrap(),
        MachineStep::Stopped
    ));
    assert_eq!(s.memory.read(0xFF04), 0, "poll advanced the hidden phase");
    s.memory.advance_mcycle();
    assert_eq!(s.memory.read(0xFF04), 1);
    assert!(matches!(
        s.step_machine_cycle().unwrap(),
        MachineStep::Stopped
    ));
    assert_eq!(s.memory.read(0xFF04), 1);
    assert_eq!(s.elapsed_ticks(), 4);
}

#[test]
fn divider_completion_error_advances_once_and_request_errors_preserve_time() {
    let mut bytes = vec![0; 13];
    bytes.push(0xD3);
    let mut s = machine(&bytes);
    for _ in 0..13 {
        s.step_instruction(1).unwrap();
    }
    assert_eq!(s.elapsed_ticks(), 52);
    assert_eq!(s.memory.read(0xFF04), 0xAB);
    match s.step_machine_cycle().unwrap_err() {
        SystemError::CpuCompletionError(CpuError::UnsupportedOpcode(fault), record) => {
            assert_eq!(fault.addr, 0x10D);
            assert_eq!((record.tick_before, record.tick_after), (52, 56));
            assert_eq!(record.action, CycleAction::Read { addr: 0x10D });
        }
        other => panic!("unexpected completion error {other:?}"),
    }
    assert_eq!(s.memory.read(0xFF04), 0xAC);
    let faulted = s.cpu_snapshot();
    for _ in 0..3 {
        assert!(matches!(
            s.step_machine_cycle(),
            Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
        ));
        assert_eq!(s.elapsed_ticks(), 56);
        assert_eq!(s.memory.read(0xFF04), 0xAC);
        assert_eq!(s.cpu_snapshot(), faulted);
    }
    assert!(matches!(
        s.step_instruction(0).unwrap(),
        InstructionStep::BudgetExhausted {
            mcycles: 0,
            ticks: 0
        }
    ));
    assert_eq!(s.elapsed_ticks(), 56);
}
