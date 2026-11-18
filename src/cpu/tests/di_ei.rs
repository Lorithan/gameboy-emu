use crate::outcome_support::instruction_complete;
use crate::response_support::response;

use super::{CPU, CpuError, CycleAction, Registers};
use crate::cpu::{InterruptState, opcode::DecodedOpcode};

#[test]
fn di_ei_decode_and_cpu_startup() {
    assert_eq!(DecodedOpcode::from(0xF3), DecodedOpcode::DisableInterrupts);
    assert_eq!(DecodedOpcode::from(0xFB), DecodedOpcode::EnabledInterrupts);
    for checksum in [0, 1, 0xE7] {
        let cpu = CPU::from_registers(Registers::dmg_post_boot(checksum));
        assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
    }
}

#[test]
fn di_ei_preserve_registers_flags_and_wrap_pc_from_every_ime_state() {
    for opcode in [0xF3, 0xFB] {
        for state in [
            InterruptState::Disabled,
            InterruptState::Delayed,
            InterruptState::Enabled,
        ] {
            for flags in (0u8..=0xF0).step_by(16) {
                for pc in [0x100u16, 0xFFFE, 0xFFFF] {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.set_af(0xA500 | u16::from(flags));
                    registers.set_bc(0x1234);
                    registers.set_de(0x5678);
                    registers.set_hl(0x9ABC);
                    registers.pc = pc;
                    let mut cpu = CPU::from_registers(registers);
                    cpu.interrupt_state = state;
                    let mut expected = cpu.snapshot();
                    for _ in 0..2 {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pc)
                        );
                    }
                    assert!(
                        matches!(cpu.complete_cycle(response(cpu.request_cycle(), None)).map(instruction_complete), Err(CpuError::UnexpectedCycleResponse(_)) if cpu.request_cycle().unwrap() == (CycleAction::Read { addr: pc }))
                    );
                    assert_eq!(cpu.snapshot(), expected);
                    assert_eq!(cpu.interrupt_state, state);
                    assert!(
                        cpu.complete_cycle(response(cpu.request_cycle(), Some(opcode)))
                            .map(instruction_complete)
                            .unwrap()
                    );
                    expected.pc = pc.wrapping_add(1);
                    assert_eq!(cpu.snapshot(), expected);
                    let expected_state = if opcode == 0xF3 {
                        InterruptState::Disabled
                    } else if state == InterruptState::Disabled {
                        InterruptState::Delayed
                    } else {
                        InterruptState::Enabled
                    };
                    assert_eq!(cpu.interrupt_state, expected_state);
                }
            }
        }
    }
}

#[test]
fn di_ei_sequences_match_independent_pending_enable_model() {
    // All seven-instruction sequences over NOP/DI/EI from each initial IME state.
    for initial in [
        InterruptState::Disabled,
        InterruptState::Delayed,
        InterruptState::Enabled,
    ] {
        for sequence in 0..3usize.pow(7) {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
            cpu.interrupt_state = initial;
            let mut enabled = initial == InterruptState::Enabled;
            let mut pending = initial == InterruptState::Delayed;
            let mut digits = sequence;
            for _ in 0..7 {
                let opcode = [0, 0xF3, 0xFB][digits % 3];
                digits /= 3;
                let due = pending;
                if opcode == 0xF3 {
                    enabled = false;
                    pending = false;
                } else {
                    if due {
                        enabled = true;
                        pending = false;
                    }
                    if opcode == 0xFB && !enabled {
                        pending = true;
                    }
                }
                assert!(
                    cpu.complete_cycle(response(cpu.request_cycle(), Some(opcode)))
                        .map(instruction_complete)
                        .unwrap()
                );
                let expected = if enabled {
                    InterruptState::Enabled
                } else if pending {
                    InterruptState::Delayed
                } else {
                    InterruptState::Disabled
                };
                assert_eq!(
                    cpu.interrupt_state, expected,
                    "sequence {sequence}, opcode {opcode:02X}"
                );
            }
        }
    }
}

fn pending_cpu() -> CPU {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    assert!(
        cpu.complete_cycle(response(cpu.request_cycle(), Some(0xFB)))
            .map(instruction_complete)
            .unwrap()
    );
    assert_eq!(cpu.interrupt_state, InterruptState::Delayed);
    cpu
}

#[test]
fn delayed_enable_waits_for_final_cycle_of_every_supported_base_instruction() {
    for opcode in 0u8..=255 {
        if opcode == 0xCB || DecodedOpcode::from(opcode) == DecodedOpcode::Unknown {
            continue;
        }
        let mut cpu = pending_cpu();
        let mut byte = Some(opcode);
        let mut completed = false;
        for _ in 0..8 {
            let before = cpu.snapshot();
            let action = cpu.request_cycle().unwrap();
            let data = match action {
                CycleAction::Read { addr } => {
                    assert!(
                        matches!(cpu.complete_cycle(response(cpu.request_cycle(), None)).map(instruction_complete), Err(CpuError::UnexpectedCycleResponse(_)) if cpu.request_cycle().unwrap() == (CycleAction::Read { addr }))
                    );
                    assert_eq!(cpu.snapshot(), before);
                    assert_eq!(cpu.interrupt_state, InterruptState::Delayed);
                    Some(byte.take().unwrap_or(0))
                }
                CycleAction::Write { .. } | CycleAction::Idle => None,
            };
            completed = cpu
                .complete_cycle(response(cpu.request_cycle(), data))
                .map(instruction_complete)
                .unwrap();
            if completed {
                assert_eq!(
                    cpu.interrupt_state,
                    if opcode == 0xF3 {
                        InterruptState::Disabled
                    } else {
                        InterruptState::Enabled
                    },
                    "opcode {opcode:02X}"
                );
                break;
            }
            assert_eq!(
                cpu.interrupt_state,
                InterruptState::Delayed,
                "opcode {opcode:02X}"
            );
        }
        assert!(completed, "opcode {opcode:02X} exceeded cycle bound");
    }
}

#[test]
fn delayed_enable_waits_for_whole_cb_instruction_for_every_encoding() {
    for opcode in 0u8..=255 {
        let mut cpu = pending_cpu();
        assert!(
            !cpu.complete_cycle(response(cpu.request_cycle(), Some(0xCB)))
                .map(instruction_complete)
                .unwrap()
        );
        assert_eq!(cpu.interrupt_state, InterruptState::Delayed);
        let mut data = Some(opcode);
        let mut completed = false;
        for _ in 0..3 {
            let before = cpu.snapshot();
            let read = match cpu.request_cycle().unwrap() {
                CycleAction::Read { addr } => {
                    assert!(
                        matches!(cpu.complete_cycle(response(cpu.request_cycle(), None)).map(instruction_complete), Err(CpuError::UnexpectedCycleResponse(_)) if cpu.request_cycle().unwrap() == (CycleAction::Read { addr }))
                    );
                    assert_eq!(cpu.snapshot(), before);
                    assert_eq!(cpu.interrupt_state, InterruptState::Delayed);
                    Some(data.take().unwrap_or(0x81))
                }
                CycleAction::Write { .. } | CycleAction::Idle => None,
            };
            completed = cpu
                .complete_cycle(response(cpu.request_cycle(), read))
                .map(instruction_complete)
                .unwrap();
            assert_eq!(
                cpu.interrupt_state,
                if completed {
                    InterruptState::Enabled
                } else {
                    InterruptState::Delayed
                }
            );
            if completed {
                break;
            }
        }
        assert!(completed, "CB {opcode:02X}");
    }
}

#[test]
fn unsupported_instruction_never_matures_delayed_enable() {
    let mut cpu = pending_cpu();
    assert!(matches!(
        cpu.complete_cycle(response(cpu.request_cycle(), Some(0xD3)))
            .map(instruction_complete),
        Err(CpuError::UnsupportedOpcode(_))
    ));
    let after = cpu.snapshot();
    for _ in 0..3 {
        assert!(matches!(
            cpu.request_cycle(),
            Err(CpuError::UnsupportedOpcode(_))
        ));
        assert!(matches!(
            cpu.complete_cycle(response(cpu.request_cycle(), Some(0)))
                .map(instruction_complete),
            Err(CpuError::UnsupportedOpcode(_))
        ));
        assert_eq!(cpu.interrupt_state, InterruptState::Delayed);
        assert_eq!(cpu.snapshot(), after);
    }
}

// Test-only observer for System tests; production CPU state remains private.
impl CPU {
    pub(crate) fn test_interrupt_status(&self) -> (bool, bool) {
        (
            self.interrupt_state == InterruptState::Delayed,
            self.interrupt_state == InterruptState::Enabled,
        )
    }
}
