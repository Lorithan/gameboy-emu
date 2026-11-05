use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(opcode: u8, vector: u16, sp: u16) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    // Put RST at 0110 so its return address has bits outside the low nibble.
    rom[0x0100..0x0106].copy_from_slice(&[0x31, sp as u8, (sp / 256) as u8, 0xC3, 0x10, 1]);
    rom[0x0110] = opcode;
    rom[0x0111] = 0xD3;
    rom[usize::from(vector)] = 0xC9;
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    complete(system.step_instruction(100).unwrap(), 3);
    complete(system.step_instruction(100).unwrap(), 4);
    system
}

fn complete(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

#[test]
fn restart_resumes_all_budgets_without_operand_reads_or_duplicate_writes() {
    for (byte, vector) in [
        (0xC7, 0u16),
        (0xCF, 8),
        (0xD7, 0x10),
        (0xDF, 0x18),
        (0xE7, 0x20),
        (0xEF, 0x28),
        (0xF7, 0x30),
        (0xFF, 0x38),
    ] {
        for budget in [0u64, 1, 2, 3, 4, 100] {
            let mut system = machine(byte, vector, 0xFFFE);
            let mut expected = system.cpu_snapshot();
            let outcome = system.step_instruction(budget).unwrap();
            if budget < 4 {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                );
                if budget > 0 {
                    expected.pc = 0x0111;
                }
                if budget == 3 {
                    expected.sp = 0xFFFD;
                }
                assert_eq!(system.cpu_snapshot(), expected);
                assert!(matches!(
                    system.step_instruction(0).unwrap(),
                    InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    }
                ));
                assert_eq!(system.cpu_snapshot(), expected);
                complete(system.step_instruction(100).unwrap(), 4 - budget);
            } else {
                complete(outcome, 4);
            }
            expected.pc = vector;
            expected.sp = 0xFFFC;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 44);
            complete(system.step_instruction(100).unwrap(), 4);
            expected.pc = 0x0111;
            expected.sp = 0xFFFE;
            assert_eq!(system.cpu_snapshot(), expected);
        }
    }
}

#[test]
fn each_restart_has_four_cycles_and_returns_to_the_next_byte() {
    for (opcode, vector) in [
        (0xC7, 0u16),
        (0xCF, 8),
        (0xD7, 0x10),
        (0xDF, 0x18),
        (0xE7, 0x20),
        (0xEF, 0x28),
        (0xF7, 0x30),
        (0xFF, 0x38),
    ] {
        for sp in [0xFFFEu16, 0xC100] {
            let mut system = machine(opcode, vector, sp);
            let mut expected = system.cpu_snapshot();
            for cycle in 0u64..4 {
                let trace = system.step_machine_cycle().unwrap();
                assert_eq!(
                    (trace.tick_before, trace.tick_after),
                    (28 + cycle * 4, 32 + cycle * 4)
                );
                assert_eq!(trace.complete, cycle == 3);
                match cycle {
                    0 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0110 }));
                        assert_eq!(trace.read_value, Some(opcode));
                        expected.pc = 0x0111;
                    }
                    1 => assert!(matches!(trace.action, CycleAction::Idle)),
                    2 => {
                        assert!(
                            matches!(trace.action, CycleAction::Write { addr, value: 1 } if addr == sp - 1)
                        );
                        expected.sp = sp - 1;
                    }
                    _ => {
                        assert!(
                            matches!(trace.action, CycleAction::Write { addr, value: 0x11 } if addr == sp - 2)
                        );
                        expected.sp = sp - 2;
                        expected.pc = vector;
                    }
                }
                if cycle > 0 {
                    assert_eq!(trace.read_value, None);
                }
                assert_eq!(system.cpu_snapshot(), expected);
            }
            complete(system.step_instruction(100).unwrap(), 4);
            expected.sp = sp;
            expected.pc = 0x0111;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 60);
        }
    }
}
