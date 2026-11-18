use crate::cpu::{
    CPU, CpuError, CycleAction, CycleKind, CycleOutcome, CycleResponse, InterruptSample,
    InterruptState, Registers,
};

fn responses() -> [CycleResponse; 3] {
    [
        CycleResponse::Read(0xA5),
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
    let action = cpu.request_cycle().unwrap();
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
        assert_eq!(cpu.request_cycle().unwrap(), action);
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
                    let response = match cpu.request_cycle().unwrap() {
                        CycleAction::Read { .. } => CycleResponse::Read(data),
                        CycleAction::Write { .. } => responses()[1],
                        CycleAction::Idle => CycleResponse::IdleDone,
                    };
                    match cpu.complete_cycle(response) {
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
            let response = match cpu.request_cycle().unwrap() {
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
        cpu.complete_cycle(CycleResponse::Read(0x77)).unwrap(); // LD (HL),A
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
        if let CycleResponse::Read(value) = response {
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
