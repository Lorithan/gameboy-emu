use crate::cpu::{
    CPU, CpuError, CycleAction, ExecutionState, Registers,
    opcode::{DecodedOpcode, JumpCondition},
};

#[test]
fn unconditional_jump_decodes_to_always() {
    assert!(matches!(
        DecodedOpcode::from(0x18),
        DecodedOpcode::RelativeJump {
            condition: JumpCondition::Always
        }
    ));
}

#[test]
fn unconditional_jump_handles_every_displacement_flags_and_pc_boundaries() {
    for start in [0u16, 0x007E, 0x0100, 0x7FFF, 0xFFFE, 0xFFFF] {
        for raw in 0u8..=255 {
            for flags in 0u16..16 {
                let mut registers = Registers::dmg_post_boot(1);
                registers.pc = start;
                registers.set_af(0x4200 | (flags << 4));
                let mut cpu = CPU::from_registers(registers);
                let mut expected = cpu.snapshot();
                assert!(!cpu.complete_cycle(Some(0x18)).unwrap());
                expected.pc = start.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
                for _ in 0..2 {
                    assert!(
                        matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == expected.pc)
                    );
                    assert_eq!(cpu.snapshot(), expected);
                }
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == expected.pc)
                );
                assert_eq!(cpu.snapshot(), expected);
                assert!(!cpu.complete_cycle(Some(raw)).unwrap());
                expected.pc = start.wrapping_add(2);
                assert_eq!(cpu.snapshot(), expected);
                for _ in 0..2 {
                    assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                    assert_eq!(cpu.snapshot(), expected);
                }
                assert!(cpu.complete_cycle(None).unwrap());
                // Independent wide arithmetic oracle, rather than wrapping_add_signed.
                let displacement = if raw < 128 {
                    i32::from(raw)
                } else {
                    i32::from(raw) - 256
                };
                expected.pc = (i32::from(expected.pc) + displacement).rem_euclid(65536) as u16;
                assert_eq!(cpu.snapshot(), expected);
                assert!(
                    matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == expected.pc)
                );
            }
        }
    }
}

#[test]
fn false_condition_completes_at_displacement_without_idle_or_extra_pc_increment() {
    // Exercise the resolved false decision before conditional mappings exist.
    for operand_pc in [0x0101u16, 0xFFFF] {
        let mut registers = Registers::dmg_post_boot(0);
        registers.set_af(0x4200);
        registers.pc = operand_pc;
        let mut cpu = CPU::from_registers(registers);
        cpu.state = ExecutionState::ReadDisplacement { taken: false };
        let mut expected = cpu.snapshot();
        expected.pc = operand_pc.wrapping_add(1);
        assert!(
            cpu.complete_cycle(Some(0x80)).unwrap(),
            "untaken branch must complete on operand read"
        );
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == expected.pc)
        );
    }
}

#[test]
fn internal_jump_cycle_uses_captured_target() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    assert!(!cpu.complete_cycle(Some(0x18)).unwrap());
    assert!(!cpu.complete_cycle(Some(0xFE)).unwrap());
    // Test-only perturbation checks that the final cycle uses its payload,
    // rather than recomputing from live PC. Normal execution leaves PC alone.
    cpu.registers.pc = 0x8000;
    let mut expected = cpu.snapshot();
    expected.pc = 0x0100;
    assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
    assert!(cpu.complete_cycle(None).unwrap());
    assert_eq!(cpu.snapshot(), expected);
}

#[test]
fn conditional_jumps_decode_to_the_correct_condition() {
    for (byte, expected) in [
        (0x20, JumpCondition::NZ),
        (0x28, JumpCondition::Z),
        (0x30, JumpCondition::NC),
        (0x38, JumpCondition::C),
    ] {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::RelativeJump { condition } => assert_eq!(condition, expected),
            _ => panic!("missing jump mapping {byte:#04X}"),
        }
    }
}

#[test]
fn conditional_jumps_cover_all_offsets_flags_wraps_and_captured_decisions() {
    for byte in [0x20, 0x28, 0x30, 0x38] {
        for flags in 0u16..16 {
            let raw_flags = flags << 4;
            let taken = match byte {
                0x20 => raw_flags & 0x80 == 0,
                0x28 => raw_flags & 0x80 != 0,
                0x30 => raw_flags & 0x10 == 0,
                0x38 => raw_flags & 0x10 != 0,
                _ => unreachable!(),
            };
            for start in [0u16, 0x007E, 0x0100, 0x7FFF, 0xFFFE, 0xFFFF] {
                for operand in 0u8..=255 {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.pc = start;
                    registers.set_af(0x4200 | raw_flags);
                    let mut cpu = CPU::from_registers(registers);
                    let initial = cpu.snapshot();
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start)
                    );
                    assert_eq!(cpu.snapshot(), initial);
                    assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                    let mut expected = initial;
                    expected.pc = start.wrapping_add(1);
                    assert_eq!(cpu.snapshot(), expected);
                    // Flip Z and C after decode. The captured decision must survive
                    // both this perturbation and a missing-data retry.
                    cpu.registers.set_af(0x4200 | (raw_flags ^ 0x90));
                    expected.af ^= 0x90;
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == expected.pc)
                    );
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(
                        matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == expected.pc)
                    );
                    assert_eq!(cpu.complete_cycle(Some(operand)).unwrap(), !taken);
                    expected.pc = start.wrapping_add(2);
                    assert_eq!(cpu.snapshot(), expected);
                    if taken {
                        assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                        assert!(cpu.complete_cycle(None).unwrap());
                        let signed = if operand < 128 {
                            i32::from(operand)
                        } else {
                            i32::from(operand) - 256
                        };
                        expected.pc = (i32::from(expected.pc) + signed).rem_euclid(65536) as u16;
                    }
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(
                        matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == expected.pc)
                    );
                }
            }
        }
    }
}
