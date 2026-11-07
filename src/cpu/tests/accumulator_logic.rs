use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers,
    opcode::{ArithmeticKind, DecodedOpcode, Operand, Target},
    registers::Register,
};

const SOURCES: [Register; 7] = [
    Register::B,
    Register::C,
    Register::D,
    Register::E,
    Register::H,
    Register::L,
    Register::A,
];
const GROUPS: [(u8, u8, ArithmeticKind); 3] = [
    (0xA0, 0xE6, ArithmeticKind::And),
    (0xA8, 0xEE, ArithmeticKind::Xor),
    (0xB0, 0xF6, ArithmeticKind::Or),
];

fn machine(a: u8, right: u8, source: usize, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(u16::from(a) * 256 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0xC123);
    r.sp = 0xDEF0;
    r.pc = pc;
    match source {
        0 => r.b = right,
        1 => r.c = right,
        2 => r.d = right,
        3 => r.e = right,
        4 => r.h = right,
        5 => r.l = right,
        7 => assert_eq!(a, right),
        _ => {}
    }
    CPU::from_registers(r)
}

fn reference(mut snapshot: CpuSnapShot, a: u8, right: u8, group: usize, bytes: u16) -> CpuSnapShot {
    // Independent boolean truth tables, rather than production bytewise operations.
    let mut result = 0u16;
    for bit in 0..8 {
        let l = !(a / (1 << bit)).is_multiple_of(2);
        let r = !(right / (1 << bit)).is_multiple_of(2);
        let set = match group {
            0 => l && r,
            1 => l != r,
            _ => l || r,
        };
        if set {
            result += 1 << bit;
        }
    }
    snapshot.af =
        result * 256 + if result == 0 { 0x80 } else { 0 } + if group == 0 { 0x20 } else { 0 };
    snapshot.pc = snapshot.pc.wrapping_add(bytes);
    snapshot
}

#[test]
fn logic_decodes_all_twenty_seven_forms() {
    for (base, immediate, kind) in GROUPS {
        for index in 0..8u8 {
            let DecodedOpcode::Arithmetic {
                kind: actual,
                operand: Operand::Target(target),
            } = DecodedOpcode::from(base + index)
            else {
                panic!("missing register/memory mapping");
            };
            assert_eq!(actual, kind);
            match target {
                Target::IndirectHL => assert_eq!(index, 6),
                Target::Register(r) => {
                    assert_eq!(r, SOURCES[if index == 7 { 6 } else { usize::from(index) }])
                }
            }
        }
        assert!(
            matches!(DecodedOpcode::from(immediate), DecodedOpcode::Arithmetic { kind: actual, operand: Operand::Immediate } if actual == kind),
            "missing immediate {immediate:02X}"
        );
    }
}

fn exhaustive(immediate: bool) {
    for (group, (base, byte, _)) in GROUPS.into_iter().enumerate() {
        for source in 0..=8 {
            if source == 7 || (source == 8) != immediate {
                continue;
            }
            let opcode = if immediate { byte } else { base + source as u8 };
            for a in 0u8..=255 {
                for right in 0u8..=255 {
                    let mut cpu = machine(a, right, source, (a ^ right) & 0xF0, 0x0100);
                    let before = cpu.snapshot();
                    let register = source < 6;
                    assert_eq!(
                        cpu.complete_cycle(Some(opcode)).expect("logic opcode"),
                        register
                    );
                    if !register {
                        let mut pending = before;
                        pending.pc += 1;
                        assert_eq!(cpu.snapshot(), pending);
                        let expected_addr = if immediate { 0x0101 } else { 0xC123 };
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected_addr)
                        );
                        assert!(cpu.complete_cycle(Some(right)).unwrap());
                    }
                    let expected =
                        reference(before, a, right, group, if immediate { 2 } else { 1 });
                    assert_eq!(
                        cpu.snapshot(),
                        expected,
                        "opcode {opcode:02X}, A={a:02X}, operand={right:02X}"
                    );
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                    );
                }
            }
        }
    }
}

#[test]
fn logic_register_and_memory_pairs_match_independent_oracle() {
    exhaustive(false);
}

#[test]
fn logic_immediate_pairs_match_independent_oracle() {
    exhaustive(true);
}

#[test]
fn logic_self_sources_overwrite_every_initial_flag_combination() {
    for (group, (base, _, _)) in GROUPS.into_iter().enumerate() {
        for a in 0u8..=255 {
            for flags in 0..16 {
                let mut cpu = machine(a, a, 7, flags * 16, 0xFFFF);
                let before = cpu.snapshot();
                assert!(matches!(
                    cpu.complete_cycle(None),
                    Err(CpuError::MissingReadValue { addr: 0xFFFF })
                ));
                assert_eq!(cpu.snapshot(), before);
                assert!(cpu.complete_cycle(Some(base + 7)).unwrap());
                assert_eq!(cpu.snapshot(), reference(before, a, a, group, 1));
            }
        }
    }
}

#[test]
fn logic_pending_reads_capture_operands_and_retry_without_mutation() {
    for (group, (base, immediate, _)) in GROUPS.into_iter().enumerate() {
        for (opcode, source) in [(base + 6, 6), (immediate, 8)] {
            for flags in 0..16 {
                let mut cpu = machine(0x96, 0x69, source, flags * 16, 0xFFFF);
                let before = cpu.snapshot();
                assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                let address = if source == 8 { 0 } else { 0xC123 };
                cpu.registers.a = 0xFF;
                cpu.registers.set_hl(0xD234);
                let pending = cpu.snapshot();
                for _ in 0..2 {
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == address)
                    );
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == address)
                    );
                    assert_eq!(cpu.snapshot(), pending);
                }
                assert!(cpu.complete_cycle(Some(0x69)).unwrap());
                let mut expected =
                    reference(before, 0x96, 0x69, group, if source == 8 { 2 } else { 1 });
                expected.hl = 0xD234;
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}

#[test]
fn logic_boundaries_ignore_initial_flags_and_wrap_program_counter() {
    for (group, (base, immediate, _)) in GROUPS.into_iter().enumerate() {
        for source in 0..9 {
            for a in [0, 1, 0x0F, 0x80, 0xFF] {
                for operand in [0, 1, 0xF0, 0x80, 0xFF] {
                    let right = if source == 7 { a } else { operand };
                    for flags in 0..16 {
                        for pc in [0xFFFE, 0xFFFF] {
                            let opcode = if source == 8 {
                                immediate
                            } else {
                                base + source as u8
                            };
                            let mut cpu = machine(a, right, source, flags * 16, pc);
                            let before = cpu.snapshot();
                            assert!(
                                matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pc)
                            );
                            assert_eq!(cpu.snapshot(), before);
                            for _ in 0..2 {
                                assert!(
                                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pc)
                                );
                                assert_eq!(cpu.snapshot(), before);
                            }
                            let register = source < 6 || source == 7;
                            assert_eq!(cpu.complete_cycle(Some(opcode)).unwrap(), register);
                            if !register {
                                assert!(cpu.complete_cycle(Some(right)).unwrap());
                            }
                            assert_eq!(
                                cpu.snapshot(),
                                reference(before, a, right, group, if source == 8 { 2 } else { 1 })
                            );
                        }
                    }
                }
            }
        }
    }
}
