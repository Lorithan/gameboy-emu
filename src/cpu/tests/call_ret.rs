use crate::cpu::{
    CPU, CpuError, CycleAction, Registers,
    opcode::{DecodedOpcode, JumpCondition},
};

fn taken(byte: u8, flags: u16) -> bool {
    match byte {
        0xCD => true,
        0xC4 => flags & 0x80 == 0,
        0xCC => flags & 0x80 != 0,
        0xD4 => flags & 0x10 == 0,
        0xDC => flags & 0x10 != 0,
        _ => unreachable!(),
    }
}

#[test]
fn ret_decodes_to_unconditional_return() {
    assert!(matches!(DecodedOpcode::from(0xC9), DecodedOpcode::Return));
}

#[test]
fn ret_reads_every_target_from_stack_and_applies_it_only_on_idle() {
    for target in 0u16..=u16::MAX {
        let mut registers = Registers::dmg_post_boot(1);
        registers.sp = 0xFFFC;
        let mut cpu = CPU::from_registers(registers);
        let mut expected = cpu.snapshot();
        assert!(!cpu.complete_cycle(Some(0xC9)).unwrap());
        expected.pc = 0x0101;
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(
                cpu.request_cycle().unwrap(),
                CycleAction::Read { addr: 0xFFFC }
            ),
            "RET must read the stack, not the instruction stream"
        );
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
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == target)
        );
    }
}

#[test]
fn ret_reports_stack_read_errors_and_retains_captured_addresses_bytes_and_target() {
    for start in [0u16, 0x0100, 0xFFFF] {
        for sp in [0u16, 1, 0xFFFE, 0xFFFF] {
            for flags in 0u16..16 {
                let mut registers = Registers::dmg_post_boot(1);
                registers.pc = start;
                registers.sp = sp;
                registers.set_af(0x4200 | (flags << 4));
                let mut cpu = CPU::from_registers(registers);
                retry(&mut cpu, start);
                assert!(!cpu.complete_cycle(Some(0xC9)).unwrap());
                cpu.registers.sp = 0x8000; // Captured address must survive.
                let mut expected = cpu.snapshot();
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == sp),
                    "missing return byte must report captured stack address"
                );
                assert_eq!(cpu.snapshot(), expected);
                retry(&mut cpu, sp);
                assert!(!cpu.complete_cycle(Some(0x34)).unwrap());
                expected.sp = sp.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
                retry(&mut cpu, sp.wrapping_add(1));
                cpu.registers.sp = 0x9000;
                assert!(!cpu.complete_cycle(Some(0x12)).unwrap());
                expected.sp = sp.wrapping_add(2);
                assert_eq!(cpu.snapshot(), expected);
                cpu.registers.pc = 0xABCD;
                assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                assert!(cpu.complete_cycle(None).unwrap());
                expected.pc = 0x1234;
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}

#[test]
fn call_opcodes_decode_to_independent_expected_conditions() {
    for (byte, expected) in [
        (0xCD, JumpCondition::Always),
        (0xC4, JumpCondition::NZ),
        (0xCC, JumpCondition::Z),
        (0xD4, JumpCondition::NC),
        (0xDC, JumpCondition::C),
    ] {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::Call { condition } => assert_eq!(condition, expected),
            _ => panic!("incorrect CALL mapping {byte:02X}"),
        }
    }
}

#[test]
fn call_every_target_has_idle_before_writes_and_commits_on_final_write() {
    for target in 0u16..=u16::MAX {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
        let mut expected = cpu.snapshot();
        assert!(!cpu.complete_cycle(Some(0xCD)).unwrap());
        expected.pc = 0x0101;
        assert_eq!(cpu.snapshot(), expected);
        assert!(!cpu.complete_cycle(Some(target as u8)).unwrap());
        expected.pc = 0x0102;
        assert_eq!(cpu.snapshot(), expected);
        assert!(!cpu.complete_cycle(Some((target / 256) as u8)).unwrap());
        expected.pc = 0x0103;
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle().unwrap(), CycleAction::Idle),
            "CALL must idle before either stack write"
        );
        assert!(!cpu.complete_cycle(None).unwrap());
        assert_eq!(cpu.snapshot(), expected);
        assert!(matches!(
            cpu.request_cycle().unwrap(),
            CycleAction::Write {
                addr: 0xFFFD,
                value: 0x01
            }
        ));
        assert!(!cpu.complete_cycle(None).unwrap());
        expected.sp = 0xFFFD;
        assert_eq!(cpu.snapshot(), expected);
        assert!(matches!(
            cpu.request_cycle().unwrap(),
            CycleAction::Write {
                addr: 0xFFFC,
                value: 0x03
            }
        ));
        assert!(cpu.complete_cycle(None).unwrap());
        expected.sp = 0xFFFC;
        expected.pc = target;
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == target)
        );
    }
}

fn retry(cpu: &mut CPU, address: u16) {
    let before = cpu.snapshot();
    for _ in 0..2 {
        assert!(
            matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == address)
        );
        assert!(
            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == address)
        );
        assert_eq!(cpu.snapshot(), before);
    }
}

#[test]
fn call_captures_decision_return_target_and_stack_through_retry_and_wrapping() {
    for byte in [0xCD, 0xC4, 0xCC, 0xD4, 0xDC] {
        for flags in 0u16..16 {
            for start in [0u16, 0x0100, 0xFFFD, 0xFFFE, 0xFFFF] {
                for sp in [0u16, 1, 0xFF80, 0xFFFE, 0xFFFF] {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.pc = start;
                    registers.sp = sp;
                    registers.set_af(0x4200 | (flags << 4));
                    let mut cpu = CPU::from_registers(registers);
                    retry(&mut cpu, start);
                    assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                    cpu.registers.set_af(0x4200 | ((flags << 4) ^ 0x90));
                    cpu.registers.sp = 0x8000; // Initial stack address was captured.
                    retry(&mut cpu, start.wrapping_add(1));
                    assert!(!cpu.complete_cycle(Some(0x34)).unwrap());
                    retry(&mut cpu, start.wrapping_add(2));
                    let do_call = taken(byte, flags << 4);
                    assert_eq!(cpu.complete_cycle(Some(0x12)).unwrap(), !do_call);
                    let return_pc = start.wrapping_add(3);
                    let mut expected = cpu.snapshot();
                    assert_eq!(expected.pc, return_pc);
                    if do_call {
                        assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                        assert!(!cpu.complete_cycle(None).unwrap());
                        // Later live PC changes must not affect pushed return bytes.
                        cpu.registers.pc = 0xABCD;
                        expected.pc = 0xABCD;
                        for _ in 0..2 {
                            assert!(
                                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr, value } if addr == sp.wrapping_sub(1) && value == (return_pc / 256) as u8)
                            );
                            assert_eq!(cpu.snapshot(), expected);
                        }
                        assert!(!cpu.complete_cycle(None).unwrap());
                        expected.sp = sp.wrapping_sub(1);
                        assert_eq!(cpu.snapshot(), expected);
                        assert!(
                            matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr, value } if addr == sp.wrapping_sub(2) && value == return_pc as u8)
                        );
                        cpu.registers.sp = 0x9000;
                        assert!(cpu.complete_cycle(None).unwrap());
                        expected.sp = sp.wrapping_sub(2);
                        expected.pc = 0x1234;
                        assert_eq!(cpu.snapshot(), expected);
                    } else {
                        assert_eq!(expected.sp, 0x8000);
                        assert!(
                            matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == return_pc)
                        );
                    }
                }
            }
        }
    }
}
