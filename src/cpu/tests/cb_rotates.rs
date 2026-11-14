use super::{CPU, CpuError, CycleAction, Registers};

fn cpu_with_a(value: u8, flags: u8) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(u16::from(value) * 256 + u16::from(flags));
    registers.set_hl(0xC123);
    CPU::from_registers(registers)
}

fn fetch_prefix(cpu: &mut CPU) {
    assert!(!cpu.complete_cycle(Some(0xCB)).unwrap());
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
                    assert!(cpu.complete_cycle(Some(group * 8 + target)).unwrap());
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
                assert!(cpu.complete_cycle(Some(group * 8 + 7)).unwrap());
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
                assert!(!cpu.complete_cycle(Some(group * 8 + 6)).unwrap());
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0xC123 })
                ));
                assert!(!cpu.complete_cycle(Some(value)).unwrap());
                let expected = reference(group, value, carry).0;
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value }) if value == expected),
                    "group={group}, value={value:02X}, carry={carry}, expected={expected:02X}"
                );
                assert!(cpu.complete_cycle(None).unwrap());
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
                assert!(!cpu.complete_cycle(Some(group * 8 + 6)).unwrap());
                assert!(!cpu.complete_cycle(Some(value)).unwrap());
                assert!(cpu.complete_cycle(None).unwrap());
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
        assert!(!cpu.complete_cycle(Some(0xCB)).unwrap());
        expected.pc = pc.wrapping_add(1);
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == expected.pc)
        );
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
        );
        assert!(cpu.complete_cycle(Some(0x07)).unwrap()); // RLC A
        expected.pc = pc.wrapping_add(2);
        expected.af = 0x8400;
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
        );
        assert!(cpu.complete_cycle(Some(0)).unwrap());
    }
}

#[test]
fn cb_rotate_memory_retries_and_writes_to_captured_address_on_fourth_cycle() {
    let mut cpu = cpu_with_a(0x55, 0);
    fetch_prefix(&mut cpu);
    assert!(!cpu.complete_cycle(Some(0x06)).unwrap()); // RLC (HL)
    let before = cpu.snapshot();
    assert!(matches!(
        cpu.complete_cycle(None),
        Err(CpuError::MissingReadValue { addr: 0xC123 })
    ));
    assert_eq!(cpu.snapshot(), before);
    cpu.registers.set_hl(0xD456);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0xC123 })
    ));
    assert!(!cpu.complete_cycle(Some(0x42)).unwrap());
    for _ in 0..2 {
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Write {
                addr: 0xC123,
                value: 0x84
            })
        ));
    }
    assert!(cpu.complete_cycle(None).unwrap());
    assert_eq!(cpu.snapshot().pc, 0x0102);
    assert_eq!(cpu.snapshot().hl, 0xD456);
    assert_eq!(cpu.snapshot().af, 0x5500);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0102 })
    ));
}

#[test]
fn cb_unsupported_decode_is_fallible_for_every_unimplemented_byte() {
    use crate::cpu::opcode::DecodedCbOpcode;
    for byte in 0u8..=255 {
        assert_eq!(
            DecodedCbOpcode::try_from(byte).is_ok(),
            byte < 0x20,
            "CB {byte:02X}"
        );
    }
}

#[test]
fn cb_unsupported_fault_is_sticky_and_does_not_request_memory() {
    for byte in [0x20, 0x26, 0x46, 0x86, 0xC6, 0xFF] {
        let mut cpu = cpu_with_a(0x55, 0xF0);
        fetch_prefix(&mut cpu);
        let first = match cpu.complete_cycle(Some(byte)) {
            Err(CpuError::UnsupportedOpcode(fault)) => fault,
            other => panic!("expected CB fault, got {other:?}"),
        };
        assert_eq!(first.opcode, crate::cpu::OpcodeEncoding::Cb(byte));
        let after = cpu.snapshot();
        for _ in 0..2 {
            match cpu.request_cycle() {
                Err(CpuError::UnsupportedOpcode(fault)) => {
                    assert_eq!((fault.opcode, fault.addr), (first.opcode, first.addr));
                }
                other => panic!("expected sticky CB fault, got {other:?}"),
            }
            match cpu.complete_cycle(Some(0x07)) {
                Err(CpuError::UnsupportedOpcode(fault)) => {
                    assert_eq!((fault.opcode, fault.addr), (first.opcode, first.addr));
                }
                other => panic!("expected sticky CB fault, got {other:?}"),
            }
            assert_eq!(cpu.snapshot(), after);
        }
    }
}

#[test]
fn cb_unsupported_fault_reports_prefix_address_and_consumes_both_bytes() {
    for pc in [0x0100u16, 0xFFFE, 0xFFFF] {
        let mut cpu = cpu_with_a(0x55, 0xF0);
        cpu.registers.pc = pc;
        let mut expected = cpu.snapshot();
        assert!(!cpu.complete_cycle(Some(0xCB)).unwrap());
        let fault = match cpu.complete_cycle(Some(0x26)) {
            Err(CpuError::UnsupportedOpcode(fault)) => fault,
            other => panic!("expected CB fault, got {other:?}"),
        };
        assert_eq!(fault.opcode, crate::cpu::OpcodeEncoding::Cb(0x26));
        expected.pc = pc.wrapping_add(2);
        assert_eq!(cpu.snapshot(), expected);
        assert_eq!(
            fault.addr, pc,
            "fault should identify the instruction start"
        );
    }
}
