use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

#[test]
fn rotates_finish_on_one_fetch_with_all_instruction_budgets() {
    for (opcode, af) in [
        (0x07, 0x0200),
        (0x0F, 0x8010),
        (0x17, 0x0300),
        (0x1F, 0x8010),
    ] {
        for budget in [0, 1, 100] {
            let mut system = program(&[opcode, 0xD3]);
            let before = system.cpu_snapshot();
            if budget == 0 {
                assert!(matches!(
                    system.step_instruction(0).unwrap(),
                    InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    }
                ));
                assert_eq!(system.cpu_snapshot(), before);
                assert_eq!(system.elapsed_ticks(), 0);
                let trace = system.step_machine_cycle().unwrap();
                assert!(matches!(trace.action, CycleAction::Read { addr: 0x0100 }));
                assert_eq!(trace.read_value, Some(opcode));
                assert_eq!(
                    (trace.tick_before, trace.tick_after, trace.complete),
                    (0, 4, true)
                );
            } else {
                assert!(matches!(
                    system.step_instruction(budget).unwrap(),
                    InstructionStep::Complete {
                        mcycles: 1,
                        ticks: 4
                    }
                ));
            }
            let mut expected = before;
            expected.af = af;
            expected.pc += 1;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 4);
        }
    }
}

#[test]
fn rotate_carry_and_cleared_zero_feed_arithmetic_and_branches() {
    let mut system = program(&[
        0x3E, 0x80, // A=80
        0x07, // RLCA -> A=01 C=1
        0x17, // RLA -> A=03 C=0
        0x0F, // RRCA -> A=81 C=1
        0x1F, // RRA -> A=C0 C=1
        0xCE, 0,    // ADC -> A=C1 C=0
        0xAF, // XOR A -> A=00 Z=1
        0x07, // RLCA -> A=00 Z=0
        0x28, 1, // JR Z untaken
        0x20, 1, 0xD3, // JR NZ taken
        0x37, // SCF -> C=1
        0x1F, // RRA -> A=80 C=0
        0x17, // RLA -> A=00 C=1 Z=0
        0xDE, 0, // SBC -> A=FF F=70
        0x38, 1, 0xD3, // JR C taken
        0x00, 0xD3,
    ]);
    let mut cycles = 0;
    for (count, af) in [
        (2, 0x80B0),
        (1, 0x0110),
        (1, 0x0300),
        (1, 0x8110),
        (1, 0xC010),
        (2, 0xC100),
        (1, 0x0080),
        (1, 0x0000),
        (2, 0x0000),
        (3, 0x0000),
        (1, 0x0010),
        (1, 0x8000),
        (1, 0x0010),
        (2, 0xFF70),
        (3, 0xFF70),
        (1, 0xFF70),
    ] {
        assert!(
            matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == count && ticks == 4 * count)
        );
        cycles += count;
        assert_eq!(system.cpu_snapshot().af, af);
    }
    assert_eq!(system.cpu_snapshot().pc, 0x0118);
    assert_eq!(system.elapsed_ticks(), cycles * 4);
}
