use super::{CPU, CpuError, CycleAction, Registers};
use crate::cpu::opcode::DecodedOpcode;

#[test]
fn complement_carry_opcode_mappings_are_independent() {
    assert!(matches!(DecodedOpcode::from(0x2F), DecodedOpcode::Cpl));
    assert!(matches!(DecodedOpcode::from(0x37), DecodedOpcode::Scf));
    assert!(matches!(DecodedOpcode::from(0x3F), DecodedOpcode::Ccf));
}

#[test]
fn complement_carry_exhaustive_flags_values_and_wrapping() {
    for opcode in [0x2Fu8, 0x37, 0x3F] {
        for a in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                for pc in [0x0100, 0xFFFF] {
                    let mut registers = Registers::dmg_post_boot(1);
                    registers.set_af(u16::from(a) * 256 + u16::from(flags));
                    registers.set_bc(0x1234);
                    registers.set_de(0x5678);
                    registers.set_hl(0x9ABC);
                    registers.sp = 0xDEF0;
                    registers.pc = pc;
                    let mut cpu = CPU::from_registers(registers);
                    let before = cpu.snapshot();
                    for _ in 0..2 {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pc)
                        );
                        assert!(
                            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pc)
                        );
                        assert_eq!(cpu.snapshot(), before);
                    }
                    let (value, expected_flags) = match opcode {
                        0x2F => (255 - a, flags | 0x60),
                        0x37 => (a, (flags & 0x80) + 0x10),
                        _ => (a, (flags & 0x80) + if flags & 0x10 == 0 { 0x10 } else { 0 }),
                    };
                    let mut expected = before;
                    expected.af = u16::from(value) * 256 + u16::from(expected_flags);
                    expected.pc = pc.wrapping_add(1);
                    assert!(cpu.complete_cycle(Some(opcode)).unwrap());
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                    );
                    // Second application restores A for Cpl and C for Ccf;
                    // N/H follow the instruction rules rather than reverting.
                    assert!(cpu.complete_cycle(Some(opcode)).unwrap());
                    expected.pc = expected.pc.wrapping_add(1);
                    expected.af = match opcode {
                        0x2F => u16::from(a) * 256 + u16::from(flags | 0x60),
                        0x37 => expected.af,
                        _ => u16::from(a) * 256 + u16::from(flags & 0x90),
                    };
                    assert_eq!(cpu.snapshot(), expected);
                }
            }
        }
    }
}
