use super::{CPU, CpuError, CycleAction, Registers};
use crate::cpu::opcode::DecodedOpcode;

fn machine(a: u8, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(u16::from(a) * 256 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = 0xDEF0;
    r.pc = pc;
    CPU::from_registers(r)
}

#[test]
fn decimal_adjust_opcode_mapping_is_explicit() {
    assert_eq!(DecodedOpcode::from(0x27), DecodedOpcode::DecimalAdjust);
}

#[test]
fn decimal_adjust_wraps_addition_correction() {
    let mut cpu = machine(0x9A, 0, 0x0100);
    assert!(cpu.complete_cycle(Some(0x27)).unwrap());
    assert_eq!(cpu.snapshot().af, 0x0090);
}

#[test]
fn decimal_adjust_wraps_subtraction_correction() {
    // Include a non-BCD-derived input: opcode behavior is defined for all states.
    let mut cpu = machine(0, 0x70, 0x0100);
    assert!(cpu.complete_cycle(Some(0x27)).unwrap());
    assert_eq!(cpu.snapshot().af, 0x9A50);
}

#[test]
fn decimal_adjust_all_input_states_preserve_contract_and_retry_atomicity() {
    for a in 0u8..=255 {
        for flags in (0u8..=0xF0).step_by(16) {
            for pc in [0x0100, 0xFFFF] {
                let mut cpu = machine(a, flags, pc);
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
                // Classify the decimal nibbles/flags, then perform a single widened
                // signed correction. Decimal arithmetic below is a separate oracle
                // for states reachable from valid packed-BCD operations.
                let subtract = flags & 0x40 != 0;
                let half = flags & 0x20 != 0;
                let old_carry = flags & 0x10 != 0;
                let (low, high) = if subtract {
                    (half, old_carry)
                } else {
                    (half || a % 16 >= 10, old_carry || a >= 0x9A)
                };
                let correction = i16::from(low) * 6 + i16::from(high) * 96;
                let adjusted = (i16::from(a) + if subtract { -correction } else { correction })
                    .rem_euclid(256) as u16;
                let mut expected = before;
                expected.af = adjusted * 256
                    + u16::from(flags & 0x40)
                    + if adjusted == 0 { 0x80 } else { 0 }
                    + if high { 0x10 } else { 0 };
                expected.pc = pc.wrapping_add(1);
                assert!(cpu.complete_cycle(Some(0x27)).unwrap());
                assert_eq!(cpu.snapshot(), expected, "A={a:02X} F={flags:02X}");
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                );
            }
        }
    }
}

fn bcd(decimal: u16) -> u8 {
    ((decimal / 10) * 16 + decimal % 10) as u8
}

#[test]
fn decimal_adjust_after_all_valid_bcd_add_adc_sub_sbc_pairs() {
    for (opcode, subtract, uses_carry) in [
        (0xC6, false, false),
        (0xCE, false, true),
        (0xD6, true, false),
        (0xDE, true, true),
    ] {
        for left in 0u16..100 {
            for right in 0u16..100 {
                for incoming in [false, true] {
                    let mut cpu = machine(bcd(left), if incoming { 0x10 } else { 0 }, 0x0100);
                    assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                    assert!(cpu.complete_cycle(Some(bcd(right))).unwrap());
                    let before = cpu.snapshot();
                    let operand = right as i16 + i16::from(incoming && uses_carry);
                    let decimal = left as i16 + if subtract { -operand } else { operand };
                    let carry = !(0..100).contains(&decimal);
                    let value = bcd(decimal.rem_euclid(100) as u16);
                    let mut expected = before;
                    expected.af = u16::from(value) * 256
                        + if value == 0 { 0x80 } else { 0 }
                        + if subtract { 0x40 } else { 0 }
                        + if carry { 0x10 } else { 0 };
                    expected.pc += 1;
                    assert!(cpu.complete_cycle(Some(0x27)).unwrap());
                    assert_eq!(
                        cpu.snapshot(),
                        expected,
                        "opcode {opcode:02X}, {left} and {right}, incoming={incoming}"
                    );
                }
            }
        }
    }
}
