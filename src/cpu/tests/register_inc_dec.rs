use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers, opcode::DecodedOpcode,
    registers::RegisterValue,
};

const CASES: [(u8, u8, RegisterValue); 7] = [
    (0x04, 0x05, RegisterValue::B),
    (0x0C, 0x0D, RegisterValue::C),
    (0x14, 0x15, RegisterValue::D),
    (0x1C, 0x1D, RegisterValue::E),
    (0x24, 0x25, RegisterValue::H),
    (0x2C, 0x2D, RegisterValue::L),
    (0x3C, 0x3D, RegisterValue::A),
];

fn machine(target: RegisterValue, value: u8, flags: u8, pc: u16) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(0xA700 | u16::from(flags));
    registers.set_bc(0x1234);
    registers.set_de(0x5678);
    registers.set_hl(0xC123);
    registers.pc = pc;
    registers.sp = 0xDEF0;
    match target {
        RegisterValue::A => registers.a = value,
        RegisterValue::B => registers.b = value,
        RegisterValue::C => registers.c = value,
        RegisterValue::D => registers.d = value,
        RegisterValue::E => registers.e = value,
        RegisterValue::H => registers.h = value,
        RegisterValue::L => registers.l = value,
    }
    CPU::from_registers(registers)
}

fn expected(
    mut snapshot: CpuSnapShot,
    target: RegisterValue,
    old: u8,
    increment: bool,
) -> CpuSnapShot {
    // Widened arithmetic is an independent reference; production uses u8.
    let delta = if increment { 1i16 } else { -1 };
    let result = (i16::from(old) + delta).rem_euclid(256) as u16;
    let nibble_result = i16::from(old % 16) + delta;
    let half = !(0..16).contains(&nibble_result);
    let flags = (snapshot.af & 0x10)
        | if result == 0 { 0x80 } else { 0 }
        | if increment { 0 } else { 0x40 }
        | if half { 0x20 } else { 0 };
    snapshot.af = (snapshot.af & 0xFF00) | flags;
    match target {
        RegisterValue::A => snapshot.af = result << 8 | flags,
        RegisterValue::B => snapshot.bc = result << 8 | (snapshot.bc & 0xFF),
        RegisterValue::C => snapshot.bc = (snapshot.bc & 0xFF00) | result,
        RegisterValue::D => snapshot.de = result << 8 | (snapshot.de & 0xFF),
        RegisterValue::E => snapshot.de = (snapshot.de & 0xFF00) | result,
        RegisterValue::H => snapshot.hl = result << 8 | (snapshot.hl & 0xFF),
        RegisterValue::L => snapshot.hl = (snapshot.hl & 0xFF00) | result,
    }
    snapshot.pc = snapshot.pc.wrapping_add(1);
    snapshot
}

#[test]
fn inc_dec_opcode_mapping_is_explicit_for_every_register() {
    for (inc, dec, target) in CASES {
        assert!(
            matches!(DecodedOpcode::from(inc), DecodedOpcode::Increment(register) if register == target)
        );
        assert!(
            matches!(DecodedOpcode::from(dec), DecodedOpcode::Decrement(register) if register == target)
        );
    }
    for unsupported in [0x03, 0x0B, 0x76] {
        assert!(matches!(
            DecodedOpcode::from(unsupported),
            DecodedOpcode::Unknown
        ));
    }
}

fn check_every_value_and_flags(increment: bool) {
    for (inc, dec, target) in CASES {
        let opcode = if increment { inc } else { dec };
        for value in 0u8..=255 {
            for combination in 0u8..16 {
                let mut cpu = machine(target, value, combination << 4, 0x0100);
                let reference = expected(cpu.snapshot(), target, value, increment);
                assert!(
                    cpu.complete_cycle(Some(opcode))
                        .expect("register arithmetic")
                );
                assert_eq!(
                    cpu.snapshot(),
                    reference,
                    "opcode {opcode:02X}, value {value:02X}, flags {combination:X}"
                );
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0x0101 })
                ));
            }
        }
    }
}

#[test]
fn every_increment_value_and_flag_combination_matches_the_reference() {
    check_every_value_and_flags(true);
}

#[test]
fn every_decrement_value_and_flag_combination_matches_the_reference() {
    check_every_value_and_flags(false);
}

#[test]
fn inc_dec_opcode_reads_can_be_retried_at_the_pc_wrap_boundary() {
    for (inc, dec, target) in CASES {
        for opcode in [inc, dec] {
            // Zero is chosen here to isolate protocol/PC behavior from the
            // value/flag assertions in the exhaustive tests above.
            let mut cpu = machine(target, 0, 0x10, 0xFFFF);
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
            assert!(cpu.complete_cycle(Some(opcode)).expect("retry opcode"));
            assert_eq!(cpu.snapshot().pc, 0);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0 })
            ));
        }
    }
}
