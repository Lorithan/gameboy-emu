use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine() -> System {
    program(&[0x18, 0xFE])
}

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn conditional_machine(byte: u8, equal: bool) -> System {
    // LD A,0/1; CP 1 yields Z=false,C=true or Z=true,C=false.
    let mut system = program(&[0x3E, u8::from(equal), 0xFE, 1, byte, 0xFE]);
    system.step_instruction(100).unwrap();
    system.step_instruction(100).unwrap();
    system
}

#[test]
fn conditional_traces_and_budgets_match_both_branch_outcomes() {
    for byte in [0x20, 0x28, 0x30, 0x38] {
        for equal in [false, true] {
            let taken = match byte {
                0x20 | 0x38 => !equal,
                _ => equal,
            };
            let cycles = if taken { 3u64 } else { 2 };
            let mut system = conditional_machine(byte, equal);
            let before = system.cpu_snapshot();
            for cycle in 0..cycles {
                let trace = system.step_machine_cycle().unwrap();
                assert_eq!(trace.tick_before, 16 + cycle * 4);
                assert_eq!(trace.tick_after, 20 + cycle * 4);
                assert_eq!(trace.complete, cycle + 1 == cycles);
                match cycle {
                    0 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0104 }));
                        assert_eq!(trace.read_value, Some(byte));
                    }
                    1 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0105 }));
                        assert_eq!(trace.read_value, Some(0xFE));
                    }
                    _ => {
                        assert!(matches!(trace.action, CycleAction::Idle));
                        assert_eq!(trace.read_value, None);
                    }
                }
                let mut expected = before;
                expected.pc = match cycle {
                    0 => 0x0105,
                    1 => 0x0106,
                    _ => 0x0104,
                };
                assert_eq!(system.cpu_snapshot(), expected);
            }
            for budget in [0, 1, 2, 3, 100] {
                let mut system = conditional_machine(byte, equal);
                let initial = system.cpu_snapshot();
                let outcome = system.step_instruction(budget).unwrap();
                let spent = budget.min(cycles);
                if budget < cycles {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == spent && ticks == spent * 4)
                    );
                    let paused = system.cpu_snapshot();
                    assert!(matches!(
                        system.step_instruction(0).unwrap(),
                        InstructionStep::BudgetExhausted {
                            mcycles: 0,
                            ticks: 0
                        }
                    ));
                    assert_eq!(system.cpu_snapshot(), paused);
                    assert!(
                        matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles - spent && ticks == (cycles - spent) * 4)
                    );
                } else {
                    assert!(
                        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
                    );
                }
                let mut expected = initial;
                expected.pc = if taken { 0x0104 } else { 0x0106 };
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 16 + cycles * 4);
            }
        }
    }
}

#[test]
fn finite_decrement_loop_takes_then_exits_with_exact_timing() {
    // LD B,3; loop: DEC B; JR NZ,loop. Stop before the unsupported sentinel.
    let mut system = program(&[0x06, 3, 0x05, 0x20, 0xFD, 0xD3]);
    let initial = system.cpu_snapshot();
    assert!(matches!(
        system.step_instruction(100).unwrap(),
        InstructionStep::Complete {
            mcycles: 2,
            ticks: 8
        }
    ));
    for remaining in [2u16, 1, 0] {
        assert!(matches!(
            system.step_instruction(100).unwrap(),
            InstructionStep::Complete {
                mcycles: 1,
                ticks: 4
            }
        ));
        assert_eq!(
            system.cpu_snapshot().bc,
            (remaining << 8) | (initial.bc & 0xFF)
        );
        let cycles = if remaining == 0 { 2 } else { 3 };
        assert!(
            matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
        );
        assert_eq!(
            system.cpu_snapshot().pc,
            if remaining == 0 { 0x0105 } else { 0x0102 }
        );
    }
    let mut expected = initial;
    expected.bc &= 0xFF;
    expected.af = (initial.af & 0xFF00) | 0xD0;
    expected.pc = 0x0105;
    assert_eq!(system.cpu_snapshot(), expected);
    assert_eq!(system.elapsed_ticks(), 52);
}

#[test]
fn backward_jump_has_two_reads_and_one_idle_with_target_committed_last() {
    let mut system = machine();
    let mut expected = system.cpu_snapshot();
    for (cycle, pc) in [(0, 0x0101), (1, 0x0102), (2, 0x0100)] {
        let trace = system.step_machine_cycle().unwrap();
        assert_eq!(trace.tick_before, cycle * 4);
        assert_eq!(trace.tick_after, (cycle + 1) * 4);
        assert_eq!(trace.complete, cycle == 2);
        match cycle {
            0 => {
                assert!(matches!(trace.action, CycleAction::Read { addr: 0x0100 }));
                assert_eq!(trace.read_value, Some(0x18));
            }
            1 => {
                assert!(matches!(trace.action, CycleAction::Read { addr: 0x0101 }));
                assert_eq!(trace.read_value, Some(0xFE));
            }
            _ => {
                assert!(matches!(trace.action, CycleAction::Idle));
                assert_eq!(trace.read_value, None);
            }
        }
        expected.pc = pc;
        assert_eq!(system.cpu_snapshot(), expected);
    }
    assert_eq!(system.elapsed_ticks(), 12);
}

#[test]
fn self_loop_resumes_budgets_and_completes_one_instruction_only() {
    for budget in [0u64, 1, 2, 3, 100] {
        let mut system = machine();
        let before = system.cpu_snapshot();
        let outcome = system.step_instruction(budget).unwrap();
        if budget < 3 {
            assert!(
                matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
            );
            assert_eq!(system.cpu_snapshot().pc, 0x0100 + budget as u16);
            assert!(
                matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == 3 - budget && ticks == (3 - budget) * 4)
            );
        } else {
            assert!(matches!(
                outcome,
                InstructionStep::Complete {
                    mcycles: 3,
                    ticks: 12
                }
            ));
        }
        assert_eq!(system.cpu_snapshot(), before);
        assert_eq!(system.elapsed_ticks(), 12);
    }
}
