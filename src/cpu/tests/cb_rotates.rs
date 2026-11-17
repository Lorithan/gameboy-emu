use crate::outcome_support::instruction_complete;

use super::{CPU, CpuError, CycleAction, Registers};

fn cpu_with_a(value: u8, flags: u8) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(u16::from(value) * 256 + u16::from(flags));
    registers.set_hl(0xC123);
    CPU::from_registers(registers)
}

fn fetch_prefix(cpu: &mut CPU) {
    assert!(
        !cpu.complete_cycle(Some(0xCB))
            .map(instruction_complete)
            .unwrap()
    );
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0101 })
    ));
}

// Independent arithmetic oracle for RLC, RRC, RL, RR.
fn reference(group: u8, value: u8, carry: bool) -> (u8, bool) {
    let value = u16::from(value);
    match group {
        0 => (((value * 2) % 256 + value / 128) as u8, value >= 128),
        1 => ((value / 2 + (value % 2) * 128) as u8, value % 2 != 0),
        2 => (((value * 2) % 256 + u16::from(carry)) as u8, value >= 128),
        3 => ((value / 2 + u16::from(carry) * 128) as u8, value % 2 != 0),
        _ => panic!("not a rotate group"),
    }
}

#[test]
fn cb_rotate_register_results_exhaust_values_and_carry() {
    // All seven register targets, checked independently of Target::from_bits.
    for target in [0u8, 1, 2, 3, 4, 5, 7] {
        for group in 0u8..4 {
            for value in 0u8..=255 {
                for carry in [false, true] {
                    let mut cpu = cpu_with_a(0x55, if carry { 0x10 } else { 0 });
                    match target {
                        0 => cpu.registers.b = value,
                        1 => cpu.registers.c = value,
                        2 => cpu.registers.d = value,
                        3 => cpu.registers.e = value,
                        4 => cpu.registers.h = value,
                        5 => cpu.registers.l = value,
                        7 => cpu.registers.a = value,
                        _ => unreachable!(),
                    }
                    fetch_prefix(&mut cpu);
                    assert!(
                        cpu.complete_cycle(Some(group * 8 + target))
                            .map(instruction_complete)
                            .unwrap()
                    );
                    let actual = match target {
                        0 => cpu.registers.b,
                        1 => cpu.registers.c,
                        2 => cpu.registers.d,
                        3 => cpu.registers.e,
                        4 => cpu.registers.h,
                        5 => cpu.registers.l,
                        7 => cpu.registers.a,
                        _ => unreachable!(),
                    };
                    assert_eq!(
                        actual,
                        reference(group, value, carry).0,
                        "group={group}, target={target}, value={value:02X}, carry={carry}"
                    );
                }
            }
        }
    }
}

#[test]
fn cb_rotate_register_flags_exhaust_values_and_initial_flags() {
    for group in 0u8..4 {
        for value in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = cpu_with_a(value, flags);
                fetch_prefix(&mut cpu);
                assert!(
                    cpu.complete_cycle(Some(group * 8 + 7))
                        .map(instruction_complete)
                        .unwrap()
                );
                let (result, carry) = reference(group, value, flags & 0x10 != 0);
                let expected_flags =
                    if result == 0 { 0x80 } else { 0 } | if carry { 0x10 } else { 0 };
                assert_eq!(
                    cpu.snapshot().af & 0xFF,
                    expected_flags,
                    "group={group}, value={value:02X}, flags={flags:02X}"
                );
            }
        }
    }
}

#[test]
fn cb_rotate_memory_results_exhaust_values_and_carry() {
    for group in 0u8..4 {
        for value in 0u8..=255 {
            for carry in [false, true] {
                let mut cpu = cpu_with_a(0x55, if carry { 0x10 } else { 0 });
                fetch_prefix(&mut cpu);
                assert!(
                    !cpu.complete_cycle(Some(group * 8 + 6))
                        .map(instruction_complete)
                        .unwrap()
                );
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0xC123 })
                ));
                assert!(
                    !cpu.complete_cycle(Some(value))
                        .map(instruction_complete)
                        .unwrap()
                );
                let expected = reference(group, value, carry).0;
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value }) if value == expected),
                    "group={group}, value={value:02X}, carry={carry}, expected={expected:02X}"
                );
                assert!(cpu.complete_cycle(None).map(instruction_complete).unwrap());
            }
        }
    }
}

#[test]
fn cb_rotate_memory_flags_exhaust_values_and_initial_flags() {
    for group in 0u8..4 {
        for value in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = cpu_with_a(0x55, flags);
                fetch_prefix(&mut cpu);
                assert!(
                    !cpu.complete_cycle(Some(group * 8 + 6))
                        .map(instruction_complete)
                        .unwrap()
                );
                assert!(
                    !cpu.complete_cycle(Some(value))
                        .map(instruction_complete)
                        .unwrap()
                );
                assert!(cpu.complete_cycle(None).map(instruction_complete).unwrap());
                let (result, carry) = reference(group, value, flags & 0x10 != 0);
                let expected_flags =
                    if result == 0 { 0x80 } else { 0 } | if carry { 0x10 } else { 0 };
                assert_eq!(
                    cpu.snapshot().af & 0xFF,
                    expected_flags,
                    "group={group}, value={value:02X}, flags={flags:02X}"
                );
            }
        }
    }
}

#[test]
fn cb_rotate_fetch_wraps_retries_and_finishes_on_second_cycle() {
    for pc in [0x0100u16, 0xFFFE, 0xFFFF] {
        let mut cpu = cpu_with_a(0x42, 0);
        cpu.registers.pc = pc;
        let mut expected = cpu.snapshot();
        assert!(
            !cpu.complete_cycle(Some(0xCB))
                .map(instruction_complete)
                .unwrap()
        );
        expected.pc = pc.wrapping_add(1);
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.complete_cycle(None).map(instruction_complete), Err(CpuError::MissingReadValue { addr }) if addr == expected.pc)
        );
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
        );
        assert!(
            cpu.complete_cycle(Some(0x07))
                .map(instruction_complete)
                .unwrap()
        ); // RLC A
        expected.pc = pc.wrapping_add(2);
        expected.af = 0x8400;
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
        );
        assert!(
            cpu.complete_cycle(Some(0))
                .map(instruction_complete)
                .unwrap()
        );
    }
}

#[test]
fn cb_rotate_memory_retries_and_writes_to_captured_address_on_fourth_cycle() {
    let mut cpu = cpu_with_a(0x55, 0);
    fetch_prefix(&mut cpu);
    assert!(
        !cpu.complete_cycle(Some(0x06))
            .map(instruction_complete)
            .unwrap()
    ); // RLC (HL)
    let before = cpu.snapshot();
    assert!(matches!(
        cpu.complete_cycle(None).map(instruction_complete),
        Err(CpuError::MissingReadValue { addr: 0xC123 })
    ));
    assert_eq!(cpu.snapshot(), before);
    cpu.registers.set_hl(0xD456);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0xC123 })
    ));
    assert!(
        !cpu.complete_cycle(Some(0x42))
            .map(instruction_complete)
            .unwrap()
    );
    for _ in 0..2 {
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Write {
                addr: 0xC123,
                value: 0x84
            })
        ));
    }
    assert!(cpu.complete_cycle(None).map(instruction_complete).unwrap());
    assert_eq!(cpu.snapshot().pc, 0x0102);
    assert_eq!(cpu.snapshot().hl, 0xD456);
    assert_eq!(cpu.snapshot().af, 0x5500);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0102 })
    ));
}

#[test]
fn cb_decoder_accepts_all_bytes_and_maps_bit_families_and_targets() {
    use crate::cpu::opcode::{CbType, DecodedCbOpcode, Target};
    use crate::cpu::registers::Register;
    let targets = [
        Target::Register(Register::B),
        Target::Register(Register::C),
        Target::Register(Register::D),
        Target::Register(Register::E),
        Target::Register(Register::H),
        Target::Register(Register::L),
        Target::IndirectHL,
        Target::Register(Register::A),
    ];
    for byte in 0u8..=255 {
        let decoded = DecodedCbOpcode::try_from(byte).unwrap();
        assert_eq!(decoded.target, targets[usize::from(byte % 8)]);
        let bit = (byte % 64) / 8;
        match byte {
            0x00..=0x3F => assert!(matches!(decoded.cb_type, CbType::Transform(_))),
            0x40..=0x7F => assert_eq!(decoded.cb_type, CbType::Bit(bit)),
            0x80..=0xBF => assert_eq!(decoded.cb_type, CbType::Clear(bit)),
            0xC0..=0xFF => assert_eq!(decoded.cb_type, CbType::Set(bit)),
        }
    }
}
// Widened arithmetic oracle: independent of production shifts and masks.
fn shift_reference(opcode: u8, value: u8) -> (u8, u16) {
    let value = u16::from(value);
    let (result, carry) = match opcode {
        0x27 => ((value * 2) % 256, value >= 128),
        0x2F => (value / 2 + (value / 128) * 128, value % 2 != 0),
        0x3F => (value / 2, value % 2 != 0),
        _ => panic!("not a shift"),
    };
    let flags = if result == 0 { 0x80 } else { 0 } | if carry { 0x10 } else { 0 };
    (result as u8, flags)
}

#[test]
fn cb_shift_register_values_and_flags() {
    for opcode in [0x27, 0x2F, 0x3F] {
        // SLA A, SRA A, SRL A
        for value in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = cpu_with_a(value, flags);
                fetch_prefix(&mut cpu);
                assert!(
                    cpu.complete_cycle(Some(opcode))
                        .map(instruction_complete)
                        .unwrap()
                );
                let (result, expected_flags) = shift_reference(opcode, value);
                assert_eq!(
                    cpu.snapshot().af,
                    u16::from(result) * 256 + expected_flags,
                    "CB {opcode:02X}, input={value:02X}, flags={flags:02X}"
                );
            }
        }
    }
}

#[test]
fn cb_shift_memory_values_flags_and_four_cycle_completion() {
    for register_opcode in [0x27, 0x2F, 0x3F] {
        for value in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = cpu_with_a(0x55, flags);
                fetch_prefix(&mut cpu);
                assert!(
                    !cpu.complete_cycle(Some(register_opcode - 1))
                        .map(instruction_complete)
                        .unwrap()
                );
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0xC123 })
                ));
                assert!(
                    !cpu.complete_cycle(Some(value))
                        .map(instruction_complete)
                        .unwrap()
                );
                let (result, expected_flags) = shift_reference(register_opcode, value);
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value }) if value == result),
                    "CB {:02X}, input={value:02X}, flags={flags:02X}, expected={result:02X}",
                    register_opcode - 1
                );
                assert!(cpu.complete_cycle(None).map(instruction_complete).unwrap());
                assert_eq!(cpu.snapshot().af, 0x5500 + expected_flags);
                assert_eq!(cpu.snapshot().pc, 0x0102);
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0x0102 })
                ));
            }
        }
    }
}

#[test]
fn cb_swap_all_targets_values_flags_and_cycle_completion() {
    for target in 0u8..8 {
        for value in 0u8..=255 {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = cpu_with_a(0x55, flags);
                match target {
                    0 => cpu.registers.b = value,
                    1 => cpu.registers.c = value,
                    2 => cpu.registers.d = value,
                    3 => cpu.registers.e = value,
                    4 => cpu.registers.h = value,
                    5 => cpu.registers.l = value,
                    6 => {}
                    7 => cpu.registers.a = value,
                    _ => unreachable!(),
                }
                let mut expected = cpu.snapshot();
                // Independent decimal arithmetic exchanges the two base-16 digits.
                let result = u16::from(value % 16) * 16 + u16::from(value / 16);
                expected.af = (expected.af & 0xFF00) | if result == 0 { 0x80 } else { 0 };
                expected.pc = 0x0102;
                match target {
                    0 => expected.bc = (expected.bc & 0x00FF) | (result * 256),
                    1 => expected.bc = (expected.bc & 0xFF00) | result,
                    2 => expected.de = (expected.de & 0x00FF) | (result * 256),
                    3 => expected.de = (expected.de & 0xFF00) | result,
                    4 => expected.hl = (expected.hl & 0x00FF) | (result * 256),
                    5 => expected.hl = (expected.hl & 0xFF00) | result,
                    6 => {}
                    7 => expected.af = (result * 256) | (expected.af & 0xFF),
                    _ => unreachable!(),
                }
                fetch_prefix(&mut cpu);
                let complete = cpu
                    .complete_cycle(Some(0x30 + target))
                    .map(instruction_complete)
                    .unwrap();
                if target == 6 {
                    assert!(!complete);
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(CycleAction::Read { addr: 0xC123 })
                    ));
                    assert!(
                        !cpu.complete_cycle(Some(value))
                            .map(instruction_complete)
                            .unwrap()
                    );
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value }) if u16::from(value) == result)
                    );
                    assert!(cpu.complete_cycle(None).map(instruction_complete).unwrap());
                } else {
                    assert!(complete);
                }
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "SWAP target={target}, input={value:02X}, flags={flags:02X}"
                );
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0x0102 })
                ));
            }
        }
    }
}

#[test]
fn cb_bit_all_bits_targets_values_flags_and_read_only_completion() {
    for bit in 0u8..8 {
        for target in 0u8..8 {
            for value in 0u8..=255 {
                for flags in (0u8..=0xF0).step_by(16) {
                    let mut cpu = cpu_with_a(0x55, flags);
                    match target {
                        0 => cpu.registers.b = value,
                        1 => cpu.registers.c = value,
                        2 => cpu.registers.d = value,
                        3 => cpu.registers.e = value,
                        4 => cpu.registers.h = value,
                        5 => cpu.registers.l = value,
                        6 => {}
                        7 => cpu.registers.a = value,
                        _ => unreachable!(),
                    }
                    let mut expected = cpu.snapshot();
                    let zero = (u16::from(value) / 2u16.pow(u32::from(bit))) % 2 == 0;
                    let expected_flags =
                        u16::from(flags & 0x10) + 0x20 + if zero { 0x80 } else { 0 };
                    expected.af = (expected.af & 0xFF00) | expected_flags;
                    expected.pc = 0x0102;
                    fetch_prefix(&mut cpu);
                    let complete = cpu
                        .complete_cycle(Some(0x40 + bit * 8 + target))
                        .map(instruction_complete)
                        .unwrap();
                    if target == 6 {
                        assert!(!complete);
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Read { addr: 0xC123 })
                        ));
                        assert!(
                            cpu.complete_cycle(Some(value))
                                .map(instruction_complete)
                                .unwrap()
                        );
                    } else {
                        assert!(complete);
                    }
                    assert_eq!(
                        cpu.snapshot(),
                        expected,
                        "BIT {bit}, target={target}, value={value:02X}, flags={flags:02X}"
                    );
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(CycleAction::Read { addr: 0x0102 })
                    ));
                }
            }
        }
    }
}

#[test]
fn cb_bit_memory_wrap_retry_captured_address_and_continuation() {
    for pc in [0x0100u16, 0xFFFE, 0xFFFF] {
        for bit in 0u8..8 {
            let mut cpu = cpu_with_a(0x55, 0xF0);
            cpu.registers.pc = pc;
            assert!(
                !cpu.complete_cycle(Some(0xCB))
                    .map(instruction_complete)
                    .unwrap()
            );
            let before_decode = cpu.snapshot();
            assert!(
                matches!(cpu.complete_cycle(None).map(instruction_complete), Err(CpuError::MissingReadValue { addr }) if addr == pc.wrapping_add(1))
            );
            assert_eq!(cpu.snapshot(), before_decode);
            assert!(
                !cpu.complete_cycle(Some(0x46 + bit * 8))
                    .map(instruction_complete)
                    .unwrap()
            );
            cpu.registers.set_hl(0xD456);
            let before_read = cpu.snapshot();
            for _ in 0..2 {
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0xC123 })
                ));
                assert!(matches!(
                    cpu.complete_cycle(None).map(instruction_complete),
                    Err(CpuError::MissingReadValue { addr: 0xC123 })
                ));
                assert_eq!(cpu.snapshot(), before_read);
            }
            assert!(
                cpu.complete_cycle(Some(0))
                    .map(instruction_complete)
                    .unwrap()
            );
            let mut expected = before_read;
            expected.af = 0x55B0;
            assert_eq!(cpu.snapshot(), expected);
            assert_eq!(expected.pc, pc.wrapping_add(2));
            assert!(
                matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
            );
            assert!(
                cpu.complete_cycle(Some(0))
                    .map(instruction_complete)
                    .unwrap()
            );
            expected.pc = expected.pc.wrapping_add(1);
            assert_eq!(cpu.snapshot(), expected);
        }
    }
}

fn check_res_set(set: bool) {
    for bit in 0u8..8 {
        for target in 0u8..8 {
            for value in 0u8..=255 {
                for flags in (0u8..=0xF0).step_by(16) {
                    let mut cpu = cpu_with_a(0x55, flags);
                    match target {
                        0 => cpu.registers.b = value,
                        1 => cpu.registers.c = value,
                        2 => cpu.registers.d = value,
                        3 => cpu.registers.e = value,
                        4 => cpu.registers.h = value,
                        5 => cpu.registers.l = value,
                        6 => {}
                        7 => cpu.registers.a = value,
                        _ => unreachable!(),
                    }
                    let mut expected = cpu.snapshot();
                    // Independent arithmetic: add/subtract the bit's positional weight.
                    let weight = 2u16.pow(u32::from(bit));
                    let present = (u16::from(value) / weight) % 2 != 0;
                    let result = match (set, present) {
                        (true, false) => u16::from(value) + weight,
                        (false, true) => u16::from(value) - weight,
                        _ => u16::from(value),
                    };
                    expected.pc = 0x0102;
                    match target {
                        0 => expected.bc = (expected.bc & 0x00FF) | (result * 256),
                        1 => expected.bc = (expected.bc & 0xFF00) | result,
                        2 => expected.de = (expected.de & 0x00FF) | (result * 256),
                        3 => expected.de = (expected.de & 0xFF00) | result,
                        4 => expected.hl = (expected.hl & 0x00FF) | (result * 256),
                        5 => expected.hl = (expected.hl & 0xFF00) | result,
                        6 => {}
                        7 => expected.af = (result * 256) | u16::from(flags),
                        _ => unreachable!(),
                    }
                    fetch_prefix(&mut cpu);
                    let opcode = if set { 0xC0 } else { 0x80 } + bit * 8 + target;
                    let complete = cpu
                        .complete_cycle(Some(opcode))
                        .map(instruction_complete)
                        .unwrap();
                    if target == 6 {
                        assert!(!complete);
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Read { addr: 0xC123 })
                        ));
                        assert!(
                            !cpu.complete_cycle(Some(value))
                                .map(instruction_complete)
                                .unwrap()
                        );
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value }) if u16::from(value) == result),
                            "set={set}, bit={bit}, input={value:02X}, expected={result:02X}"
                        );
                        assert!(cpu.complete_cycle(None).map(instruction_complete).unwrap());
                    } else {
                        assert!(complete);
                    }
                    assert_eq!(
                        cpu.snapshot(),
                        expected,
                        "set={set}, bit={bit}, target={target}, input={value:02X}, flags={flags:02X}"
                    );
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(CycleAction::Read { addr: 0x0102 })
                    ));
                }
            }
        }
    }
}

#[test]
fn cb_res_all_bits_targets_values_flags_and_completion() {
    check_res_set(false);
}

#[test]
fn cb_set_all_bits_targets_values_flags_and_completion() {
    check_res_set(true);
}
