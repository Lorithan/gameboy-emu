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

fn complete(system: &mut System, cycles: u64) {
    assert!(
        matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

#[test]
fn decimal_adjust_completes_on_one_fetch_and_respects_all_budgets() {
    for (a, operation, operand, af) in [
        (0x09, 0xC6, 1, 0x1000),
        (0x99, 0xC6, 1, 0x0090),
        (0x10, 0xD6, 1, 0x0940),
        (0x00, 0xD6, 1, 0x9950),
    ] {
        for budget in [0, 1, 100] {
            let mut system = program(&[0x3E, a, operation, operand, 0x27, 0xD3]);
            complete(&mut system, 2);
            complete(&mut system, 2);
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
                assert_eq!(system.elapsed_ticks(), 16);
                let trace = system.step_machine_cycle().unwrap();
                assert!(matches!(trace.action, CycleAction::Read { addr: 0x0104 }));
                assert_eq!(trace.read_value, Some(0x27));
                assert_eq!(
                    (trace.tick_before, trace.tick_after, trace.complete),
                    (16, 20, true)
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
            assert_eq!(system.elapsed_ticks(), 20);
        }
    }
}

#[test]
fn decimal_flags_feed_following_arithmetic_and_branches() {
    let bytes = [
        0x3E, 0x99, // A=99
        0xC6, 1,    // ADD 1 -> 9A
        0x27, // DAA -> 00 with C/Z
        0x28, 1, 0xD3, // JR Z taken
        0xCE, 0,    // ADC 0 consumes DAA carry -> 01
        0x27, // DAA -> 01
        0xD6, 2,    // SUB 2 -> FF with borrow
        0x27, // DAA -> 99, N/C preserved
        0x38, 1, 0xD3, // JR C taken
        0x00, 0xD3,
    ];
    let mut system = program(&bytes);
    for (cycles, af) in [
        (2, 0x99B0),
        (2, 0x9A00),
        (1, 0x0090),
        (3, 0x0090),
        (2, 0x0100),
        (1, 0x0100),
        (2, 0xFF70),
        (1, 0x9950),
        (3, 0x9950),
        (1, 0x9950),
    ] {
        complete(&mut system, cycles);
        assert_eq!(system.cpu_snapshot().af, af);
    }
    assert_eq!(system.cpu_snapshot().pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 72);
}
