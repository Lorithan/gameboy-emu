use crate::cpu::{
    CPU, CpuError, CycleAction, Registers,
    opcode::{DecodedOpcode, JumpCondition},
};

const IMMEDIATE: [u8; 5] = [0xC3, 0xC2, 0xCA, 0xD2, 0xDA];

fn taken(byte: u8, flags: u16) -> bool {
    match byte {
        0xC3 => true,
        0xC2 => flags & 0x80 == 0,
        0xCA => flags & 0x80 != 0,
        0xD2 => flags & 0x10 == 0,
        0xDA => flags & 0x10 != 0,
        _ => panic!("unexpected test opcode"),
    }
}

fn read_request(cpu: &CPU, address: u16) {
    let before = cpu.snapshot();
    for _ in 0..2 {
        assert!(
            matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == address)
        );
        assert_eq!(cpu.snapshot(), before);
    }
}

fn missing_read(cpu: &mut CPU, address: u16) {
    let before = cpu.snapshot();
    assert!(
        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == address)
    );
    assert_eq!(cpu.snapshot(), before);
    read_request(cpu, address);
}

#[test]
fn absolute_jump_mappings_are_independent_and_complete() {
    for (byte, expected) in [
        (0xC3, JumpCondition::Always),
        (0xC2, JumpCondition::NZ),
        (0xCA, JumpCondition::Z),
        (0xD2, JumpCondition::NC),
        (0xDA, JumpCondition::C),
    ] {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::AbsoluteJump { condition } => assert_eq!(condition, expected),
            _ => panic!("wrong mapping for {byte:#04X}"),
        }
    }
    assert!(matches!(DecodedOpcode::from(0xE9), DecodedOpcode::JumpHL));
}

#[test]
fn immediate_jumps_consume_every_target_in_little_endian_order() {
    for byte in IMMEDIATE {
        for flags in [0u16, 0xF0] {
            for target in 0u16..=u16::MAX {
                let mut registers = Registers::dmg_post_boot(1);
                registers.set_af(0x4200 | flags);
                let mut cpu = CPU::from_registers(registers);
                let mut expected = cpu.snapshot();
                assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                expected.pc = 0x0101;
                assert_eq!(cpu.snapshot(), expected);
                assert!(!cpu.complete_cycle(Some(target as u8)).unwrap());
                expected.pc = 0x0102;
                assert_eq!(cpu.snapshot(), expected);
                assert_eq!(
                    cpu.complete_cycle(Some((target / 256) as u8)).unwrap(),
                    !taken(byte, flags)
                );
                expected.pc = 0x0103;
                assert_eq!(cpu.snapshot(), expected);
                if taken(byte, flags) {
                    assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                    assert!(cpu.complete_cycle(None).unwrap());
                    expected.pc = target;
                }
                assert_eq!(cpu.snapshot(), expected);
                read_request(&cpu, expected.pc);
            }
        }
    }
}

#[test]
fn jump_hl_copies_every_address_for_every_flag_combination_in_one_cycle() {
    for flags in 0u16..16 {
        for target in 0u16..=u16::MAX {
            let mut registers = Registers::dmg_post_boot(1);
            registers.pc = 0xFFFF;
            registers.set_hl(target);
            registers.set_af(0x4200 | (flags << 4));
            let mut cpu = CPU::from_registers(registers);
            let mut expected = cpu.snapshot();
            missing_read(&mut cpu, 0xFFFF);
            assert!(cpu.complete_cycle(Some(0xE9)).unwrap());
            expected.pc = target;
            assert_eq!(cpu.snapshot(), expected);
            read_request(&cpu, target);
        }
    }
}

#[test]
fn jump_reads_retry_wrap_and_preserve_decisions_low_bytes_and_targets() {
    for byte in IMMEDIATE {
        for flag_bits in 0u16..16 {
            let flags = flag_bits << 4;
            for start in [0u16, 0x0100, 0xFFFD, 0xFFFE, 0xFFFF] {
                for target in [0u16, 0x00FF, 0x1234, 0x8000, 0xFF00, 0xFFFF] {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.pc = start;
                    registers.set_af(0x4200 | flags);
                    let mut cpu = CPU::from_registers(registers);
                    missing_read(&mut cpu, start);
                    assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                    // Flipped flags cannot alter the captured branch decision.
                    cpu.registers.set_af(0x4200 | (flags ^ 0x90));
                    let mut expected = cpu.snapshot();
                    missing_read(&mut cpu, start.wrapping_add(1));
                    assert!(!cpu.complete_cycle(Some(target as u8)).unwrap());
                    expected.pc = start.wrapping_add(2);
                    assert_eq!(cpu.snapshot(), expected);
                    missing_read(&mut cpu, expected.pc);
                    assert_eq!(
                        cpu.complete_cycle(Some((target / 256) as u8)).unwrap(),
                        !taken(byte, flags)
                    );
                    expected.pc = start.wrapping_add(3);
                    assert_eq!(cpu.snapshot(), expected);
                    if taken(byte, flags) {
                        // Only the stored target determines the final PC.
                        cpu.registers.pc = !target;
                        for _ in 0..2 {
                            assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
                            assert_eq!(cpu.snapshot().pc, !target);
                        }
                        assert!(cpu.complete_cycle(None).unwrap());
                        expected.pc = target;
                    }
                    assert_eq!(cpu.snapshot(), expected);
                    read_request(&cpu, expected.pc);
                }
            }
        }
    }
}
