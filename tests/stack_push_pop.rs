use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(sp: u16) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0108].copy_from_slice(&[
        0x01,
        0x34,
        0x12,
        0x31,
        sp as u8,
        (sp / 256) as u8,
        0xC5,
        0xD3,
    ]);
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    for _ in 0..2 {
        completed(system.step_instruction(100).unwrap(), 3);
    }
    system
}

fn completed(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

#[test]
fn push_bc_has_read_idle_high_write_low_write_and_per_cycle_sp_updates() {
    for start in [0u16, 1, 0x8000, 0xC002, 0xFF80, 0xFFFE, 0xFFFF] {
        let mut system = machine(start);
        let mut expected = system.cpu_snapshot();
        for cycle in 0u64..4 {
            let trace = system.step_machine_cycle().unwrap();
            assert_eq!(
                (trace.tick_before, trace.tick_after),
                (24 + cycle * 4, 28 + cycle * 4)
            );
            assert_eq!(trace.complete, cycle == 3);
            match cycle {
                0 => {
                    assert!(matches!(trace.action, CycleAction::Read { addr: 0x0106 }));
                    assert_eq!(trace.read_value, Some(0xC5));
                    expected.pc = 0x0107;
                }
                1 => assert!(matches!(trace.action, CycleAction::Idle)),
                2 => {
                    assert!(
                        matches!(trace.action, CycleAction::Write { addr, value: 0x12 } if addr == start.wrapping_sub(1))
                    );
                    expected.sp = start.wrapping_sub(1);
                }
                _ => {
                    assert!(
                        matches!(trace.action, CycleAction::Write { addr, value: 0x34 } if addr == start.wrapping_sub(2))
                    );
                    expected.sp = start.wrapping_sub(2);
                }
            }
            if cycle != 0 {
                assert_eq!(trace.read_value, None);
            }
            assert_eq!(system.cpu_snapshot(), expected);
        }
        assert_eq!(system.elapsed_ticks(), 40);
    }
}

#[test]
fn push_bc_resumes_every_budget_without_extra_fetch_or_sp_decrements() {
    for budget in [0u64, 1, 2, 3, 4, 100] {
        let mut system = machine(0xFFFE);
        let mut expected = system.cpu_snapshot();
        let outcome = system.step_instruction(budget).unwrap();
        if budget < 4 {
            assert!(
                matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
            );
            expected.pc += u16::from(budget > 0);
            if budget == 3 {
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
            completed(system.step_instruction(100).unwrap(), 4 - budget);
        } else {
            completed(outcome, 4);
        }
        expected.pc = 0x0107;
        expected.sp = 0xFFFC;
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.elapsed_ticks(), 40);
    }
}
