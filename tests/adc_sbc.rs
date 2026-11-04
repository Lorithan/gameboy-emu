use gameboy_emu::{
    cpu::{CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Register(u8),
    Memory,
    Immediate,
}

const CASES: [(u8, u8, Source); 9] = [
    (0x88, 0x98, Source::Register(0x06)),
    (0x89, 0x99, Source::Register(0x0E)),
    (0x8A, 0x9A, Source::Register(0x16)),
    (0x8B, 0x9B, Source::Register(0x1E)),
    (0x8C, 0x9C, Source::Register(0x26)),
    (0x8D, 0x9D, Source::Register(0x2E)),
    (0x8E, 0x9E, Source::Memory),
    (0x8F, 0x9F, Source::Register(0x3E)),
    (0xCE, 0xDE, Source::Immediate),
];

fn program(bytes: &[u8], carry: bool) -> System {
    let mut rom = vec![0; 32_768];
    // Two independently known synthetic header profiles: title E7 produces
    // checksum 00/C=0; empty title produces checksum E7/C=1.
    rom[0x0134] = if carry { 0 } else { 0xE7 };
    rom[0x014D] = if carry { 0xE7 } else { 0 };
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"))
}

fn machine(opcode: u8, source: Source, left: u8, right: u8, carry: bool) -> System {
    let (mut bytes, setup): (Vec<u8>, Vec<u64>) = match source {
        Source::Register(load) => (vec![load, right, 0x3E, left], vec![2, 2]),
        Source::Memory => (
            vec![0x21, 0x23, 0xC1, 0x36, right, 0x3E, left],
            vec![3, 3, 2],
        ),
        Source::Immediate => (vec![0x3E, left], vec![2]),
    };
    bytes.push(opcode);
    if source == Source::Immediate {
        bytes.push(right);
    }
    bytes.push(0xD3);
    let mut system = program(&bytes, carry);
    for cycles in setup {
        system.step_instruction(cycles).expect("setup");
    }
    system
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

#[test]
fn every_carry_source_has_correct_traces_budgets_and_boundary_results() {
    for (adc, sbc, source) in CASES {
        for (opcode, add) in [(adc, true), (sbc, false)] {
            for (left, right) in [(0u8, 0), (0x0F, 0), (0xFF, 0), (0xFF, 0xFF), (0x10, 0x0F)] {
                if source == Source::Register(0x3E) && left != right {
                    continue;
                }
                for carry in [false, true] {
                    let mut system = machine(opcode, source, left, right, carry);
                    let before = system.cpu_snapshot();
                    let ticks = system.elapsed_ticks();
                    let memory_operand = !matches!(source, Source::Register(_));
                    let cycles = if memory_operand { 2 } else { 1 };
                    let mut expected = reference(before, left, right, carry, add);
                    expected.pc += if source == Source::Immediate { 2 } else { 1 };
                    let fetch = system.step_machine_cycle().expect("opcode");
                    assert!(
                        matches!(fetch.action, CycleAction::Read { addr } if addr == before.pc)
                    );
                    assert_eq!(fetch.read_value, Some(opcode));
                    assert_eq!((fetch.tick_before, fetch.tick_after), (ticks, ticks + 4));
                    assert_eq!(fetch.complete, !memory_operand);
                    if memory_operand {
                        let mut pending = before;
                        pending.pc += 1;
                        assert_eq!(system.cpu_snapshot(), pending);
                        let read = system.step_machine_cycle().expect("operand");
                        let address = if source == Source::Immediate {
                            pending.pc
                        } else {
                            0xC123
                        };
                        assert!(
                            matches!(read.action, CycleAction::Read { addr } if addr == address)
                        );
                        assert_eq!(read.read_value, Some(right));
                        assert_eq!((read.tick_before, read.tick_after), (ticks + 4, ticks + 8));
                        assert!(read.complete);
                    }
                    assert_eq!(system.cpu_snapshot(), expected);
                    for budget in [cycles, 100] {
                        let mut system = machine(opcode, source, left, right, carry);
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
                        assert_eq!(system.cpu_snapshot(), expected);
                    }
                    if memory_operand {
                        let mut system = machine(opcode, source, left, right, carry);
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
                        assert!(matches!(
                            system.step_instruction(0),
                            Ok(InstructionStep::BudgetExhausted {
                                mcycles: 0,
                                ticks: 0
                            })
                        ));
                        assert_eq!(system.cpu_snapshot(), pending);
                        assert!(matches!(
                            system.step_instruction(100),
                            Ok(InstructionStep::Complete {
                                mcycles: 1,
                                ticks: 4
                            })
                        ));
                        assert_eq!(system.cpu_snapshot(), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn carry_and_borrow_chain_through_flag_preserving_transfers() {
    let mut system = program(
        &[
            0x21, 0x23, 0xC1, // HL=C123
            0x3E, 0xFF, // low addition byte
            0xC6, 0x01, // FF+01 -> 00, C=1
            0x47, // save low result in B
            0x3E, 0x12, // high addition byte, preserve C
            0xCE, 0x34, // 12+34+1 -> 47
            0x77, // save high result in RAM
            0x3E, 0x00, // low subtraction byte
            0xD6, 0x01, // 00-01 -> FF, C=1
            0x4F, // save low result in C
            0x3E, 0x12, // high subtraction byte, preserve C
            0xDE, 0x01, // 12-01-1 -> 10
            0x7E, // reload 47, preserving F=40
            0x00, 0xD3,
        ],
        true,
    );
    for (cycles, flags) in [
        (3, 0xB0),
        (2, 0xB0),
        (2, 0xB0),
        (1, 0xB0),
        (2, 0xB0),
        (2, 0x00),
        (2, 0x00),
        (2, 0x00),
        (2, 0x70),
        (1, 0x70),
        (2, 0x70),
        (2, 0x40),
        (2, 0x40),
        (1, 0x40),
    ] {
        assert!(
            matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
        );
        assert_eq!(system.cpu_snapshot().af & 0xFF, flags);
    }
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: 0x4740,
            bc: 0x00FF,
            de: 0x00D8,
            hl: 0xC123,
            pc: 0x0118,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 104);
}
