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
fn load_sp_from_hl_has_exact_read_idle_trace_and_every_budget_resume() {
    for hl in [0u16, 0xC100, 0xFFFE, 0xFFFF] {
        let bytes = [0x21, hl as u8, (hl / 256) as u8, 0xF9, 0, 0xD3];
        for budget in [0, 1, 2, 100] {
            let mut system = program(&bytes);
            complete(&mut system, 3);
            let before = system.cpu_snapshot();
            let mut expected = before;
            expected.pc += 1;
            expected.sp = hl;
            let outcome = system.step_instruction(budget).unwrap();
            if budget < 2 {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                );
                let mut pending = before;
                pending.pc += budget as u16;
                assert_eq!(system.cpu_snapshot(), pending);
                assert_eq!(system.elapsed_ticks(), 12 + budget * 4);
                assert!(matches!(
                    system.step_instruction(0).unwrap(),
                    InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    }
                ));
                complete(&mut system, 2 - budget);
            } else {
                assert!(matches!(
                    outcome,
                    InstructionStep::Complete {
                        mcycles: 2,
                        ticks: 8
                    }
                ));
            }
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 20);
            complete(&mut system, 1);
            expected.pc += 1;
            assert_eq!(system.cpu_snapshot(), expected);
            let mut system = program(&bytes);
            complete(&mut system, 3);
            let fetch = system.step_machine_cycle().unwrap();
            assert!(matches!(fetch.action, CycleAction::Read { addr: 0x0103 }));
            assert_eq!(fetch.read_value, Some(0xF9));
            assert_eq!(
                (fetch.tick_before, fetch.tick_after, fetch.complete),
                (12, 16, false)
            );
            let mut pending = before;
            pending.pc += 1;
            assert_eq!(system.cpu_snapshot(), pending);
            let idle = system.step_machine_cycle().unwrap();
            assert!(matches!(idle.action, CycleAction::Idle));
            assert_eq!(idle.read_value, None);
            assert_eq!(
                (idle.tick_before, idle.tick_after, idle.complete),
                (16, 20, true)
            );
            expected.pc -= 1;
            assert_eq!(system.cpu_snapshot(), expected);
        }
    }
}
#[test]
fn load_sp_from_hl_sets_the_stack_pointer_used_by_push_and_pop() {
    let bytes = [
        0x21, 0, 0xC1, 0xF9, 0x01, 0x34, 0x12, 0xC5, 0x11, 0, 0, 0xD1, 0, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [3, 2, 3, 4, 3, 3, 1] {
        complete(&mut system, cycles);
    }
    let s = system.cpu_snapshot();
    assert_eq!(
        (s.af, s.bc, s.de, s.hl, s.sp),
        (0x01B0, 0x1234, 0x1234, 0xC100, 0xC100)
    );
    assert_eq!(s.pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 76);
}
