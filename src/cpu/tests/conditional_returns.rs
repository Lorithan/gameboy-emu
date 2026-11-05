use crate::cpu::{
    CPU, CpuError, CycleAction, Registers,
    opcode::{DecodedOpcode, JumpCondition},
};

fn taken(byte: u8, flags: u16) -> bool {
    match byte {
        0xC0 => flags & 0x80 == 0,
        0xC8 => flags & 0x80 != 0,
        0xD0 => flags & 0x10 == 0,
        0xD8 => flags & 0x10 != 0,
        _ => unreachable!(),
    }
}

#[test]
fn conditional_return_mappings_are_independent_and_leave_c9_unchanged() {
    for (byte, expected) in [
        (0xC0, JumpCondition::NZ),
        (0xC8, JumpCondition::Z),
        (0xD0, JumpCondition::NC),
        (0xD8, JumpCondition::C),
    ] {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::ConditionalReturn { condition } => assert_eq!(condition, expected),
            _ => panic!("incorrect conditional return {byte:02X}"),
        }
    }
    assert!(matches!(DecodedOpcode::from(0xC9), DecodedOpcode::Return));
    assert!(matches!(DecodedOpcode::from(0xD9), DecodedOpcode::Unknown));
}

#[test]
fn conditional_returns_cover_every_target_and_both_decisions() {
    for byte in [0xC0, 0xC8, 0xD0, 0xD8] {
        for flags in [0u16, 0xF0] {
            for target in 0u16..=65535 {
                let mut registers = Registers::dmg_post_boot(1);
                registers.set_af(0x4200 | flags);
                registers.sp = 0xFFFC;
                let mut cpu = CPU::from_registers(registers);
                let mut expected = cpu.snapshot();
                assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                expected.pc += 1;
                assert_eq!(cpu.snapshot(), expected);
                assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                let decision = taken(byte, flags);
                assert_eq!(cpu.complete_cycle(None).unwrap(), !decision);
                assert_eq!(cpu.snapshot(), expected);
                if decision {
                    assert!(matches!(
                        cpu.request_cycle().unwrap(),
                        CycleAction::Read { addr: 0xFFFC }
                    ));
                    assert!(!cpu.complete_cycle(Some(target as u8)).unwrap());
                    expected.sp = 0xFFFD;
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(matches!(
                        cpu.request_cycle().unwrap(),
                        CycleAction::Read { addr: 0xFFFD }
                    ));
                    assert!(!cpu.complete_cycle(Some((target / 256) as u8)).unwrap());
                    expected.sp = 0xFFFE;
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                    assert!(cpu.complete_cycle(None).unwrap());
                    expected.pc = target;
                }
                assert_eq!(cpu.snapshot(), expected);
                assert!(
                    matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == expected.pc)
                );
            }
        }
    }
}

#[test]
fn conditional_returns_capture_flags_and_stack_through_pause_retry_and_wrap() {
    for byte in [0xC0, 0xC8, 0xD0, 0xD8] {
        for flags in 0u16..16 {
            for pc in [0u16, 0x0100, 0xFFFF] {
                for sp in [0u16, 1, 0xFFFE, 0xFFFF] {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.pc = pc;
                    registers.sp = sp;
                    registers.set_af(0x4200 | (flags << 4));
                    let mut cpu = CPU::from_registers(registers);
                    let initial = cpu.snapshot();
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pc)
                    );
                    assert_eq!(cpu.snapshot(), initial);
                    assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                    cpu.registers.sp = 0x8000;
                    cpu.registers.set_af(initial.af ^ 0x90);
                    let mut expected = cpu.snapshot();
                    for _ in 0..2 {
                        assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    let decision = taken(byte, flags << 4);
                    assert_eq!(cpu.complete_cycle(None).unwrap(), !decision);
                    assert_eq!(cpu.snapshot(), expected);
                    if decision {
                        for (address, value, final_read) in
                            [(sp, 0x34, false), (sp.wrapping_add(1), 0x12, true)]
                        {
                            for _ in 0..2 {
                                assert!(
                                    matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == address)
                                );
                                assert!(
                                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == address)
                                );
                                assert_eq!(cpu.snapshot(), expected);
                            }
                            assert!(!cpu.complete_cycle(Some(value)).unwrap());
                            expected.sp = address.wrapping_add(1);
                            assert_eq!(cpu.snapshot(), expected);
                            if final_read {
                                assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                            }
                        }
                        assert!(cpu.complete_cycle(None).unwrap());
                        expected.pc = 0x1234;
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
