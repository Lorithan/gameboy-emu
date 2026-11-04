use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers,
    opcode::{ArithmeticKind, DecodedOpcode, Operand, Target},
    registers::{Flag, Register},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Register(Register),
    Memory,
    Immediate,
}

const CASES: [(u8, u8, Source); 9] = [
    (0x88, 0x98, Source::Register(Register::B)),
    (0x89, 0x99, Source::Register(Register::C)),
    (0x8A, 0x9A, Source::Register(Register::D)),
    (0x8B, 0x9B, Source::Register(Register::E)),
    (0x8C, 0x9C, Source::Register(Register::H)),
    (0x8D, 0x9D, Source::Register(Register::L)),
    (0x8E, 0x9E, Source::Memory),
    (0x8F, 0x9F, Source::Register(Register::A)),
    (0xCE, 0xDE, Source::Immediate),
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

fn reference(
    mut snapshot: CpuSnapShot,
    left: u8,
    right: u8,
    carry: bool,
    add: bool,
) -> CpuSnapShot {
    let sign = if add { 1i16 } else { -1 };
    let full = i16::from(left) + sign * (i16::from(right) + i16::from(carry));
    let low = i16::from(left % 16) + sign * (i16::from(right % 16) + i16::from(carry));
    let result = full.rem_euclid(256) as u16;
    let flags = if result == 0 { 0x80 } else { 0 }
        | if add { 0 } else { 0x40 }
        | if !(0..16).contains(&low) { 0x20 } else { 0 }
        | if !(0..256).contains(&full) { 0x10 } else { 0 };
    snapshot.af = result << 8 | flags;
    snapshot
}

fn finish(cpu: &mut CPU, opcode: u8, source: Source, right: u8) {
    let register = matches!(source, Source::Register(_));
    assert_eq!(cpu.complete_cycle(Some(opcode)).expect("opcode"), register);
    if !register {
        assert!(cpu.complete_cycle(Some(right)).expect("operand"));
    }
}

#[test]
fn carry_arithmetic_decodes_each_register_memory_and_immediate_source() {
    for (adc, sbc, source) in CASES {
        match source {
            Source::Register(register) => {
                assert!(
                    matches!(DecodedOpcode::from(adc), DecodedOpcode::Arithmetic { kind: ArithmeticKind::Add { carry: true }, operand: Operand::Target(Target::Register(actual)) } if actual == register)
                );
                assert!(
                    matches!(DecodedOpcode::from(sbc), DecodedOpcode::Arithmetic { kind: ArithmeticKind::Sub { carry: true }, operand: Operand::Target(Target::Register(actual)) } if actual == register)
                );
            }
            Source::Memory => {
                assert!(matches!(
                    DecodedOpcode::from(adc),
                    DecodedOpcode::Arithmetic {
                        kind: ArithmeticKind::Add { carry: true },
                        operand: Operand::Target(Target::IndirectHL)
                    }
                ));
                assert!(matches!(
                    DecodedOpcode::from(sbc),
                    DecodedOpcode::Arithmetic {
                        kind: ArithmeticKind::Sub { carry: true },
                        operand: Operand::Target(Target::IndirectHL)
                    }
                ));
            }
            Source::Immediate => {
                assert!(matches!(
                    DecodedOpcode::from(adc),
                    DecodedOpcode::Arithmetic {
                        kind: ArithmeticKind::Add { carry: true },
                        operand: Operand::Immediate
                    }
                ));
                assert!(matches!(
                    DecodedOpcode::from(sbc),
                    DecodedOpcode::Arithmetic {
                        kind: ArithmeticKind::Sub { carry: true },
                        operand: Operand::Immediate
                    }
                ));
            }
        }
    }
}

fn all_pairs(add: bool) {
    for (adc, sbc, source) in CASES
        .into_iter()
        .filter(|(_, _, source)| *source != Source::Register(Register::A))
    {
        let opcode = if add { adc } else { sbc };
        for left in 0u8..=255 {
            for right in 0u8..=255 {
                for carry in [false, true] {
                    let flags = ((left ^ right) & 0xE0) | if carry { 0x10 } else { 0 };
                    let mut cpu = machine(left, right, source, flags, 0x0100);
                    let mut expected = reference(cpu.snapshot(), left, right, carry, add);
                    expected.pc += if source == Source::Immediate { 2 } else { 1 };
                    finish(&mut cpu, opcode, source, right);
                    assert_eq!(
                        cpu.snapshot(),
                        expected,
                        "opcode {opcode:02X}, A {left:02X}, source {right:02X}, carry {carry}"
                    );
                }
            }
        }
    }
}

#[test]
fn adc_all_operand_pairs_and_carry_inputs_match_independent_reference() {
    all_pairs(true);
}

#[test]
fn sbc_all_operand_pairs_and_carry_inputs_match_independent_reference() {
    all_pairs(false);
}

#[test]
fn self_source_uses_original_a_twice_for_every_value_and_initial_flags() {
    let source = Source::Register(Register::A);
    for (opcode, add) in [(0x8F, true), (0x9F, false)] {
        for value in 0u8..=255 {
            for combination in 0u8..16 {
                let mut cpu = machine(value, value, source, combination << 4, 0x0100);
                let mut expected =
                    reference(cpu.snapshot(), value, value, combination & 1 != 0, add);
                expected.pc += 1;
                finish(&mut cpu, opcode, source, value);
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}

#[test]
fn pending_carry_reads_preserve_captured_inputs_across_retry_and_pc_wrap() {
    for (adc, sbc, source) in CASES
        .into_iter()
        .filter(|(_, _, source)| !matches!(source, Source::Register(_)))
    {
        for (opcode, add) in [(adc, true), (sbc, false)] {
            for start in [0xFFFEu16, 0xFFFF] {
                for combination in 0u8..16 {
                    let carry = combination & 1 != 0;
                    let mut cpu = machine(0x42, 0x0F, source, combination << 4, start);
                    let before = cpu.snapshot();
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start)
                    );
                    assert_eq!(cpu.snapshot(), before);
                    assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode"));
                    let mut pending = before;
                    pending.pc = pending.pc.wrapping_add(1);
                    assert_eq!(cpu.snapshot(), pending);
                    // Deliberate perturbation checks pending-state captures;
                    // ordinary instruction flow leaves live A/flags alone.
                    cpu.registers.a = 0x99;
                    cpu.registers.set_hl(0xD456);
                    cpu.registers.set_flag(Flag::C, !carry);
                    pending = cpu.snapshot();
                    let address = if source == Source::Immediate {
                        pending.pc
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
                        assert_eq!(cpu.snapshot(), pending);
                    }
                    let mut expected = reference(pending, 0x42, 0x0F, carry, add);
                    if source == Source::Immediate {
                        expected.pc = expected.pc.wrapping_add(1);
                    }
                    assert!(cpu.complete_cycle(Some(0x0F)).expect("retry operand"));
                    assert_eq!(cpu.snapshot(), expected);
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                    );
                }
            }
        }
    }
}

#[test]
fn carry_boundaries_ignore_stale_znh_flags_and_wrap_opcode_pc() {
    for (adc, sbc, source) in CASES {
        for (opcode, add) in [(adc, true), (sbc, false)] {
            for left in [0u8, 1, 0x0F, 0x10, 0xFE, 0xFF] {
                for right in [0u8, 1, 0x0F, 0x10, 0xFE, 0xFF] {
                    if source == Source::Register(Register::A) && left != right {
                        continue;
                    }
                    for combination in 0u8..16 {
                        let mut cpu = machine(left, right, source, combination << 4, 0xFFFF);
                        let before = cpu.snapshot();
                        assert!(matches!(
                            cpu.complete_cycle(None),
                            Err(CpuError::MissingReadValue { addr: 0xFFFF })
                        ));
                        assert_eq!(cpu.snapshot(), before);
                        let mut expected =
                            reference(before, left, right, combination & 1 != 0, add);
                        expected.pc = if source == Source::Immediate { 1 } else { 0 };
                        finish(&mut cpu, opcode, source, right);
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
