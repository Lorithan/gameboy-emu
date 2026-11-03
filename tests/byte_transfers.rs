use gameboy_emu::{
    cpu::{CpuError, CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System, SystemError},
};

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
// B,C,D,E,H,L,memory,A after setup.
const VALUES: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0xC1, 0x23, 0x9B, 0xA7];

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"))
}

fn machine(opcode: u8, read_back_store: bool) -> System {
    let mut bytes = vec![
        0x06, 0x12, 0x0E, 0x34, 0x16, 0x56, 0x1E, 0x78, 0x26, 0xC1, 0x2E, 0x23, 0x3E, 0xA7, 0x36,
        0x9B, opcode,
    ];
    if read_back_store {
        bytes.push(0x7E);
    }
    bytes.push(0xD3);
    let mut system = program(&bytes);
    for _ in 0..7 {
        assert!(matches!(
            system.step_instruction(2),
            Ok(InstructionStep::Complete {
                mcycles: 2,
                ticks: 8
            })
        ));
    }
    assert!(matches!(
        system.step_instruction(3),
        Ok(InstructionStep::Complete {
            mcycles: 3,
            ticks: 12
        })
    ));
    system
}

fn expected(mut snapshot: CpuSnapShot, destination: usize, source: usize) -> CpuSnapShot {
    let value = u16::from(VALUES[source]);
    match destination {
        0 => snapshot.bc = value << 8 | (snapshot.bc & 0xFF),
        1 => snapshot.bc = (snapshot.bc & 0xFF00) | value,
        2 => snapshot.de = value << 8 | (snapshot.de & 0xFF),
        3 => snapshot.de = (snapshot.de & 0xFF00) | value,
        4 => snapshot.hl = value << 8 | (snapshot.hl & 0xFF),
        5 => snapshot.hl = (snapshot.hl & 0xFF00) | value,
        6 => {}
        7 => snapshot.af = value << 8 | (snapshot.af & 0xFF),
        _ => unreachable!(),
    }
    snapshot.pc += 1;
    snapshot
}

#[test]
fn all_transfers_have_exact_public_traces_and_preserve_unrelated_state() {
    for (destination, row) in OPCODES.iter().enumerate() {
        for (source, &opcode) in row.iter().enumerate() {
            if opcode == 0x76 {
                continue;
            }
            let memory = source == 6 || destination == 6;
            let mut system = machine(opcode, destination == 6);
            let before = system.cpu_snapshot();
            let final_state = expected(before, destination, source);
            let start = system.elapsed_ticks();
            let fetch = system.step_machine_cycle().expect("opcode fetch");
            assert!(matches!(fetch.action, CycleAction::Read { addr } if addr == before.pc));
            assert_eq!(fetch.read_value, Some(opcode));
            assert_eq!((fetch.tick_before, fetch.tick_after), (start, start + 4));
            assert_eq!(fetch.complete, !memory);
            if memory {
                let mut pending = before;
                pending.pc += 1;
                assert_eq!(system.cpu_snapshot(), pending);
                let access = system.step_machine_cycle().expect("data access");
                assert_eq!(
                    (access.tick_before, access.tick_after),
                    (start + 4, start + 8)
                );
                assert!(access.complete);
                if source == 6 {
                    assert!(matches!(access.action, CycleAction::Read { addr: 0xC123 }));
                    assert_eq!(access.read_value, Some(0x9B));
                } else {
                    assert!(
                        matches!(access.action, CycleAction::Write { addr: 0xC123, value } if value == VALUES[source])
                    );
                    assert_eq!(access.read_value, None);
                }
            }
            assert_eq!(system.cpu_snapshot(), final_state, "opcode {opcode:02X}");
            if destination == 6 {
                system.step_instruction(2).expect("read back stored byte");
                assert_eq!((system.cpu_snapshot().af >> 8) as u8, VALUES[source]);
            }
        }
    }
}

#[test]
fn all_transfer_budgets_stop_at_completion_and_memory_transfers_resume() {
    for (destination, row) in OPCODES.iter().enumerate() {
        for (source, &opcode) in row.iter().enumerate() {
            if opcode == 0x76 {
                continue;
            }
            let cycles = if source == 6 || destination == 6 {
                2u64
            } else {
                1
            };
            for budget in [cycles, 100] {
                let mut system = machine(opcode, false);
                let before = system.cpu_snapshot();
                let ticks = system.elapsed_ticks();
                assert!(matches!(
                    system.step_instruction(0),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    })
                ));
                assert_eq!(system.cpu_snapshot(), before);
                assert_eq!(system.elapsed_ticks(), ticks);
                assert!(
                    matches!(system.step_instruction(budget), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
                );
                assert_eq!(system.cpu_snapshot(), expected(before, destination, source));
                assert_eq!(system.elapsed_ticks(), ticks + cycles * 4);
            }
            if cycles == 2 {
                let mut system = machine(opcode, false);
                let before = system.cpu_snapshot();
                assert!(matches!(
                    system.step_instruction(1),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 1,
                        ticks: 4
                    })
                ));
                let mut pending = before;
                pending.pc += 1;
                assert_eq!(system.cpu_snapshot(), pending);
                let ticks = system.elapsed_ticks();
                assert!(matches!(
                    system.step_instruction(0),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    })
                ));
                assert_eq!(system.elapsed_ticks(), ticks);
                assert_eq!(system.cpu_snapshot(), pending);
                assert!(matches!(
                    system.step_instruction(100),
                    Ok(InstructionStep::Complete {
                        mcycles: 1,
                        ticks: 4
                    })
                ));
                assert_eq!(system.cpu_snapshot(), expected(before, destination, source));
            }
        }
    }
}

#[test]
fn mixed_transfers_can_change_hl_after_using_it_as_a_pointer() {
    let mut system = program(&[
        0x01, 0x34, 0x12, // BC=1234
        0x11, 0x78, 0x56, // DE=5678
        0x21, 0x23, 0xC1, // HL=C123
        0x3E, 0x9B, // A=9B
        0x42, // B=D
        0x59, // E=C
        0x73, // (HL)=E
        0x66, // H=(old HL), so HL=3423
        0x68, // L=B, so HL=3456
        0x7C, // A=H
        0x00, 0xD3,
    ]);
    for cycles in [3, 3, 3, 2, 1, 1, 2, 2, 1, 1, 1] {
        assert!(
            matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
        );
    }
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: 0x34B0,
            bc: 0x5634,
            de: 0x5634,
            hl: 0x3456,
            pc: 0x0112,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 80);
}

#[test]
fn halt_remains_an_unsupported_sticky_fault_without_a_memory_transfer() {
    let mut system = machine(0x76, false);
    let before = system.cpu_snapshot();
    let start = system.elapsed_ticks();
    let error = system.step_machine_cycle().expect_err("HALT is deferred");
    match error {
        SystemError::CpuCompletionError(CpuError::UnsupportedOpcode(_), trace) => {
            assert!(matches!(trace.action, CycleAction::Read { addr } if addr == before.pc));
            assert_eq!(trace.read_value, Some(0x76));
            assert_eq!((trace.tick_before, trace.tick_after), (start, start + 4));
            assert!(!trace.complete);
        }
        _ => panic!("expected unsupported opcode completion"),
    }
    let mut expected = before;
    expected.pc += 1;
    assert_eq!(system.cpu_snapshot(), expected);
    assert!(matches!(
        system.step_machine_cycle(),
        Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
    ));
    assert_eq!(system.elapsed_ticks(), start + 4);
    assert_eq!(system.cpu_snapshot(), expected);
}
