use gameboy_emu::{
    cpu::{CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"))
}

fn machine(opcode: u8, left: u8, right: u8, immediate: bool) -> System {
    let mut bytes = vec![0x21, 0x23, 0xC1, 0x36, right, 0x3E, left, opcode];
    if immediate {
        bytes.push(right);
    }
    bytes.push(0xD3);
    let mut system = program(&bytes);
    for cycles in [3, 3, 2] {
        system.step_instruction(cycles).expect("operand setup");
    }
    system
}

#[test]
fn operand_reads_have_two_exact_cycles_and_resumable_instruction_budgets() {
    let cases = [
        (0x86, true, false),
        (0x96, false, false),
        (0xC6, true, true),
        (0xD6, false, true),
    ];
    let examples: [(u8, u8, u8, u8, u8, u8); 5] = [
        (0xFF, 0x01, 0x00, 0xB0, 0xFE, 0x40),
        (0x0F, 0x01, 0x10, 0x20, 0x0E, 0x40),
        (0x10, 0x01, 0x11, 0x00, 0x0F, 0x60),
        (0x00, 0x01, 0x01, 0x00, 0xFF, 0x70),
        (0x42, 0x42, 0x84, 0x00, 0x00, 0xC0),
    ];
    for (opcode, add, immediate) in cases {
        for (left, right, add_result, add_flags, sub_result, sub_flags) in examples {
            let mut system = machine(opcode, left, right, immediate);
            let before = system.cpu_snapshot();
            let mut pending = before;
            pending.pc += 1;
            let mut expected = pending;
            expected.af = if add {
                u16::from(add_result) << 8 | u16::from(add_flags)
            } else {
                u16::from(sub_result) << 8 | u16::from(sub_flags)
            };
            if immediate {
                expected.pc += 1;
            }
            let fetch = system.step_machine_cycle().expect("opcode fetch");
            assert!(matches!(fetch.action, CycleAction::Read { addr: 0x0107 }));
            assert_eq!(fetch.read_value, Some(opcode));
            assert_eq!((fetch.tick_before, fetch.tick_after), (32, 36));
            assert!(!fetch.complete);
            assert_eq!(system.cpu_snapshot(), pending);
            let operand = system.step_machine_cycle().expect("operand read");
            let address = if immediate { 0x0108 } else { 0xC123 };
            assert!(matches!(operand.action, CycleAction::Read { addr } if addr == address));
            assert_eq!(operand.read_value, Some(right));
            assert_eq!((operand.tick_before, operand.tick_after), (36, 40));
            assert!(operand.complete);
            assert_eq!(system.cpu_snapshot(), expected);
            for budget in [2, 100] {
                let mut system = machine(opcode, left, right, immediate);
                assert!(matches!(
                    system.step_instruction(0),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    })
                ));
                assert_eq!(system.cpu_snapshot(), before);
                assert_eq!(system.elapsed_ticks(), 32);
                assert!(matches!(
                    system.step_instruction(budget),
                    Ok(InstructionStep::Complete {
                        mcycles: 2,
                        ticks: 8
                    })
                ));
                assert_eq!(system.cpu_snapshot(), expected);
            }
            let mut system = machine(opcode, left, right, immediate);
            assert!(matches!(
                system.step_instruction(1),
                Ok(InstructionStep::BudgetExhausted {
                    mcycles: 1,
                    ticks: 4
                })
            ));
            assert_eq!(system.cpu_snapshot(), pending);
            assert!(matches!(
                system.step_instruction(0),
                Ok(InstructionStep::BudgetExhausted {
                    mcycles: 0,
                    ticks: 0
                })
            ));
            assert_eq!(system.cpu_snapshot(), pending);
            assert_eq!(system.elapsed_ticks(), 36);
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

#[test]
fn all_operand_forms_interleave_without_modifying_source_memory() {
    let mut system = program(&[
        0x21, 0x23, 0xC1, // HL=C123
        0x36, 0x01, // (HL)=01
        0x3E, 0xFF, // A=FF
        0x86, // ADD A,(HL) -> 00, F=B0
        0xD6, 0x01, // SUB A,01 -> FF, F=70
        0xC6, 0x01, // ADD A,01 -> 00, F=B0
        0x96, // SUB A,(HL) -> FF, F=70
        0x47, // B=A
        0x90, // SUB A,B -> 00, F=C0
        0x00, 0xD3,
    ]);
    for (cycles, flags) in [
        (3, 0xB0),
        (3, 0xB0),
        (2, 0xB0),
        (2, 0xB0),
        (2, 0x70),
        (2, 0xB0),
        (2, 0x70),
        (1, 0x70),
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
            af: 0x00C0,
            bc: 0xFF13,
            de: 0x00D8,
            hl: 0xC123,
            pc: 0x0110,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 76);
}
