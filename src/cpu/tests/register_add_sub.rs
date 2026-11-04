use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers, opcode::DecodedOpcode,
    registers::RegisterValue,
};

const CASES: [(u8, u8, RegisterValue); 7] = [
    (0x80, 0x90, RegisterValue::B),
    (0x81, 0x91, RegisterValue::C),
    (0x82, 0x92, RegisterValue::D),
    (0x83, 0x93, RegisterValue::E),
    (0x84, 0x94, RegisterValue::H),
    (0x85, 0x95, RegisterValue::L),
    (0x87, 0x97, RegisterValue::A),
];

fn machine(left: u8, right: u8, source: RegisterValue, flags: u8, pc: u16) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(u16::from(left) << 8 | u16::from(flags));
    registers.set_bc(0x1234);
    registers.set_de(0x5678);
    registers.set_hl(0xC123);
    registers.pc = pc;
    registers.sp = 0xDEF0;
    match source {
        RegisterValue::B => registers.b = right,
        RegisterValue::C => registers.c = right,
        RegisterValue::D => registers.d = right,
        RegisterValue::E => registers.e = right,
        RegisterValue::H => registers.h = right,
        RegisterValue::L => registers.l = right,
        RegisterValue::A => assert_eq!(left, right, "self-source inputs must alias"),
    }
    CPU::from_registers(registers)
}

fn reference(mut snapshot: CpuSnapShot, left: u8, right: u8, add: bool) -> CpuSnapShot {
    // Widened signed arithmetic and nibble arithmetic provide a reference
    // independent of the production overflowing/XOR calculations.
    let sign = if add { 1i16 } else { -1 };
    let full = i16::from(left) + sign * i16::from(right);
    let low = i16::from(left % 16) + sign * i16::from(right % 16);
    let result = full.rem_euclid(256) as u16;
    let flags = if result == 0 { 0x80 } else { 0 }
        | if add { 0 } else { 0x40 }
        | if !(0..16).contains(&low) { 0x20 } else { 0 }
        | if !(0..256).contains(&full) { 0x10 } else { 0 };
    snapshot.af = result << 8 | flags;
    snapshot.pc = snapshot.pc.wrapping_add(1);
    snapshot
}

#[test]
fn add_sub_register_mappings_and_exclusions_are_explicit() {
    for (add, sub, source) in CASES {
        assert!(
            matches!(DecodedOpcode::from(add), DecodedOpcode::AddRegister(actual) if actual == source)
        );
        assert!(
            matches!(DecodedOpcode::from(sub), DecodedOpcode::SubRegister(actual) if actual == source)
        );
    }
    for byte in [0xA0, 0xA8, 0x76] {
        assert!(matches!(DecodedOpcode::from(byte), DecodedOpcode::Unknown));
    }
}

fn check_all_pairs(add: bool) {
    for (add_opcode, sub_opcode, source) in CASES
        .into_iter()
        .filter(|(_, _, source)| *source != RegisterValue::A)
    {
        let opcode = if add { add_opcode } else { sub_opcode };
        for left in 0u8..=255 {
            for right in 0u8..=255 {
                // Vary stale flags across the exhaustive operand grid.
                let flags = (left ^ right) & 0xF0;
                let mut cpu = machine(left, right, source, flags, 0x0100);
                let expected = reference(cpu.snapshot(), left, right, add);
                assert!(cpu.complete_cycle(Some(opcode)).expect("binary arithmetic"));
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "opcode {opcode:02X}, left {left:02X}, right {right:02X}"
                );
            }
        }
    }
}

#[test]
fn add_matches_the_reference_for_all_operand_pairs_and_non_a_sources() {
    check_all_pairs(true);
}

#[test]
fn sub_matches_the_reference_for_all_operand_pairs_and_non_a_sources() {
    check_all_pairs(false);
}

#[test]
fn self_source_reads_original_a_twice_and_overwrites_all_initial_flags() {
    for (opcode, add) in [(0x87, true), (0x97, false)] {
        for value in 0u8..=255 {
            for combination in 0u8..16 {
                let mut cpu = machine(value, value, RegisterValue::A, combination << 4, 0x0100);
                let expected = reference(cpu.snapshot(), value, value, add);
                assert!(
                    cpu.complete_cycle(Some(opcode))
                        .expect("self-source arithmetic")
                );
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}

#[test]
fn boundaries_overwrite_all_flags_and_opcode_retry_wraps_pc() {
    for (add_opcode, sub_opcode, source) in CASES {
        for (opcode, add) in [(add_opcode, true), (sub_opcode, false)] {
            for left in [0u8, 1, 0x0F, 0x10, 0x7F, 0x80, 0xFE, 0xFF] {
                for right in [0u8, 1, 0x0F, 0x10, 0x7F, 0x80, 0xFE, 0xFF] {
                    if source == RegisterValue::A && left != right {
                        continue;
                    }
                    for combination in 0u8..16 {
                        let mut cpu = machine(left, right, source, combination << 4, 0xFFFF);
                        let before = cpu.snapshot();
                        for _ in 0..2 {
                            assert!(matches!(
                                cpu.request_cycle(),
                                Ok(CycleAction::Read { addr: 0xFFFF })
                            ));
                            assert!(matches!(
                                cpu.complete_cycle(None),
                                Err(CpuError::MissingReadValue { addr: 0xFFFF })
                            ));
                            assert_eq!(cpu.snapshot(), before);
                        }
                        assert!(
                            cpu.complete_cycle(Some(opcode))
                                .expect("retry arithmetic opcode")
                        );
                        assert_eq!(cpu.snapshot(), reference(before, left, right, add));
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Read { addr: 0 })
                        ));
                    }
                }
            }
        }
    }
}
