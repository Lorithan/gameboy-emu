use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(sp: u16) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    // PUSH 1234, overwrite BC, then POP it back. Default and WRAM stacks.
    let program = [
        0x31,
        sp as u8,
        (sp / 256) as u8,
        0x01,
        0x34,
        0x12,
        0xC5,
        0x01,
        0xCD,
        0xAB,
        0xC1,
        0xD3,
    ];
    rom[0x0100..0x0100 + program.len()].copy_from_slice(&program);
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    for cycles in [3, 3, 4, 3] {
        completed(system.step_instruction(100).unwrap(), cycles);
    }
    system
}

fn completed(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

#[test]
fn pop_bc_reads_low_then_high_with_atomic_pair_assignment_and_exact_timing() {
    for sp in [0xFFFEu16, 0xC002] {
        let mut system = machine(sp);
        let mut expected = system.cpu_snapshot();
        for cycle in 0u64..3 {
            let trace = system.step_machine_cycle().unwrap();
            assert_eq!(
                (trace.tick_before, trace.tick_after),
                (52 + cycle * 4, 56 + cycle * 4)
            );
            assert_eq!(trace.complete, cycle == 2);
            let (addr, data) = match cycle {
                0 => (0x010A, 0xC1),
                1 => (sp - 2, 0x34),
                _ => (sp - 1, 0x12),
            };
            assert!(matches!(trace.action, CycleAction::Read { addr: actual } if actual == addr));
            assert_eq!(trace.read_value, Some(data));
            expected.pc = 0x010B;
            if cycle == 1 {
                expected.sp = sp - 1;
            }
            if cycle == 2 {
                expected.sp = sp;
                expected.bc = 0x1234;
            }
            assert_eq!(system.cpu_snapshot(), expected);
        }
        assert_eq!(system.elapsed_ticks(), 64);
    }
}

#[test]
fn pop_bc_resumes_each_budget_and_stops_before_the_next_opcode() {
    for sp in [0xFFFEu16, 0xC002] {
        for budget in [0u64, 1, 2, 3, 100] {
            let mut system = machine(sp);
            let mut expected = system.cpu_snapshot();
            let outcome = system.step_instruction(budget).unwrap();
            if budget < 3 {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                );
                if budget > 0 {
                    expected.pc = 0x010B;
                }
                if budget > 1 {
                    expected.sp = sp - 1;
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
                completed(system.step_instruction(100).unwrap(), 3 - budget);
            } else {
                completed(outcome, 3);
            }
            expected.pc = 0x010B;
            expected.sp = sp;
            expected.bc = 0x1234;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 64);
        }
    }
}
