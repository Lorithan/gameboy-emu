use crate::cpu::{
    CPU, CpuError, CycleAction, CycleKind, CycleOutcome, CycleResponse, InterruptSample,
    InterruptState, Registers,
};
use crate::response_support::expect_execute;

fn responses() -> [CycleResponse; 3] {
    [
        CycleResponse::Read {
            value: 0xA5,
            pending_interrupts: 0,
            stop_wake_active: false,
        },
        CycleResponse::WriteDone {
            interrupt_sample: InterruptSample {
                enable_after_bus: 0xFF,
                flags_before_bus: 0xE0,
            },
        },
        CycleResponse::IdleDone,
    ]
}

fn reject_wrong_kinds(cpu: &mut CPU) {
    let action = cpu.request_cycle().map(expect_execute).unwrap();
    let expected = match action {
        CycleAction::Read { .. } => CycleKind::Read,
        CycleAction::Write { .. } => CycleKind::Write,
        CycleAction::Idle => CycleKind::Idle,
    };
    let snapshot = cpu.snapshot();
    let state = format!("{:?}", cpu.state);
    let ime = cpu.interrupt_state;
    for response in responses() {
        if response.kind() == expected {
            continue;
        }
        match cpu.complete_cycle(response).unwrap_err() {
            CpuError::UnexpectedCycleResponse(error) => {
                assert_eq!(error.expected, expected);
                assert_eq!(error.received, response.kind());
            }
            other => panic!("unexpected error {other:?}"),
        }
        assert_eq!(cpu.snapshot(), snapshot);
        assert_eq!(format!("{:?}", cpu.state), state);
        assert_eq!(cpu.interrupt_state, ime);
        assert_eq!(cpu.request_cycle().map(expect_execute).unwrap(), action);
    }
}

#[test]
fn response_mismatches_preserve_every_instruction_phase_and_ime() {
    for cb in [false, true] {
        for opcode in 0u8..=255 {
            for ime in [
                InterruptState::Disabled,
                InterruptState::Delayed,
                InterruptState::Enabled,
            ] {
                let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
                cpu.interrupt_state = ime;
                let mut finished = false;
                for cycle in 0..8 {
                    reject_wrong_kinds(&mut cpu);
                    let data = if cycle == 0 {
                        if cb { 0xCB } else { opcode }
                    } else if cb && cycle == 1 {
                        opcode
                    } else {
                        0
                    };
                    let response = match cpu.request_cycle().map(expect_execute).unwrap() {
                        CycleAction::Read { .. } => CycleResponse::Read {
                            value: data,
                            pending_interrupts: 0,
                            stop_wake_active: false,
                        },
                        CycleAction::Write { .. } => responses()[1],
                        CycleAction::Idle => CycleResponse::IdleDone,
                    };
                    match cpu.complete_cycle(response) {
                        Ok(CycleOutcome::StopEntered) => {
                            assert!(!cb);
                            assert_eq!(opcode, 0x10);
                            assert!(matches!(
                                cpu.request_cycle(),
                                Ok(crate::cpu::CycleRequest::Stopped)
                            ));
                            finished = true;
                            break;
                        }
                        Ok(CycleOutcome::InstructionComplete) => {
                            finished = true;
                            break;
                        }
                        Ok(CycleOutcome::InProgress) => {}
                        Err(CpuError::UnsupportedOpcode(original)) => {
                            assert!(!cb);
                            for response in responses() {
                                match cpu.complete_cycle(response).unwrap_err() {
                                    CpuError::UnsupportedOpcode(fault) => {
                                        assert_eq!(fault.opcode, original.opcode);
                                        assert_eq!(fault.addr, original.addr);
                                    }
                                    other => panic!("fault was replaced: {other:?}"),
                                }
                            }
                            finished = true;
                            break;
                        }
                        other => panic!("unexpected completion {other:?}"),
                    }
                }
                assert!(finished, "instruction {opcode:02X}, CB={cb} did not finish");
            }
        }
    }
}

#[test]
fn response_mismatches_preserve_each_interrupt_entry_phase() {
    for mask in [1, 2, 4, 8, 16] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
        cpu.interrupt_state = InterruptState::Enabled;
        assert!(cpu.try_accept_interrupt(mask));
        let source = crate::memory::Interrupt::from_selection(mask).unwrap();
        for cycle in 0..5 {
            reject_wrong_kinds(&mut cpu);
            let response = match cpu.request_cycle().map(expect_execute).unwrap() {
                CycleAction::Write { .. } => CycleResponse::WriteDone {
                    interrupt_sample: InterruptSample {
                        enable_after_bus: mask,
                        flags_before_bus: mask,
                    },
                },
                CycleAction::Idle => CycleResponse::IdleDone,
                other => panic!("unexpected entry action {other:?}"),
            };
            assert_eq!(
                cpu.complete_cycle(response).unwrap(),
                if cycle == 4 {
                    CycleOutcome::InterruptEntryComplete {
                        vector: source.vector_address(),
                    }
                } else if cycle == 3 {
                    CycleOutcome::InterruptSelected { interrupt: source }
                } else {
                    CycleOutcome::InProgress
                }
            );
        }
    }
}

#[test]
fn ordinary_write_completion_ignores_interrupt_sample_values() {
    let mut expected = None;
    for sample in [
        InterruptSample {
            enable_after_bus: 0,
            flags_before_bus: 0,
        },
        InterruptSample {
            enable_after_bus: 255,
            flags_before_bus: 255,
        },
        InterruptSample {
            enable_after_bus: 0xA5,
            flags_before_bus: 0x5A,
        },
    ] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
        cpu.complete_cycle(CycleResponse::Read {
            value: 0x77,
            pending_interrupts: 0,
            stop_wake_active: false,
        })
        .unwrap(); // LD (HL),A
        assert_eq!(
            cpu.complete_cycle(CycleResponse::WriteDone {
                interrupt_sample: sample
            })
            .unwrap(),
            CycleOutcome::InstructionComplete
        );
        if let Some(snapshot) = expected {
            assert_eq!(cpu.snapshot(), snapshot);
        }
        expected = Some(cpu.snapshot());
    }
    for response in responses() {
        if let CycleResponse::Read { value, .. } = response {
            assert_eq!(response.read_value().unwrap(), value);
        } else {
            let error = response.read_value().unwrap_err();
            assert_eq!(
                (error.expected, error.received),
                (CycleKind::Read, response.kind())
            );
        }
    }
}

#[test]
fn stop_wake_read_accessor_preserves_both_levels_and_rejects_other_response_kinds() {
    for value in 0u8..=255 {
        for pending_interrupts in 0u8..=255 {
            for stop_wake_active in [false, true] {
                let response = CycleResponse::Read {
                    value,
                    pending_interrupts,
                    stop_wake_active,
                };
                let before = response;
                for _ in 0..2 {
                    assert_eq!(response.stop_wake_active(), Ok(stop_wake_active));
                    assert_eq!(response.read_value(), Ok(value));
                    assert_eq!(response.pending_interrupts(), Ok(pending_interrupts));
                    assert_eq!(response.kind(), CycleKind::Read);
                    assert_eq!(response, before);
                }
            }
        }
    }
    for response in [responses()[1], CycleResponse::IdleDone] {
        let before = response;
        let error = response.stop_wake_active().unwrap_err();
        assert_eq!(error.expected, CycleKind::Read);
        assert_eq!(error.received, response.kind());
        assert_eq!(response, before);
    }
}

#[test]
fn ordinary_read_cycles_ignore_pending_masks_and_wake_levels_without_accepting_interrupts() {
    for pending in 0u8..=255 {
        for stop_wake_active in [false, true] {
            for ime in [
                InterruptState::Disabled,
                InterruptState::Delayed,
                InterruptState::Enabled,
            ] {
                let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
                cpu.interrupt_state = ime;
                // Immediate operand, indirect data, and both CB fetch stages.
                let mut reference = cpu.clone();
                for value in [0x3E, 0x42, 0x7E, 0xA5, 0xCB, 0x00] {
                    let response = CycleResponse::Read {
                        value,
                        pending_interrupts: pending,
                        stop_wake_active,
                    };
                    assert_eq!(response.kind(), CycleKind::Read);
                    assert_eq!(response.read_value().unwrap(), value);
                    assert_eq!(
                        cpu.request_cycle().map(expect_execute).unwrap(),
                        reference.request_cycle().map(expect_execute).unwrap()
                    );
                    assert_eq!(
                        cpu.complete_cycle(response).unwrap(),
                        reference
                            .complete_cycle(CycleResponse::Read {
                                value,
                                pending_interrupts: 0,
                                stop_wake_active: false,
                            })
                            .unwrap()
                    );
                    assert_eq!(cpu.snapshot(), reference.snapshot());
                    assert_eq!(cpu.interrupt_state, reference.interrupt_state);
                    assert_eq!(format!("{:?}", cpu.state), format!("{:?}", reference.state));
                }
            }
        }
    }
}
