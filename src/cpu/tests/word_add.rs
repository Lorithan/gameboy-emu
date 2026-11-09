use crate::cpu::{
    CPU, Registers,
    opcode::DecodedOpcode,
    registers::{Flag, Register16},
};

const CASES: [(u8, Register16); 4] = [
    (0x09, Register16::BC),
    (0x19, Register16::DE),
    (0x29, Register16::HL),
    (0x39, Register16::SP),
];

fn machine(lhs: u16, rhs: u16, source: Register16, flags: u8) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(0x5A00 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(lhs);
    r.sp = 0xDEF0;
    match source {
        Register16::BC => r.set_bc(rhs),
        Register16::DE => r.set_de(rhs),
        Register16::HL => assert_eq!(lhs, rhs),
        Register16::SP => r.set_sp(rhs),
    }
    CPU::from_registers(r)
}

fn finish(cpu: &mut CPU, opcode: u8) {
    let mut pending = cpu.snapshot();
    pending.pc += 1;
    assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
    assert_eq!(
        cpu.snapshot(),
        pending,
        "opcode fetch must not change HL/flags"
    );
    assert!(cpu.complete_cycle(None).unwrap());
}

#[test]
fn word_add_mappings_are_independent() {
    for (opcode, source) in CASES {
        assert_eq!(
            DecodedOpcode::from(opcode),
            DecodedOpcode::AddHL { target: source }
        );
    }
}

#[test]
fn word_add_half_carry_tracks_bit_twelve_not_bit_eleven() {
    for (opcode, source) in CASES {
        for (lhs, rhs, half) in [
            (0x0800, 0x0800, true),
            (0x0400, 0x0400, false),
            (0x07FF, 1, false),
            (0x0FFF, 1, true),
            (0xFFFF, 1, true),
        ] {
            if source == Register16::HL && lhs != rhs {
                continue;
            }
            let mut cpu = machine(lhs, rhs, source, 0);
            finish(&mut cpu, opcode);
            assert_eq!(
                cpu.registers.get_flag(Flag::H),
                half,
                "opcode {opcode:02X}, {lhs:04X}+{rhs:04X}"
            );
        }
    }
}

#[test]
fn word_add_preserves_z_independently_of_the_result() {
    for (opcode, source) in CASES {
        for (lhs, rhs) in [(0u16, 0u16), (1, 1), (0x8000, 0x8000)] {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = machine(lhs, rhs, source, flags);
                finish(&mut cpu, opcode);
                assert_eq!(
                    cpu.registers.get_flag(Flag::Z),
                    flags & 0x80 != 0,
                    "opcode {opcode:02X}, {lhs:04X}+{rhs:04X}, F={flags:02X}"
                );
                assert!(!cpu.registers.get_flag(Flag::N));
            }
        }
    }
}

fn check_sum(opcode: u8, source: Register16, lhs: u16, rhs: u16) {
    let mut cpu = machine(lhs, rhs, source, 0xF0);
    let mut expected = cpu.snapshot();
    finish(&mut cpu, opcode);
    let actual = cpu.snapshot();
    let sum = u32::from(lhs) + u32::from(rhs);
    expected.hl = (sum % 65536) as u16;
    expected.pc += 1;
    // Widened full/low-12-bit sums form an independent oracle for all flags.
    expected.af = (expected.af & 0xFF00)
        + (expected.af & 0x80)
        + if u32::from(lhs % 4096) + u32::from(rhs % 4096) >= 4096 {
            0x20
        } else {
            0
        }
        + if sum > 65535 { 0x10 } else { 0 };
    assert_eq!(actual, expected);
}

#[test]
fn word_add_result_and_carry_sweep_both_operands_and_self_source() {
    let boundaries = [
        0u16, 1, 0x000F, 0x0010, 0x00FF, 0x0100, 0x07FF, 0x0800, 0x0FFF, 0x1000, 0x7FFF, 0x8000,
        0xFFFF,
    ];
    for (opcode, source) in CASES {
        for value in 0u16..=65535 {
            if source == Register16::HL {
                check_sum(opcode, source, value, value);
            } else {
                for boundary in boundaries {
                    check_sum(opcode, source, value, boundary);
                    check_sum(opcode, source, boundary, value);
                }
            }
        }
    }
}

#[test]
fn word_add_captures_operands_preserves_flags_and_retries_across_pc_wrap() {
    for (opcode, source) in CASES {
        for lhs in [0u16, 1, 0x07FF, 0x0800, 0x0FFF, 0x8000, 0xFFFF] {
            for operand in [0u16, 1, 0x0800, 0x1000, 0xFFFF] {
                let rhs = if source == Register16::HL {
                    lhs
                } else {
                    operand
                };
                for flags in (0u8..=0xF0).step_by(16) {
                    let mut cpu = machine(lhs, rhs, source, flags);
                    cpu.registers.pc = 0xFFFF;
                    let before = cpu.snapshot();
                    assert!(matches!(
                        cpu.complete_cycle(None),
                        Err(crate::cpu::CpuError::MissingReadValue { addr: 0xFFFF })
                    ));
                    assert_eq!(cpu.snapshot(), before);
                    assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                    let mut pending = before;
                    pending.pc = 0;
                    assert_eq!(cpu.snapshot(), pending);
                    // Test-only changes verify both operands were captured.
                    cpu.registers.set_hl(0x4321);
                    match source {
                        Register16::BC => cpu.registers.set_bc(0x1234),
                        Register16::DE => cpu.registers.set_de(0x2345),
                        Register16::SP => cpu.registers.sp = 0x3456,
                        Register16::HL => {}
                    }
                    let changed = cpu.snapshot();
                    for _ in 0..3 {
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(crate::cpu::CycleAction::Idle)
                        ));
                        assert_eq!(cpu.snapshot(), changed);
                    }
                    assert!(cpu.complete_cycle(None).unwrap());
                    let sum = u32::from(lhs) + u32::from(rhs);
                    let mut expected = changed;
                    expected.hl = (sum % 65536) as u16;
                    expected.af = (before.af & 0xFF80)
                        + if u32::from(lhs % 4096) + u32::from(rhs % 4096) >= 4096 {
                            0x20
                        } else {
                            0
                        }
                        + if sum > 65535 { 0x10 } else { 0 };
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(crate::cpu::CycleAction::Read { addr: 0 })
                    ));
                    assert!(cpu.complete_cycle(Some(0)).unwrap());
                    expected.pc = 1;
                    assert_eq!(cpu.snapshot(), expected);
                }
            }
        }
    }
}
