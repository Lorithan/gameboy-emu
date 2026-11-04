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

const CASES: [(u8, Source); 9] = [
    (0xB8, Source::Register(0x06)),
    (0xB9, Source::Register(0x0E)),
    (0xBA, Source::Register(0x16)),
    (0xBB, Source::Register(0x1E)),
    (0xBC, Source::Register(0x26)),
    (0xBD, Source::Register(0x2E)),
    (0xBE, Source::Memory),
    (0xBF, Source::Register(0x3E)),
    (0xFE, Source::Immediate),
];

fn program(bytes: &[u8], carry: bool) -> System {
    let mut rom = vec![0; 32_768];
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

#[test]
fn public_comparisons_preserve_a_with_exact_traces_and_instruction_budgets() {
    for (opcode, source) in CASES {
        for (left, right, flags) in [
            (0x10u8, 0x01u8, 0x60u16),
            (0, 1, 0x70),
            (0x42, 0x42, 0xC0),
            (0xFF, 0, 0x40),
        ] {
            if source == Source::Register(0x3E) && left != right {
                continue;
            }
            for carry in [false, true] {
                let mut system = machine(opcode, source, left, right, carry);
                let before = system.cpu_snapshot();
                let ticks = system.elapsed_ticks();
                let needs_operand = !matches!(source, Source::Register(_));
                let cycles = if needs_operand { 2u64 } else { 1 };
                let mut expected = before;
                expected.af = (expected.af & 0xFF00) | flags;
                expected.pc += if source == Source::Immediate { 2 } else { 1 };
                let fetch = system.step_machine_cycle().expect("compare opcode");
                assert!(matches!(fetch.action, CycleAction::Read { addr } if addr == before.pc));
                assert_eq!(fetch.read_value, Some(opcode));
                assert_eq!((fetch.tick_before, fetch.tick_after), (ticks, ticks + 4));
                assert_eq!(fetch.complete, !needs_operand);
                if needs_operand {
                    let mut pending = before;
                    pending.pc += 1;
                    assert_eq!(system.cpu_snapshot(), pending);
                    let read = system.step_machine_cycle().expect("compare operand");
                    let address = if source == Source::Immediate {
                        pending.pc
                    } else {
                        0xC123
                    };
                    assert!(matches!(read.action, CycleAction::Read { addr } if addr == address));
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
                if needs_operand {
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

#[test]
fn comparison_flags_feed_arithmetic_and_survive_register_memory_transfers() {
    let mut system = program(
        &[
            0x21, 0x23, 0xC1, // HL=C123
            0x36, 0x10, // (HL)=10
            0x3E, 0x10, // A=10
            0x06, 0x11, // B=11
            0xB8, // CP B -> F=70, A remains 10
            0x4F, // C=A -> 10, preserve F
            0xCE, 0x00, // ADC A,00 -> 11, F=00
            0xBE, // CP (HL) -> F=40, A remains 11
            0x77, // (HL)=A, preserve F
            0xFE, 0x11, // CP 11 -> F=C0, A remains 11
            0x57, // D=A -> 11, preserve F
            0xBF, // CP A -> F=C0
            0x00, 0xD3,
        ],
        true,
    );
    for (cycles, flags) in [
        (3, 0xB0),
        (3, 0xB0),
        (2, 0xB0),
        (2, 0xB0),
        (1, 0x70),
        (1, 0x70),
        (2, 0x00),
        (2, 0x40),
        (2, 0x40),
        (2, 0xC0),
        (1, 0xC0),
        (1, 0xC0),
        (1, 0xC0),
    ] {
        assert!(
            matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
        );
        assert_eq!(system.cpu_snapshot().af & 0xFF, flags);
    }
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: 0x11C0,
            bc: 0x1110,
            de: 0x11D8,
            hl: 0xC123,
            pc: 0x0114,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 92);
}
