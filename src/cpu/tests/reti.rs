use crate::cpu::{
    CPU, CpuError, CycleAction, CycleKind, CycleOutcome, CycleResponse, InterruptSample,
    InterruptState, Registers, opcode::DecodedOpcode, registers::Flag,
};
use crate::response_support::expect_execute;

fn check(target: u16, sp: u16, pc: u16, flags: u8, ime: InterruptState, opcode: u8) {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    cpu.registers.sp = sp;
    cpu.registers.pc = pc;
    for (flag, bit) in [(Flag::Z, 128), (Flag::N, 64), (Flag::H, 32), (Flag::C, 16)] {
        cpu.registers.set_flag(flag, flags & bit != 0);
    }
    cpu.interrupt_state = ime;
    let original = cpu.snapshot();
    for cycle in 0..4 {
        let action = match cycle {
            0 => CycleAction::Read { addr: pc },
            1 => CycleAction::Read { addr: sp },
            2 => CycleAction::Read {
                addr: sp.wrapping_add(1),
            },
            _ => CycleAction::Idle,
        };
        let before = cpu.snapshot();
        let state = format!("{:?}", cpu.state);
        for _ in 0..2 {
            assert_eq!(cpu.request_cycle().map(expect_execute).unwrap(), action);
        }
        let expected_kind = if cycle == 3 {
            CycleKind::Idle
        } else {
            CycleKind::Read
        };
        for wrong in [
            CycleResponse::Read {
                value: 0xFF,
                pending_interrupts: 0,
                stop_wake_active: false,
            },
            CycleResponse::IdleDone,
            CycleResponse::WriteDone {
                interrupt_sample: InterruptSample {
                    enable_after_bus: 255,
                    flags_before_bus: 255,
                },
            },
        ] {
            if wrong.kind() == expected_kind {
                continue;
            }
            match cpu.complete_cycle(wrong).unwrap_err() {
                CpuError::UnexpectedCycleResponse(error) => assert_eq!(
                    (error.expected, error.received),
                    (expected_kind, wrong.kind())
                ),
                other => panic!("wrong error {other:?}"),
            }
            assert_eq!(cpu.snapshot(), before);
            assert_eq!(format!("{:?}", cpu.state), state);
            assert_eq!(cpu.interrupt_state, ime);
        }
        let response = match cycle {
            0 => CycleResponse::Read {
                value: opcode,
                pending_interrupts: 0,
                stop_wake_active: false,
            },
            1 => CycleResponse::Read {
                value: target as u8,
                pending_interrupts: 0,
                stop_wake_active: false,
            },
            2 => CycleResponse::Read {
                value: (target / 256) as u8,
                pending_interrupts: 0,
                stop_wake_active: false,
            },
            _ => CycleResponse::IdleDone,
        };
        assert_eq!(
            cpu.complete_cycle(response).unwrap(),
            if cycle == 3 {
                CycleOutcome::InstructionComplete
            } else {
                CycleOutcome::InProgress
            }
        );
        let mut expected = original;
        expected.pc = if cycle == 3 {
            target
        } else {
            pc.wrapping_add(1)
        };
        if cycle > 0 {
            expected.sp = sp.wrapping_add(cycle.min(2));
        }
        assert_eq!(cpu.snapshot(), expected);
        let expected_ime = if cycle == 3 && (opcode == 0xD9 || ime == InterruptState::Delayed) {
            InterruptState::Enabled
        } else {
            ime
        };
        assert_eq!(cpu.interrupt_state, expected_ime);
    }
    assert_eq!(
        cpu.request_cycle().map(expect_execute).unwrap(),
        CycleAction::Read { addr: target }
    );
}

#[test]
fn reti_exhausts_targets_and_stack_addresses_in_separate_sweeps() {
    assert_eq!(
        DecodedOpcode::from(0xD9),
        DecodedOpcode::ReturnFromInterrupt
    );
    for value in 0u16..=65535 {
        check(value, 0xFFFC, 0x100, 0xB0, InterruptState::Disabled, 0xD9);
        check(0xA53C, value, 0xFFFF, 0, InterruptState::Disabled, 0xD9);
    }
}

#[test]
fn reti_and_ret_preserve_flags_and_differ_only_in_explicit_ime_effect() {
    for flags in 0..16 {
        for ime in [
            InterruptState::Disabled,
            InterruptState::Delayed,
            InterruptState::Enabled,
        ] {
            for opcode in [0xC9, 0xD9] {
                check(0x1234, 0xFFFF, 0xFFFF, flags * 16, ime, opcode);
            }
        }
    }
}

#[test]
fn reti_captures_return_mode_stack_address_and_low_byte() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    cpu.registers.sp = 0xFFFF;
    cpu.complete_cycle(CycleResponse::Read {
        value: 0xD9,
        pending_interrupts: 0,
        stop_wake_active: false,
    })
    .unwrap();
    cpu.registers.sp = 0x4321;
    assert_eq!(
        cpu.request_cycle().map(expect_execute).unwrap(),
        CycleAction::Read { addr: 0xFFFF }
    );
    cpu.complete_cycle(CycleResponse::Read {
        value: 0x34,
        pending_interrupts: 0,
        stop_wake_active: false,
    })
    .unwrap();
    cpu.registers.sp = 0xBEEF;
    assert_eq!(
        cpu.request_cycle().map(expect_execute).unwrap(),
        CycleAction::Read { addr: 0 }
    );
    cpu.complete_cycle(CycleResponse::Read {
        value: 0x12,
        pending_interrupts: 0,
        stop_wake_active: false,
    })
    .unwrap();
    assert_eq!(cpu.snapshot().sp, 1);
    assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
    cpu.complete_cycle(CycleResponse::IdleDone).unwrap();
    assert_eq!(cpu.snapshot().pc, 0x1234);
    assert_eq!(cpu.interrupt_state, InterruptState::Enabled);
}
