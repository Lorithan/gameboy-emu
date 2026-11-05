use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(byte: u8, equal: bool) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0108].copy_from_slice(&[0x3E, u8::from(equal), 0xFE, 1, byte, 4, 1, 0xD3]);
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    for _ in 0..2 {
        complete(system.step_instruction(100).unwrap(), 2);
    }
    system
}

fn complete(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

fn taken(byte: u8, equal: bool) -> bool {
    match byte {
        0xCD => true,
        0xC4 | 0xDC => !equal,
        _ => equal,
    }
}

#[test]
fn nested_calls_return_in_lifo_order_and_run_each_caller_continuation() {
    for sp in [0xFFFEu16, 0xC100] {
        let mut rom = vec![0xD3; 32768];
        // Keep the synthetic header valid while leaving code sentinels elsewhere.
        rom[0x0134..=0x014D].fill(0);
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
        rom[0x0300..0x0303].copy_from_slice(&[0, 0xC9, 0xD3]);
        let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
        let mut expected = system.cpu_snapshot();
        for (cycles, pc, stack) in [
            (3, 0x0103, sp),
            (6, 0x0200, sp - 2),
            (6, 0x0300, sp - 4),
            (1, 0x0301, sp - 4),
            (4, 0x0203, sp - 2),
            (1, 0x0204, sp - 2),
            (4, 0x0106, sp),
            (1, 0x0107, sp),
        ] {
            complete(system.step_instruction(100).unwrap(), cycles);
            expected.pc = pc;
            expected.sp = stack;
            assert_eq!(system.cpu_snapshot(), expected);
        }
        assert_eq!(system.elapsed_ticks(), 104);
    }
}

fn return_machine(sp: u16) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0107].copy_from_slice(&[0x31, sp as u8, (sp / 256) as u8, 0xCD, 0, 2, 0xD3]);
    rom[0x0200] = 0xC9;
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    complete(system.step_instruction(100).unwrap(), 3);
    complete(system.step_instruction(100).unwrap(), 6);
    system
}

#[test]
fn call_ret_round_trip_has_stack_reads_then_idle_and_restores_sp() {
    for sp in [0xFFFEu16, 0xC100] {
        let mut system = return_machine(sp);
        let mut expected = system.cpu_snapshot();
        for cycle in 0u64..4 {
            let trace = system.step_machine_cycle().unwrap();
            assert_eq!(
                (trace.tick_before, trace.tick_after),
                (36 + cycle * 4, 40 + cycle * 4)
            );
            assert_eq!(trace.complete, cycle == 3);
            match cycle {
                0 => {
                    assert!(matches!(trace.action, CycleAction::Read { addr: 0x0200 }));
                    assert_eq!(trace.read_value, Some(0xC9));
                    expected.pc = 0x0201;
                }
                1 => {
                    assert!(matches!(trace.action, CycleAction::Read { addr } if addr == sp - 2));
                    assert_eq!(trace.read_value, Some(6));
                    expected.sp = sp - 1;
                }
                2 => {
                    assert!(matches!(trace.action, CycleAction::Read { addr } if addr == sp - 1));
                    assert_eq!(trace.read_value, Some(1));
                    expected.sp = sp;
                }
                _ => {
                    assert!(matches!(trace.action, CycleAction::Idle));
                    assert_eq!(trace.read_value, None);
                    expected.pc = 0x0106;
                }
            }
            assert_eq!(system.cpu_snapshot(), expected);
        }
        assert_eq!(system.elapsed_ticks(), 52);
    }
}

#[test]
fn ret_resumes_every_budget_and_stops_before_the_caller_sentinel() {
    for sp in [0xFFFEu16, 0xC100] {
        for budget in [0u64, 1, 2, 3, 4, 100] {
            let mut system = return_machine(sp);
            let mut expected = system.cpu_snapshot();
            let outcome = system.step_instruction(budget).unwrap();
            if budget < 4 {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                );
                if budget > 0 {
                    expected.pc = 0x0201;
                }
                expected.sp += budget.saturating_sub(1) as u16;
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
            expected.pc = 0x0106;
            expected.sp = sp;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 52);
        }
    }
}

#[test]
fn call_traces_place_idle_before_return_writes_for_all_conditions() {
    for byte in [0xCD, 0xC4, 0xCC, 0xD4, 0xDC] {
        for equal in [false, true] {
            let mut system = machine(byte, equal);
            let mut expected = system.cpu_snapshot();
            let cycles = if taken(byte, equal) { 6u64 } else { 3 };
            for cycle in 0..cycles {
                let trace = system.step_machine_cycle().unwrap();
                assert_eq!(
                    (trace.tick_before, trace.tick_after),
                    (16 + cycle * 4, 20 + cycle * 4)
                );
                assert_eq!(trace.complete, cycle + 1 == cycles);
                if cycle < 3 {
                    assert!(
                        matches!(trace.action, CycleAction::Read { addr } if addr == 0x0104 + cycle as u16)
                    );
                    assert_eq!(trace.read_value, Some([byte, 4, 1][cycle as usize]));
                    expected.pc = 0x0105 + cycle as u16;
                } else {
                    assert_eq!(trace.read_value, None);
                    match cycle {
                        3 => assert!(matches!(trace.action, CycleAction::Idle)),
                        4 => {
                            assert!(matches!(
                                trace.action,
                                CycleAction::Write {
                                    addr: 0xFFFD,
                                    value: 1
                                }
                            ));
                            expected.sp = 0xFFFD;
                        }
                        _ => {
                            assert!(matches!(
                                trace.action,
                                CycleAction::Write {
                                    addr: 0xFFFC,
                                    value: 7
                                }
                            ));
                            expected.sp = 0xFFFC;
                            expected.pc = 0x0104;
                        }
                    }
                }
                assert_eq!(system.cpu_snapshot(), expected);
            }
        }
    }
}

#[test]
fn bounded_calls_resume_every_budget_and_stop_after_one_self_call() {
    for byte in [0xCD, 0xC4, 0xCC, 0xD4, 0xDC] {
        for equal in [false, true] {
            let do_call = taken(byte, equal);
            let cycles = if do_call { 6u64 } else { 3 };
            for budget in [0u64, 1, 2, 3, 4, 5, 6, 100] {
                let mut system = machine(byte, equal);
                let mut expected = system.cpu_snapshot();
                let outcome = system.step_instruction(budget).unwrap();
                if budget < cycles {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                    expected.pc += budget.min(3) as u16;
                    if budget == 5 {
                        expected.sp -= 1;
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
                    complete(system.step_instruction(100).unwrap(), cycles - budget);
                } else {
                    complete(outcome, cycles);
                }
                expected.pc = if do_call { 0x0104 } else { 0x0107 };
                expected.sp = if do_call { 0xFFFC } else { 0xFFFE };
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 16 + cycles * 4);
            }
        }
    }
}
