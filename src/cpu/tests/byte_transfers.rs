use crate::cpu::{
    CPU, CpuError, CpuSnapShot, CycleAction, Registers,
    opcode::{DecodedOpcode, Target},
    registers::Register,
};

// Independent reference matrix: rows are destinations, columns are sources,
// in B,C,D,E,H,L,(HL),A order. The HALT cell is checked separately.
const OPCODES: [[u8; 8]; 8] = [
    [0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47],
    [0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F],
    [0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57],
    [0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F],
    [0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67],
    [0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F],
    [0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77],
    [0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F],
];

fn initial() -> CpuSnapShot {
    CpuSnapShot {
        af: 0xA7B0,
        bc: 0x1234,
        de: 0x5678,
        hl: 0xC123,
        pc: 0x0100,
        sp: 0xDEF0,
    }
}

fn machine(snapshot: CpuSnapShot) -> CPU {
    let mut registers = Registers::dmg_post_boot(1);
    registers.set_af(snapshot.af);
    registers.set_bc(snapshot.bc);
    registers.set_de(snapshot.de);
    registers.set_hl(snapshot.hl);
    registers.pc = snapshot.pc;
    registers.sp = snapshot.sp;
    CPU::from_registers(registers)
}

fn byte(snapshot: CpuSnapShot, index: usize) -> u8 {
    match index {
        0 => (snapshot.bc >> 8) as u8,
        1 => snapshot.bc as u8,
        2 => (snapshot.de >> 8) as u8,
        3 => snapshot.de as u8,
        4 => (snapshot.hl >> 8) as u8,
        5 => snapshot.hl as u8,
        7 => (snapshot.af >> 8) as u8,
        _ => panic!("memory has no register byte"),
    }
}

fn with_byte(mut snapshot: CpuSnapShot, index: usize, value: u8) -> CpuSnapShot {
    let value = u16::from(value);
    match index {
        0 => snapshot.bc = value << 8 | (snapshot.bc & 0xFF),
        1 => snapshot.bc = (snapshot.bc & 0xFF00) | value,
        2 => snapshot.de = value << 8 | (snapshot.de & 0xFF),
        3 => snapshot.de = (snapshot.de & 0xFF00) | value,
        4 => snapshot.hl = value << 8 | (snapshot.hl & 0xFF),
        5 => snapshot.hl = (snapshot.hl & 0xFF00) | value,
        7 => snapshot.af = value << 8 | (snapshot.af & 0xFF),
        _ => panic!("memory is not a register destination"),
    }
    snapshot
}

#[test]
fn explicit_transfer_matrix_decodes_every_direction_and_excludes_halt() {
    let registers = [
        Some(Register::B),
        Some(Register::C),
        Some(Register::D),
        Some(Register::E),
        Some(Register::H),
        Some(Register::L),
        None,
        Some(Register::A),
    ];
    for (destination, row) in OPCODES.iter().enumerate() {
        for (source, &opcode) in row.iter().enumerate() {
            match (
                registers[destination],
                registers[source],
                DecodedOpcode::from(opcode),
            ) {
                (
                    Some(expected_destination),
                    Some(expected_source),
                    DecodedOpcode::Load {
                        source: Target::Register(source),
                        destination: Target::Register(destination),
                    },
                ) => {
                    assert_eq!(
                        (destination, source),
                        (expected_destination, expected_source),
                        "opcode {opcode:02X}"
                    );
                }
                (
                    Some(expected),
                    None,
                    DecodedOpcode::Load {
                        destination: Target::Register(destination),
                        source: Target::IndirectHL,
                    },
                ) => {
                    assert_eq!(destination, expected)
                }
                (
                    None,
                    Some(expected),
                    DecodedOpcode::Load {
                        destination: Target::IndirectHL,
                        source: Target::Register(from),
                    },
                ) => {
                    assert_eq!(from, expected)
                }
                (None, None, DecodedOpcode::Unknown) => assert_eq!(opcode, 0x76),
                _ => panic!("wrong operation for opcode {opcode:02X}"),
            }
        }
    }
}

#[test]
fn every_register_pair_copies_every_byte_including_self_copies() {
    for (destination, row) in OPCODES.iter().enumerate().filter(|(index, _)| *index != 6) {
        for (source, &opcode) in row.iter().enumerate().filter(|(index, _)| *index != 6) {
            for value in 0u8..=255 {
                let starting = with_byte(initial(), source, value);
                let mut cpu = machine(starting);
                assert!(cpu.complete_cycle(Some(opcode)).expect("register copy"));
                let mut expected = with_byte(starting, destination, value);
                expected.pc += 1;
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "opcode {opcode:02X}, value {value:02X}"
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
fn every_memory_read_loads_every_byte_using_the_original_hl_address() {
    for (destination, row) in OPCODES.iter().enumerate().filter(|(index, _)| *index != 6) {
        for value in 0u8..=255 {
            let mut cpu = machine(initial());
            assert!(!cpu.complete_cycle(Some(row[6])).expect("load opcode"));
            let mut pending = initial();
            pending.pc += 1;
            assert_eq!(cpu.snapshot(), pending);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0xC123 })
            ));
            assert!(cpu.complete_cycle(Some(value)).expect("memory read"));
            assert_eq!(cpu.snapshot(), with_byte(pending, destination, value));
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0x0101 })
            ));
        }
    }
}

#[test]
fn every_memory_store_captures_every_source_byte_including_h_and_l() {
    for (source, &opcode) in OPCODES[6]
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != 6)
    {
        for value in 0u8..=255 {
            let mut expected = with_byte(initial(), source, value);
            let original_address = expected.hl;
            let mut cpu = machine(expected);
            assert!(!cpu.complete_cycle(Some(opcode)).expect("store opcode"));
            expected.pc += 1;
            assert_eq!(cpu.snapshot(), expected);
            assert!(
                matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr, value: actual }) if addr == original_address && actual == value)
            );
            assert!(cpu.complete_cycle(None).expect("memory write"));
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0x0101 })
            ));
        }
    }
}

#[test]
fn all_transfers_preserve_flags_wrap_pc_and_retry_missing_reads() {
    for (destination, row) in OPCODES.iter().enumerate() {
        for (source, &opcode) in row.iter().enumerate() {
            if opcode == 0x76 {
                continue;
            }
            for flags in 0u16..16 {
                let mut expected = initial();
                expected.pc = 0xFFFF;
                expected.af = (expected.af & 0xFF00) | flags << 4;
                let mut cpu = machine(expected);
                assert!(matches!(
                    cpu.complete_cycle(None),
                    Err(CpuError::MissingReadValue { addr: 0xFFFF })
                ));
                assert_eq!(cpu.snapshot(), expected);
                let complete = cpu.complete_cycle(Some(opcode)).expect("retry opcode");
                expected.pc = 0;
                if source == 6 {
                    assert!(!complete);
                    for _ in 0..2 {
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Read { addr: 0xC123 })
                        ));
                        assert!(matches!(
                            cpu.complete_cycle(None),
                            Err(CpuError::MissingReadValue { addr: 0xC123 })
                        ));
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    assert!(cpu.complete_cycle(Some(0x9B)).expect("retry memory read"));
                    expected = with_byte(expected, destination, 0x9B);
                } else if destination == 6 {
                    assert!(!complete);
                    let value = byte(expected, source);
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value: actual }) if actual == value)
                    );
                    assert!(cpu.complete_cycle(None).expect("memory write"));
                } else {
                    assert!(complete);
                    expected = with_byte(expected, destination, byte(expected, source));
                }
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "opcode {opcode:02X}, flags {flags:X}"
                );
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0 })
                ));
            }
        }
    }
}
