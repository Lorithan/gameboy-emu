use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn complete(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

fn prepared(byte: u8, equal: bool, sp: u16) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x010D].copy_from_slice(&[
        0x31,
        sp as u8,
        (sp / 256) as u8,
        0x01,
        0,
        2,
        0xC5,
        0x3E,
        u8::from(equal),
        0xFE,
        1,
        byte,
        0xD3,
    ]);
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    for cycles in [3, 3, 4, 2, 2] {
        complete(system.step_instruction(100).unwrap(), cycles);
    }
    system
}

#[test]
fn conditional_return_traces_and_all_budgets_cover_both_outcomes() {
    for byte in [0xC0, 0xC8, 0xD0, 0xD8] {
        for equal in [false, true] {
            let taken = match byte {
                0xC0 | 0xD8 => !equal,
                _ => equal,
            };
            let cycles = if taken { 5u64 } else { 2 };
            for sp in [0xFFFEu16, 0xC100] {
                let mut system = prepared(byte, equal, sp);
                let mut expected = system.cpu_snapshot();
                for cycle in 0..cycles {
                    let trace = system.step_machine_cycle().unwrap();
                    assert_eq!(
                        (trace.tick_before, trace.tick_after),
                        (56 + cycle * 4, 60 + cycle * 4)
                    );
                    assert_eq!(trace.complete, cycle + 1 == cycles);
                    match cycle {
                        0 => {
                            assert!(matches!(trace.action, CycleAction::Read { addr: 0x010B }));
                            assert_eq!(trace.read_value, Some(byte));
                            expected.pc = 0x010C;
                        }
                        1 | 4 => {
                            assert!(matches!(trace.action, CycleAction::Idle));
                            assert_eq!(trace.read_value, None);
                            if cycle == 4 {
                                expected.pc = 0x0200;
                            }
                        }
                        2 => {
                            assert!(
                                matches!(trace.action, CycleAction::Read { addr } if addr == sp - 2)
                            );
                            assert_eq!(trace.read_value, Some(0));
                            expected.sp = sp - 1;
                        }
                        _ => {
                            assert!(
                                matches!(trace.action, CycleAction::Read { addr } if addr == sp - 1)
                            );
                            assert_eq!(trace.read_value, Some(2));
                            expected.sp = sp;
                        }
                    }
                    assert_eq!(system.cpu_snapshot(), expected);
                }
                for budget in [0u64, 1, 2, 3, 4, 5, 100] {
                    let mut system = prepared(byte, equal, sp);
                    let mut expected = system.cpu_snapshot();
                    let outcome = system.step_instruction(budget).unwrap();
                    if budget < cycles {
                        assert!(
                            matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                        );
                        if budget > 0 {
                            expected.pc = 0x010C;
                        }
                        expected.sp += budget.saturating_sub(2) as u16;
                        assert_eq!(system.cpu_snapshot(), expected);
                        assert!(matches!(
                            system.step_instruction(0).unwrap(),
                            InstructionStep::BudgetExhausted {
                                mcycles: 0,
                                ticks: 0
                            }
                        ));
                        assert_eq!(system.cpu_snapshot(), expected);
                        complete(system.step_instruction(100).unwrap(), cycles - budget);
                    } else {
                        complete(outcome, cycles);
                    }
                    expected.pc = if taken { 0x0200 } else { 0x010C };
                    expected.sp = if taken { sp } else { sp - 2 };
                    assert_eq!(system.cpu_snapshot(), expected);
                    assert_eq!(system.elapsed_ticks(), 56 + cycles * 4);
                }
            }
        }
    }
}

#[test]
fn nested_subroutine_continues_on_false_return_and_unwinds_on_true_return() {
    for sp in [0xFFFEu16, 0xC100] {
        for (false_return, true_return) in [(0xC0, 0xC8), (0xD0, 0xD8)] {
            let mut rom = vec![0; 32768];
            rom[0x014D] = 0xE7;
            rom[0x0100..0x0108].copy_from_slice(&[
                0x31,
                sp as u8,
                (sp / 256) as u8,
                0xCD,
                0,
                2,
                0,
                0xD3,
            ]);
            rom[0x0200..0x0206].copy_from_slice(&[0xCD, 0, 3, 0, 0xC9, 0xD3]);
            rom[0x0300..0x0303].copy_from_slice(&[false_return, true_return, 0xD3]);
            let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
            let mut expected = system.cpu_snapshot();
            for (cycles, pc, stack) in [
                (3, 0x0103, sp),
                (6, 0x0200, sp - 2),
                (6, 0x0300, sp - 4),
                (2, 0x0301, sp - 4),
                (5, 0x0203, sp - 2),
                (1, 0x0204, sp - 2),
                (4, 0x0106, sp),
                (1, 0x0107, sp),
            ] {
                complete(system.step_instruction(100).unwrap(), cycles);
                expected.pc = pc;
                expected.sp = stack;
                assert_eq!(system.cpu_snapshot(), expected);
            }
            assert_eq!(system.elapsed_ticks(), 112);
        }
    }
}
