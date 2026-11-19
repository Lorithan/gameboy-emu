use crate::cpu::{
    CPU, CpuError, CycleAction, CycleOutcome, ExecutionState, InterruptState, Registers,
};
use crate::response_support::expect_execute;
use crate::response_support::response;

fn entry_response(action: CycleAction, mask: u8) -> crate::cpu::CycleResponse {
    match action {
        CycleAction::Write { .. } => crate::cpu::CycleResponse::WriteDone {
            interrupt_sample: crate::cpu::InterruptSample {
                enable_after_bus: mask,
                flags_before_bus: mask,
            },
        },
        CycleAction::Idle => crate::cpu::CycleResponse::IdleDone,
        _ => panic!("entry must not read"),
    }
}

#[test]
fn interrupt_acceptance_masks_and_rejects_without_mutation() {
    for pending in 0u8..=255 {
        for ime in [
            InterruptState::Disabled,
            InterruptState::Delayed,
            InterruptState::Enabled,
        ] {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
            cpu.interrupt_state = ime;
            let before = cpu.snapshot();
            let accepted = cpu.try_accept_interrupt(pending);
            if ime == InterruptState::Enabled && pending % 32 != 0 {
                assert!(accepted);
                assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
                assert!(matches!(cpu.state, ExecutionState::InterruptEntry { .. }));
                assert!(!cpu.try_accept_interrupt(31));
            } else {
                assert!(!accepted);
                assert!(matches!(cpu.state, ExecutionState::FetchOpcode));
                assert_eq!(cpu.interrupt_state, ime);
            }
            assert_eq!(cpu.snapshot(), before);
        }
    }
    // Reject while Enabled even in the middle of an instruction or a fault.
    for opcode in [0x3E, 0xCB, 0xCD, 0xC5, 0xD3] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
        let _ = cpu.complete_cycle(response(
            cpu.request_cycle().map(expect_execute),
            Some(opcode),
        ));
        cpu.interrupt_state = InterruptState::Enabled;
        let snapshot = cpu.snapshot();
        let state = format!("{:?}", cpu.state);
        assert!(!cpu.try_accept_interrupt(31));
        assert_eq!(cpu.snapshot(), snapshot);
        assert_eq!(format!("{:?}", cpu.state), state);
        assert_eq!(cpu.interrupt_state, InterruptState::Enabled);
        if opcode == 0xD3 {
            assert!(matches!(
                cpu.request_cycle().map(expect_execute),
                Err(CpuError::UnsupportedOpcode(_))
            ));
        }
    }
}

fn check_entry(pc: u16, sp: u16, mask: u8, initial_flags: u8) {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
    for (flag, bit) in [
        (crate::cpu::registers::Flag::Z, 0x80),
        (crate::cpu::registers::Flag::N, 0x40),
        (crate::cpu::registers::Flag::H, 0x20),
        (crate::cpu::registers::Flag::C, 0x10),
    ] {
        cpu.registers.set_flag(flag, initial_flags & bit != 0);
    }
    cpu.registers.pc = pc;
    cpu.registers.sp = sp;
    cpu.interrupt_state = InterruptState::Enabled;
    let original = cpu.snapshot();
    assert!(cpu.try_accept_interrupt(mask));
    let vector = 0x40 + mask.trailing_zeros() as u16 * 8;
    let actions = [
        CycleAction::Idle,
        CycleAction::Idle,
        CycleAction::Write {
            addr: sp.wrapping_sub(1),
            value: (pc / 256) as u8,
        },
        CycleAction::Write {
            addr: sp.wrapping_sub(2),
            value: pc as u8,
        },
        CycleAction::Idle,
    ];
    for (cycle, action) in actions.into_iter().enumerate() {
        let before = cpu.snapshot();
        for _ in 0..2 {
            assert_eq!(cpu.request_cycle().map(expect_execute).unwrap(), action);
            assert_eq!(cpu.snapshot(), before);
            assert!(!cpu.try_accept_interrupt(31));
        }
        let outcome = cpu
            .complete_cycle(entry_response(
                cpu.request_cycle().map(expect_execute).unwrap(),
                mask,
            ))
            .unwrap();
        assert_eq!(
            outcome,
            if cycle == 4 {
                CycleOutcome::InterruptEntryComplete { vector }
            } else if cycle == 3 {
                CycleOutcome::InterruptSelected {
                    interrupt: crate::memory::Interrupt::from_selection(mask).unwrap(),
                }
            } else {
                CycleOutcome::InProgress
            }
        );
        let mut expected = original;
        if cycle >= 2 {
            expected.sp = sp.wrapping_sub(if cycle == 2 { 1 } else { 2 });
        }
        if cycle == 4 {
            expected.pc = vector;
        }
        assert_eq!(cpu.snapshot(), expected);
        assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
    }
    assert_eq!(
        cpu.request_cycle().map(expect_execute).unwrap(),
        CycleAction::Read { addr: vector }
    );
    assert_eq!(
        cpu.complete_cycle(response(cpu.request_cycle().map(expect_execute), Some(0)))
            .unwrap(),
        CycleOutcome::InstructionComplete
    );
    assert_eq!(cpu.snapshot().pc, vector + 1);
}

#[test]
fn interrupt_entry_exhausts_return_addresses_and_stack_wrapping_separately() {
    for value in 0u16..=65535 {
        check_entry(value, 0xFFFE, 4, 0xE7);
        check_entry(0xA53C, value, 4, 0);
    }
    for mask in [1, 2, 4, 8, 16] {
        for flags in 0..16 {
            check_entry(0x1234, 0xFFFE, mask, flags * 16);
        }
        for sp in [0, 1, 0x8000, 0xC002, 0xE002, 0xFF10, 0xFF80, 0xFFFF] {
            check_entry(0x1234, sp, mask, 0xE7);
        }
    }
}

#[test]
fn interrupt_entry_uses_captured_pc_and_stack_addresses() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    cpu.registers.pc = 0xA53C;
    cpu.registers.sp = 0;
    cpu.interrupt_state = InterruptState::Enabled;
    assert!(cpu.try_accept_interrupt(8));
    cpu.registers.pc = 0xDEAD;
    cpu.registers.sp = 0xBEEF;
    for _ in 0..2 {
        assert_eq!(
            cpu.complete_cycle(entry_response(
                cpu.request_cycle().map(expect_execute).unwrap(),
                8
            ))
            .unwrap(),
            CycleOutcome::InProgress
        );
    }
    assert_eq!(
        cpu.request_cycle().map(expect_execute).unwrap(),
        CycleAction::Write {
            addr: 0xFFFF,
            value: 0xA5
        }
    );
    cpu.complete_cycle(entry_response(
        cpu.request_cycle().map(expect_execute).unwrap(),
        8,
    ))
    .unwrap();
    cpu.registers.sp = 0x4321;
    assert_eq!(
        cpu.request_cycle().map(expect_execute).unwrap(),
        CycleAction::Write {
            addr: 0xFFFE,
            value: 0x3C
        }
    );
    cpu.complete_cycle(entry_response(
        cpu.request_cycle().map(expect_execute).unwrap(),
        8,
    ))
    .unwrap();
    assert_eq!(cpu.snapshot().sp, 0xFFFE);
    assert_eq!(cpu.snapshot().pc, 0xDEAD);
    assert_eq!(
        cpu.complete_cycle(entry_response(
            cpu.request_cycle().map(expect_execute).unwrap(),
            8
        ))
        .unwrap(),
        CycleOutcome::InterruptEntryComplete { vector: 0x58 }
    );
    assert_eq!(cpu.snapshot().pc, 0x58);
}

#[test]
fn late_selection_exhausts_ie_if_samples_and_preserves_cancellation_state() {
    use crate::cpu::{CycleResponse, InterruptSample};
    for enable in 0u8..=255 {
        for flags in 0u8..=255 {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
            cpu.interrupt_state = InterruptState::Enabled;
            assert!(cpu.try_accept_interrupt(4)); // Initial source is only a gate.
            for _ in 0..2 {
                cpu.complete_cycle(CycleResponse::IdleDone).unwrap();
            }
            cpu.complete_cycle(CycleResponse::WriteDone {
                interrupt_sample: InterruptSample {
                    enable_after_bus: enable,
                    flags_before_bus: !flags,
                },
            })
            .unwrap();
            let outcome = cpu
                .complete_cycle(CycleResponse::WriteDone {
                    interrupt_sample: InterruptSample {
                        enable_after_bus: !enable,
                        flags_before_bus: flags,
                    },
                })
                .unwrap();
            let eligible = enable & flags & 31;
            let vector = if eligible == 0 {
                assert_eq!(outcome, CycleOutcome::InterruptSelectionCancelled);
                0
            } else {
                let index = (0..5).find(|i| eligible & (1 << i) != 0).unwrap();
                let vector = 0x40 + index * 8;
                match outcome {
                    CycleOutcome::InterruptSelected { interrupt } => {
                        assert_eq!(interrupt.vector_address(), vector)
                    }
                    other => panic!("unexpected outcome {other:?}"),
                }
                vector
            };
            assert_eq!(cpu.snapshot().pc, 0x100);
            assert_eq!(cpu.snapshot().sp, 0xFFFC);
            let before = cpu.snapshot();
            assert!(matches!(
                cpu.complete_cycle(CycleResponse::Read {
                    value: 0,
                    pending_interrupts: 0,
                    stop_wake_active: false,
                }),
                Err(CpuError::UnexpectedCycleResponse(_))
            ));
            assert_eq!(cpu.snapshot(), before);
            assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
            assert_eq!(
                cpu.complete_cycle(CycleResponse::IdleDone).unwrap(),
                if eligible == 0 {
                    CycleOutcome::InterruptEntryCancelled
                } else {
                    CycleOutcome::InterruptEntryComplete { vector }
                }
            );
            assert_eq!(cpu.snapshot().pc, vector);
            assert_eq!(cpu.snapshot().af, 0x01B0);
            assert_eq!(
                cpu.request_cycle().map(expect_execute).unwrap(),
                CycleAction::Read { addr: vector }
            );
        }
    }
}
