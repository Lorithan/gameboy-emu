use crate::cpu::{
    CPU, CpuError, CycleAction, CycleOutcome, CycleRequest, CycleResponse, ExecutionState,
    InterruptSample, InterruptState, Registers, opcode::DecodedOpcode,
};

fn read(value: u8, pending_interrupts: u8) -> CycleResponse {
    CycleResponse::Read {
        value,
        pending_interrupts,
        stop_wake_active: false,
    }
}

fn responses() -> [CycleResponse; 3] {
    [
        read(0x10, 0xFF),
        CycleResponse::WriteDone {
            interrupt_sample: InterruptSample {
                enable_after_bus: 0xFF,
                flags_before_bus: 0xFF,
            },
        },
        CycleResponse::IdleDone,
    ]
}

#[test]
fn stop_entry_preserves_registers_skips_padding_and_matures_ei() {
    assert_eq!(DecodedOpcode::from(0x10), DecodedOpcode::Stop);
    for pc in [0u16, 0x100, 0x7FFF, 0xFFFE, 0xFFFF] {
        for flags in (0u16..=0xF0).step_by(16) {
            for ime in [
                InterruptState::Disabled,
                InterruptState::Delayed,
                InterruptState::Enabled,
            ] {
                // Pending-at-entry values exercise our declared deterministic
                // fallback, not the conditional hardware STOP branches.
                for pending in [0, 1, 31, 0xE0, 0xFF] {
                    let mut regs = Registers::dmg_post_boot(1);
                    regs.set_af(0xA500 | flags);
                    regs.set_bc(0x1234);
                    regs.set_de(0x5678);
                    regs.set_hl(0x9ABC);
                    regs.sp = 0;
                    regs.pc = pc;
                    let mut cpu = CPU::from_registers(regs);
                    cpu.interrupt_state = ime;
                    let mut expected = cpu.snapshot();
                    for _ in 0..2 {
                        assert_eq!(
                            cpu.request_cycle().unwrap(),
                            CycleRequest::Execute(CycleAction::Read { addr: pc })
                        );
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    for wrong in &responses()[1..] {
                        assert!(matches!(
                            cpu.complete_cycle(*wrong),
                            Err(CpuError::UnexpectedCycleResponse(_))
                        ));
                        assert_eq!(cpu.snapshot(), expected);
                        assert_eq!(cpu.interrupt_state, ime);
                        assert!(matches!(cpu.state, ExecutionState::FetchOpcode));
                        assert!(!cpu.suppress_next_opcode_increment);
                    }
                    assert_eq!(
                        cpu.complete_cycle(read(0x10, pending)).unwrap(),
                        CycleOutcome::StopEntered
                    );
                    expected.pc = pc.wrapping_add(2);
                    assert_eq!(cpu.snapshot(), expected, "STOP at {pc:04X}");
                    assert!(matches!(cpu.state, ExecutionState::Stopped));
                    assert_eq!(cpu.request_cycle().unwrap(), CycleRequest::Stopped);
                    assert!(!cpu.suppress_next_opcode_increment);
                    assert_eq!(
                        cpu.interrupt_state,
                        if ime == InterruptState::Delayed {
                            InterruptState::Enabled
                        } else {
                            ime
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn stopped_queries_and_all_completions_preserve_state_and_ignore_interrupts() {
    for pending in 0u8..=255 {
        for ime in [
            InterruptState::Disabled,
            InterruptState::Delayed,
            InterruptState::Enabled,
        ] {
            for suppressed in [false, true] {
                let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
                cpu.state = ExecutionState::Stopped;
                cpu.interrupt_state = ime;
                cpu.suppress_next_opcode_increment = suppressed;
                let before = cpu.snapshot();
                for _ in 0..2 {
                    assert_eq!(cpu.request_cycle().unwrap(), CycleRequest::Stopped);
                    cpu.wake_if_pending(pending);
                    assert!(!cpu.try_accept_interrupt(pending));
                    for response in responses() {
                        assert!(matches!(
                            cpu.complete_cycle(response),
                            Err(CpuError::NoCycleRequested)
                        ));
                        // Also cover the inner match's defensive arm directly.
                        assert!(matches!(
                            cpu.complete_cycle_inner(response),
                            Err(CpuError::NoCycleRequested)
                        ));
                        assert_eq!(cpu.snapshot(), before);
                        assert_eq!(cpu.interrupt_state, ime);
                        assert_eq!(cpu.suppress_next_opcode_increment, suppressed);
                        assert!(matches!(cpu.state, ExecutionState::Stopped));
                    }
                }
            }
        }
    }
}

#[test]
fn stop_composes_with_one_shot_fetch_suppression_in_the_declared_model() {
    // Internal model consistency only; hardware HALT/STOP interaction is deferred.
    for pc in [0u16, 0x100, 0xFFFE, 0xFFFF] {
        let mut regs = Registers::dmg_post_boot(1);
        regs.pc = pc;
        let mut cpu = CPU::from_registers(regs);
        cpu.suppress_next_opcode_increment = true;
        let mut expected = cpu.snapshot();
        assert!(matches!(
            cpu.complete_cycle(CycleResponse::IdleDone),
            Err(CpuError::UnexpectedCycleResponse(_))
        ));
        assert!(cpu.suppress_next_opcode_increment);
        assert_eq!(cpu.snapshot(), expected);
        assert_eq!(
            cpu.complete_cycle(read(0x10, 31)).unwrap(),
            CycleOutcome::StopEntered
        );
        // Suppressed opcode increment + one padding skip, not an absolute P+2.
        expected.pc = pc.wrapping_add(1);
        assert_eq!(cpu.snapshot(), expected);
        assert!(!cpu.suppress_next_opcode_increment);
        assert!(matches!(cpu.state, ExecutionState::Stopped));
    }
}
