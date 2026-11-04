use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers,
    opcode::{ArithmeticKind, DecodedOpcode, Operand, Target},
};

const CASES: [(u8, bool, bool); 4] = [
    (0x86, true, false),
    (0x96, false, false),
    (0xC6, true, true),
    (0xD6, false, true),
];

fn machine(left: u8, flags: u8, pc: u16) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(u16::from(left) << 8 | u16::from(flags));
    registers.set_bc(0x1234);
    registers.set_de(0x5678);
    registers.set_hl(0xC123);
    registers.pc = pc;
    registers.sp = 0xDEF0;
    CPU::from_registers(registers)
}

fn reference(mut snapshot: CpuSnapShot, left: u8, right: u8, add: bool) -> CpuSnapShot {
    let sign = if add { 1i16 } else { -1 };
    let full = i16::from(left) + sign * i16::from(right);
    let low = i16::from(left % 16) + sign * i16::from(right % 16);
    let result = full.rem_euclid(256) as u16;
    let flags = if result == 0 { 0x80 } else { 0 }
        | if add { 0 } else { 0x40 }
        | if !(0..16).contains(&low) { 0x20 } else { 0 }
        | if !(0..256).contains(&full) { 0x10 } else { 0 };
    snapshot.af = result << 8 | flags;
    snapshot
}

#[test]
fn operand_source_mappings_are_explicit() {
    assert!(matches!(
        DecodedOpcode::from(0x86),
        DecodedOpcode::Arithmetic {
            kind: ArithmeticKind::Add { carry: false },
            operand: Operand::Target(Target::IndirectHL)
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0x96),
        DecodedOpcode::Arithmetic {
            kind: ArithmeticKind::Sub { carry: false },
            operand: Operand::Target(Target::IndirectHL)
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0xC6),
        DecodedOpcode::Arithmetic {
            kind: ArithmeticKind::Add { carry: false },
            operand: Operand::Immediate
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0xD6),
        DecodedOpcode::Arithmetic {
            kind: ArithmeticKind::Sub { carry: false },
            operand: Operand::Immediate
        }
    ));
}

#[test]
fn all_operand_pairs_match_reference_and_finish_at_the_correct_pc() {
    for (opcode, add, immediate) in CASES {
        for left in 0u8..=255 {
            for right in 0u8..=255 {
                let mut cpu = machine(left, (left ^ right) & 0xF0, 0x0100);
                let before = cpu.snapshot();
                cpu.complete_cycle(Some(opcode)).expect("opcode read");
                let mut pending = before;
                pending.pc += 1;
                assert_eq!(
                    cpu.snapshot(),
                    pending,
                    "opcode {opcode:02X} must retain A/flags while awaiting its operand"
                );
                let address = if immediate { 0x0101 } else { 0xC123 };
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == address)
                );
                assert!(cpu.complete_cycle(Some(right)).expect("operand read"));
                let mut expected = reference(pending, left, right, add);
                if immediate {
                    expected.pc += 1;
                }
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "opcode {opcode:02X}, left {left:02X}, right {right:02X}"
                );
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc),
                    "arithmetic completion must return to opcode fetch"
                );
            }
        }
    }
}

#[test]
fn retries_wrap_pc_and_preserve_captured_a_address_and_initial_flags() {
    for (opcode, add, immediate) in CASES {
        for start in [0xFFFEu16, 0xFFFF] {
            for combination in 0u8..16 {
                let mut cpu = machine(0x42, combination << 4, start);
                let before = cpu.snapshot();
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start)
                );
                assert_eq!(cpu.snapshot(), before);
                assert!(!cpu.complete_cycle(Some(opcode)).expect("retry opcode"));
                let mut pending = before;
                pending.pc = pending.pc.wrapping_add(1);
                assert_eq!(cpu.snapshot(), pending);
                // Deliberately perturb live registers to test the pending
                // state's captures independently of ordinary instruction flow.
                cpu.registers.a = 0x99;
                cpu.registers.set_hl(0xD456);
                pending = cpu.snapshot();
                let address = if immediate { pending.pc } else { 0xC123 };
                for _ in 0..2 {
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == address)
                    );
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr: actual }) if actual == address)
                    );
                    assert_eq!(cpu.snapshot(), pending);
                }
                assert!(cpu.complete_cycle(Some(0x10)).expect("retry operand"));
                let mut expected = reference(pending, 0x42, 0x10, add);
                if immediate {
                    expected.pc = expected.pc.wrapping_add(1);
                }
                assert_eq!(cpu.snapshot(), expected);
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                );
            }
        }
    }
}

#[test]
fn boundary_operands_overwrite_every_initial_flag_combination_for_each_source_kind() {
    for (opcode, add, immediate) in CASES {
        for left in [0u8, 1, 0x0F, 0x10, 0x7F, 0x80, 0xFF] {
            for right in [0u8, 1, 0x0F, 0x10, 0x7F, 0x80, 0xFF] {
                for combination in 0u8..16 {
                    let mut cpu = machine(left, combination << 4, 0x0100);
                    let mut expected = cpu.snapshot();
                    expected.pc += if immediate { 2 } else { 1 };
                    expected = reference(expected, left, right, add);
                    assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode"));
                    assert!(cpu.complete_cycle(Some(right)).expect("operand"));
                    assert_eq!(cpu.snapshot(), expected);
                }
            }
        }
    }
}
