use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers,
    opcode::{ArithmeticKind, DecodedOpcode, Operand, Target},
    registers::Register,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Register(Register),
    Memory,
    Immediate,
}

const CASES: [(u8, Source); 9] = [
    (0xB8, Source::Register(Register::B)),
    (0xB9, Source::Register(Register::C)),
    (0xBA, Source::Register(Register::D)),
    (0xBB, Source::Register(Register::E)),
    (0xBC, Source::Register(Register::H)),
    (0xBD, Source::Register(Register::L)),
    (0xBE, Source::Memory),
    (0xBF, Source::Register(Register::A)),
    (0xFE, Source::Immediate),
];

fn machine(left: u8, right: u8, source: Source, flags: u8, pc: u16) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(u16::from(left) << 8 | u16::from(flags));
    registers.set_bc(0x1234);
    registers.set_de(0x5678);
    registers.set_hl(0xC123);
    registers.pc = pc;
    registers.sp = 0xDEF0;
    match source {
        Source::Register(Register::B) => registers.b = right,
        Source::Register(Register::C) => registers.c = right,
        Source::Register(Register::D) => registers.d = right,
        Source::Register(Register::E) => registers.e = right,
        Source::Register(Register::H) => registers.h = right,
        Source::Register(Register::L) => registers.l = right,
        Source::Register(Register::A) => assert_eq!(left, right),
        Source::Memory | Source::Immediate => {}
    }
    CPU::from_registers(registers)
}

fn reference(mut snapshot: CpuSnapShot, left: u8, right: u8) -> CpuSnapShot {
    let difference = i16::from(left) - i16::from(right);
    let low = i16::from(left % 16) - i16::from(right % 16);
    let flags = 0x40u16
        | if difference == 0 { 0x80 } else { 0 }
        | if low < 0 { 0x20 } else { 0 }
        | if difference < 0 { 0x10 } else { 0 };
    snapshot.af = (snapshot.af & 0xFF00) | flags;
    snapshot
}

#[test]
fn comparison_decodes_all_sources_and_preserves_unsupported_boundaries() {
    for (opcode, source) in CASES {
        match source {
            Source::Register(register) => assert!(
                matches!(DecodedOpcode::from(opcode), DecodedOpcode::Arithmetic { kind: ArithmeticKind::Compare, operand: Operand::Target(Target::Register(actual)) } if actual == register)
            ),
            Source::Memory => assert!(matches!(
                DecodedOpcode::from(opcode),
                DecodedOpcode::Arithmetic {
                    kind: ArithmeticKind::Compare,
                    operand: Operand::Target(Target::IndirectHL)
                }
            )),
            Source::Immediate => assert!(matches!(
                DecodedOpcode::from(opcode),
                DecodedOpcode::Arithmetic {
                    kind: ArithmeticKind::Compare,
                    operand: Operand::Immediate
                }
            )),
        }
    }
    assert!(matches!(DecodedOpcode::from(0x76), DecodedOpcode::Unknown));
}

#[test]
fn comparison_all_operand_pairs_change_only_flags_and_pc() {
    for (opcode, source) in CASES
        .into_iter()
        .filter(|(_, source)| *source != Source::Register(Register::A))
    {
        for left in 0u8..=255 {
            for right in 0u8..=255 {
                let mut cpu = machine(left, right, source, (left ^ right) & 0xF0, 0x0100);
                let before = cpu.snapshot();
                let register = matches!(source, Source::Register(_));
                assert_eq!(cpu.complete_cycle(Some(opcode)).expect("opcode"), register);
                if !register {
                    let mut pending = before;
                    pending.pc += 1;
                    assert_eq!(cpu.snapshot(), pending);
                    let address = if source == Source::Immediate {
                        0x0101
                    } else {
                        0xC123
                    };
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == address)
                    );
                    assert!(cpu.complete_cycle(Some(right)).expect("operand"));
                }
                let mut expected = reference(before, left, right);
                expected.pc += if source == Source::Immediate { 2 } else { 1 };
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "opcode {opcode:02X}, A {left:02X}, operand {right:02X}"
                );
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                );
            }
        }
    }
}

#[test]
fn self_comparison_preserves_every_a_value_and_overwrites_all_flags() {
    for value in 0u8..=255 {
        for combination in 0u8..16 {
            let mut cpu = machine(
                value,
                value,
                Source::Register(Register::A),
                combination << 4,
                0x0100,
            );
            let mut expected = cpu.snapshot();
            expected.af = u16::from(value) << 8 | 0xC0;
            expected.pc += 1;
            assert!(cpu.complete_cycle(Some(0xBF)).expect("CP A,A"));
            assert_eq!(cpu.snapshot(), expected);
        }
    }
}

#[test]
fn comparison_boundaries_ignore_carry_and_retain_reads_across_retry_and_wrap() {
    for (opcode, source) in CASES {
        for left in [0u8, 1, 0x0F, 0x10, 0x80, 0xFF] {
            for right in [0u8, 1, 0x0F, 0x10, 0x80, 0xFF] {
                if source == Source::Register(Register::A) && left != right {
                    continue;
                }
                for combination in 0u8..16 {
                    for start in [0xFFFEu16, 0xFFFF] {
                        let mut cpu = machine(left, right, source, combination << 4, start);
                        let before = cpu.snapshot();
                        assert!(
                            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start)
                        );
                        assert_eq!(cpu.snapshot(), before);
                        let register = matches!(source, Source::Register(_));
                        assert_eq!(
                            cpu.complete_cycle(Some(opcode)).expect("retry opcode"),
                            register
                        );
                        let mut expected = before;
                        expected.pc = expected.pc.wrapping_add(1);
                        if !register {
                            assert_eq!(cpu.snapshot(), expected);
                            // Independent capture check: CP must calculate from
                            // captured A, and must never assign/restore live A.
                            cpu.registers.a = 0xA5;
                            cpu.registers.set_hl(0xD456);
                            expected = cpu.snapshot();
                            let address = if source == Source::Immediate {
                                expected.pc
                            } else {
                                0xC123
                            };
                            for _ in 0..2 {
                                assert!(
                                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == address)
                                );
                                assert!(
                                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == address)
                                );
                                assert_eq!(cpu.snapshot(), expected);
                            }
                            assert!(cpu.complete_cycle(Some(right)).expect("retry operand"));
                            if source == Source::Immediate {
                                expected.pc = expected.pc.wrapping_add(1);
                            }
                        }
                        expected = reference(expected, left, right);
                        assert_eq!(cpu.snapshot(), expected);
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                        );
                    }
                }
            }
        }
    }
}
