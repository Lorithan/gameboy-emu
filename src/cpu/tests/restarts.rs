use crate::cpu::{CPU, CpuError, CycleAction, Registers, opcode::DecodedOpcode};

const RESTARTS: [(u8, u16); 8] = [
    (0xC7, 0),
    (0xCF, 8),
    (0xD7, 0x10),
    (0xDF, 0x18),
    (0xE7, 0x20),
    (0xEF, 0x28),
    (0xF7, 0x30),
    (0xFF, 0x38),
];

#[test]
fn restart_opcodes_decode_to_independent_vector_table() {
    for (byte, expected) in RESTARTS {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::Restart { target } => assert_eq!(target, expected),
            _ => panic!("incorrect RST mapping {byte:02X}"),
        }
    }
}

#[test]
fn restart_captures_return_stack_and_vector_and_preserves_flags_at_boundaries() {
    for (byte, target) in RESTARTS {
        for pc in [0u16, 0x00FE, 0x00FF, 0x010F, 0xFFF0, 0xFFFF] {
            for sp in [0u16, 1, 0xC002, 0xFF80, 0xFFFE, 0xFFFF] {
                for flags in 0u16..16 {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.pc = pc;
                    registers.sp = sp;
                    registers.set_af(0x4200 | (flags << 4));
                    let mut cpu = CPU::from_registers(registers);
                    let before = cpu.snapshot();
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pc)
                    );
                    assert_eq!(cpu.snapshot(), before);
                    assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                    cpu.registers.pc = 0xABCD;
                    cpu.registers.sp = 0x8000;
                    let mut expected = cpu.snapshot();
                    for _ in 0..2 {
                        assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    assert!(!cpu.complete_cycle(None).unwrap());
                    let return_pc = pc.wrapping_add(1);
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
                    expected.pc = target;
                    assert_eq!(cpu.snapshot(), expected);
                }
            }
        }
    }
}

#[test]
fn restarts_push_the_full_return_address_for_every_opcode_address() {
    for (byte, target) in RESTARTS {
        for pc in 0u16..=65535 {
            let mut registers = Registers::dmg_post_boot(1);
            registers.pc = pc;
            let mut cpu = CPU::from_registers(registers);
            let mut expected = cpu.snapshot();
            let return_pc = pc.wrapping_add(1);
            assert!(!cpu.complete_cycle(Some(byte)).unwrap());
            expected.pc = return_pc;
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
            assert!(!cpu.complete_cycle(None).unwrap());
            assert_eq!(cpu.snapshot(), expected);
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr: 0xFFFD, value } if value == (return_pc / 256) as u8)
            );
            assert!(!cpu.complete_cycle(None).unwrap());
            expected.sp = 0xFFFD;
            assert_eq!(cpu.snapshot(), expected);
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr: 0xFFFC, value } if value == return_pc as u8),
                "RST {byte:02X} at {pc:04X} must push all eight low bits of {return_pc:04X}"
            );
            assert!(cpu.complete_cycle(None).unwrap());
            expected.sp = 0xFFFC;
            expected.pc = target;
            assert_eq!(cpu.snapshot(), expected);
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == target)
            );
        }
    }
}
