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
fn complement_carry_have_one_fetch_cycle_and_respect_zero_and_excess_budgets() {
    for (opcode, af) in [(0x2F, 0xFEB0u16 | 0x60), (0x37, 0x0190), (0x3F, 0x0180)] {
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
fn complement_carry_flags_feed_arithmetic_and_conditional_branches() {
    let mut system = program(&[
        0xAF, // XOR A -> Z=1, C=0
        0x2F, // CPL -> A=FF, Z preserved, N/H set
        0x28, 1, 0xD3, // JR Z taken
        0x37, // SCF -> Z preserved, C=1
        0xCE, 0,    // ADC 0 -> A=0, F=B0
        0x3F, // CCF -> F=80
        0x30, 1, 0xD3, // JR NC taken
        0x37, // SCF -> F=90
        0xDE, 0,    // SBC 0 -> A=FF, F=70
        0x3F, // CCF -> F=00
        0x20, 1, 0xD3, // JR NZ taken
        0x00, 0xD3,
    ]);
    for (cycles, af) in [
        (1, 0x0080),
        (1, 0xFFE0),
        (3, 0xFFE0),
        (1, 0xFF90),
        (2, 0x00B0),
        (1, 0x0080),
        (3, 0x0080),
        (1, 0x0090),
        (2, 0xFF70),
        (1, 0xFF00),
        (3, 0xFF00),
        (1, 0xFF00),
    ] {
        assert!(
            matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == 4 * cycles)
        );
        assert_eq!(system.cpu_snapshot().af, af);
    }
    assert_eq!(system.cpu_snapshot().pc, 0x0114);
    assert_eq!(system.elapsed_ticks(), 80);
}
