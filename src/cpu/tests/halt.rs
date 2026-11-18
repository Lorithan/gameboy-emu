use crate::cpu::{
    CPU, CpuError, CycleAction, CycleOutcome, CycleResponse, ExecutionState, InterruptSample,
    InterruptState, Registers, opcode::DecodedOpcode, registers::Flag,
};

fn read(value: u8, pending: u8) -> CycleResponse {
    CycleResponse::Read {
        value,
        pending_interrupts: pending,
    }
}

#[test]
fn halt_decision_exhausts_pending_flags_ime_and_pc_wrap() {
    assert_eq!(DecodedOpcode::from(0x76), DecodedOpcode::Halt);
    for pending in 0u8..=255 {
        for flags in 0..16 {
            for ime in [
                InterruptState::Disabled,
                InterruptState::Delayed,
                InterruptState::Enabled,
            ] {
                let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
                cpu.registers.pc = 0xFFFF;
                for (f, bit) in [(Flag::Z, 128), (Flag::N, 64), (Flag::H, 32), (Flag::C, 16)] {
                    cpu.registers.set_flag(f, (flags * 16) & bit != 0);
                }
                cpu.interrupt_state = ime;
                let mut expected = cpu.snapshot();
                expected.pc = 0;
                for wrong in [
                    CycleResponse::IdleDone,
                    CycleResponse::WriteDone {
                        interrupt_sample: InterruptSample {
                            enable_after_bus: 255,
                            flags_before_bus: 255,
                        },
                    },
                ] {
                    let before = cpu.snapshot();
                    assert!(matches!(
                        cpu.complete_cycle(wrong),
                        Err(CpuError::UnexpectedCycleResponse(_))
                    ));
                    assert_eq!(cpu.snapshot(), before);
                    assert_eq!(cpu.interrupt_state, ime);
                    assert!(!cpu.suppress_next_opcode_increment);
                }
                assert_eq!(
                    cpu.complete_cycle(read(0x76, pending)).unwrap(),
                    CycleOutcome::InstructionComplete
                );
                assert_eq!(cpu.snapshot(), expected);
                let bug = ime != InterruptState::Enabled && pending & 31 != 0;
                assert_eq!(cpu.suppress_next_opcode_increment, bug);
                assert_eq!(matches!(cpu.state, ExecutionState::Halted), !bug);
                assert_eq!(
                    cpu.interrupt_state,
                    if ime == InterruptState::Delayed {
                        InterruptState::Enabled
                    } else {
                        ime
                    }
                );
                for _ in 0..2 {
                    assert_eq!(
                        cpu.request_cycle().unwrap(),
                        if bug {
                            CycleAction::Read { addr: 0 }
                        } else {
                            CycleAction::Idle
                        }
                    );
                    assert_eq!(cpu.snapshot(), expected);
                }
                if !bug {
                    for wrong in [
                        read(0, 31),
                        CycleResponse::WriteDone {
                            interrupt_sample: InterruptSample {
                                enable_after_bus: 31,
                                flags_before_bus: 31,
                            },
                        },
                    ] {
                        assert!(matches!(
                            cpu.complete_cycle(wrong),
                            Err(CpuError::UnexpectedCycleResponse(_))
                        ));
                        assert_eq!(cpu.snapshot(), expected);
                        assert!(matches!(cpu.state, ExecutionState::Halted));
                        assert!(!cpu.suppress_next_opcode_increment);
                    }
                    for _ in 0..2 {
                        assert_eq!(
                            cpu.complete_cycle(CycleResponse::IdleDone).unwrap(),
                            CycleOutcome::InProgress
                        );
                        assert_eq!(cpu.snapshot(), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn wake_uses_eligible_mask_independent_of_ime_and_preserves_other_states() {
    for pending in 0u8..=255 {
        for ime in [InterruptState::Disabled, InterruptState::Enabled] {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
            cpu.interrupt_state = ime;
            cpu.complete_cycle(read(0x76, 0)).unwrap();
            let before = cpu.snapshot();
            cpu.wake_if_pending(pending);
            assert_eq!(
                matches!(cpu.state, ExecutionState::FetchOpcode),
                pending & 31 != 0
            );
            assert_eq!(cpu.snapshot(), before);
            assert_eq!(cpu.interrupt_state, ime);
        }
        for opcode in [0x3E, 0xCB, 0xD3] {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
            let _ = cpu.complete_cycle(read(opcode, 0));
            let state = format!("{:?}", cpu.state);
            let before = cpu.snapshot();
            cpu.wake_if_pending(pending);
            assert_eq!(format!("{:?}", cpu.state), state);
            assert_eq!(cpu.snapshot(), before);
        }
    }
}

#[test]
fn suppression_retries_and_rejected_acceptance_do_not_consume_latch() {
    for pc in [0u16, 0x100, 0xFFFF] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
        cpu.registers.pc = pc;
        cpu.interrupt_state = InterruptState::Delayed;
        cpu.complete_cycle(read(0x76, 4)).unwrap();
        let before = cpu.snapshot();
        assert!(cpu.suppress_next_opcode_increment);
        for mask in [0, 0xE0] {
            assert!(!cpu.try_accept_interrupt(mask));
            assert_eq!(cpu.snapshot(), before);
            assert!(cpu.suppress_next_opcode_increment);
        }
        assert!(matches!(
            cpu.complete_cycle(CycleResponse::IdleDone),
            Err(CpuError::UnexpectedCycleResponse(_))
        ));
        assert_eq!(cpu.snapshot(), before);
        assert!(cpu.suppress_next_opcode_increment);
        cpu.complete_cycle(read(0x3E, 0)).unwrap();
        assert!(!cpu.suppress_next_opcode_increment);
        assert_eq!(cpu.snapshot().pc, before.pc);
        cpu.complete_cycle(read(0x3E, 0)).unwrap();
        assert_eq!(cpu.snapshot().af >> 8, 0x3E);
        assert_eq!(cpu.snapshot().pc, before.pc.wrapping_add(1));
    }
}

#[test]
fn ei_halt_acceptance_rewinds_once_and_cancellation_cannot_leak_latch() {
    for pc in [0u16, 0x100, 0xFFFF] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
        cpu.registers.pc = pc;
        cpu.interrupt_state = InterruptState::Delayed;
        cpu.complete_cycle(read(0x76, 4)).unwrap();
        assert!(cpu.try_accept_interrupt(4));
        assert!(!cpu.suppress_next_opcode_increment);
        assert_eq!(cpu.snapshot().pc, pc);
        for _ in 0..2 {
            cpu.complete_cycle(CycleResponse::IdleDone).unwrap();
        }
        let sample = CycleResponse::WriteDone {
            interrupt_sample: InterruptSample {
                enable_after_bus: 0,
                flags_before_bus: 4,
            },
        };
        cpu.complete_cycle(sample).unwrap();
        assert_eq!(
            cpu.complete_cycle(sample).unwrap(),
            CycleOutcome::InterruptSelectionCancelled
        );
        assert_eq!(
            cpu.complete_cycle(CycleResponse::IdleDone).unwrap(),
            CycleOutcome::InterruptEntryCancelled
        );
        assert_eq!(cpu.snapshot().pc, 0);
        assert!(!cpu.suppress_next_opcode_increment);
        cpu.complete_cycle(read(0, 0)).unwrap();
        assert_eq!(cpu.snapshot().pc, 1);
    }
}
