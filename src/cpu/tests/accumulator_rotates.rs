use super::{CPU, CpuError, CycleAction, Registers};
use crate::cpu::opcode::{DecodedOpcode, RotationDirection};

#[test]
fn rotate_mappings_are_explicit() {
    for (byte, direction) in [
        (
            0x07,
            RotationDirection::Left {
                through_carry: false,
            },
        ),
        (
            0x0F,
            RotationDirection::Right {
                through_carry: false,
            },
        ),
        (
            0x17,
            RotationDirection::Left {
                through_carry: true,
            },
        ),
        (
            0x1F,
            RotationDirection::Right {
                through_carry: true,
            },
        ),
    ] {
        assert_eq!(
            DecodedOpcode::from(byte),
            DecodedOpcode::Rotate { direction }
        );
    }
}

// Widened arithmetic models the 8/9-bit ring independently of production shifts.
fn reference(opcode: u8, a: u8, carry: bool) -> (u8, bool) {
    let a = u16::from(a);
    match opcode {
        0x07 => (((a * 2) % 256 + a / 128) as u8, a >= 128),
        0x0F => ((a / 2 + (a % 2) * 128) as u8, a % 2 != 0),
        0x17 => (((a * 2) % 256 + u16::from(carry)) as u8, a >= 128),
        _ => ((a / 2 + u16::from(carry) * 128) as u8, a % 2 != 0),
    }
}

#[test]
fn rotations_exhaust_all_values_flags_wrap_retries_and_repetition() {
    for opcode in [0x07, 0x0F, 0x17, 0x1F] {
        for a in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                for pc in [0x0100, 0xFFFE, 0xFFFF] {
                    let mut r = Registers::dmg_post_boot(1);
                    r.set_af(u16::from(a) * 256 + u16::from(flags));
                    r.set_bc(0x1234);
                    r.set_de(0x5678);
                    r.set_hl(0x9ABC);
                    r.pc = pc;
                    r.sp = 0xDEF0;
                    let mut cpu = CPU::from_registers(r);
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
                    let (mut value, mut carry) = (a, flags & 0x10 != 0);
                    let mut expected = before;
                    for _ in 0..9 {
                        (value, carry) = reference(opcode, value, carry);
                        assert!(cpu.complete_cycle(Some(opcode)).unwrap());
                        expected.af = u16::from(value) * 256 + if carry { 0x10 } else { 0 };
                        expected.pc = expected.pc.wrapping_add(1);
                        assert_eq!(
                            cpu.snapshot(),
                            expected,
                            "opcode {opcode:02X}, original A={a:02X}, F={flags:02X}"
                        );
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                        );
                    }
                }
            }
        }
    }
}
