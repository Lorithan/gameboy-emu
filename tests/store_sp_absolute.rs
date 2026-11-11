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
fn store_sp_absolute_has_five_cycles_and_resumes_between_both_writes() {
    for target in [
        0u16, 0x7FFF, 0xBFFF, 0xC000, 0xDFFF, 0xFDFF, 0xFF7F, 0xFF80, 0xFFFE, 0xFFFF,
    ] {
        let bytes = [
            0x31,
            0xA5,
            0x5A,
            0x08,
            target as u8,
            (target / 256) as u8,
            0,
            0xD3,
        ];
        let mut system = program(&bytes);
        complete(&mut system, 3);
        let before = system.cpu_snapshot();
        let mut expected = before;
        expected.pc += 3;
        for cycle in 0..5u64 {
            let trace = system.step_machine_cycle().unwrap();
            if cycle < 3 {
                assert!(
                    matches!(trace.action, CycleAction::Read { addr } if addr == 0x0103 + cycle as u16)
                );
                assert_eq!(
                    trace.read_value,
                    Some([0x08, target as u8, (target / 256) as u8][cycle as usize])
                );
            } else {
                let addr = if cycle == 3 {
                    target
                } else {
                    target.wrapping_add(1)
                };
                let value = if cycle == 3 { 0xA5 } else { 0x5A };
                assert!(
                    matches!(trace.action, CycleAction::Write { addr: actual, value: actual_value } if actual == addr && actual_value == value)
                );
                assert_eq!(trace.read_value, None);
            }
            assert_eq!(
                (trace.tick_before, trace.tick_after, trace.complete),
                (12 + cycle * 4, 16 + cycle * 4, cycle == 4)
            );
            let mut pending = before;
            pending.pc += (cycle + 1).min(3) as u16;
            assert_eq!(system.cpu_snapshot(), pending);
        }
        assert_eq!(system.cpu_snapshot(), expected);
        for budget in [0, 1, 2, 3, 4, 5, 100] {
            let mut system = program(&bytes);
            complete(&mut system, 3);
            let outcome = system.step_instruction(budget).unwrap();
            if budget < 5 {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                );
                let mut pending = before;
                pending.pc += budget.min(3) as u16;
                assert_eq!(system.cpu_snapshot(), pending);
                assert_eq!(system.elapsed_ticks(), 12 + budget * 4);
                assert!(matches!(
                    system.step_instruction(0).unwrap(),
                    InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    }
                ));
                complete(&mut system, 5 - budget);
            } else {
                assert!(matches!(
                    outcome,
                    InstructionStep::Complete {
                        mcycles: 5,
                        ticks: 20
                    }
                ));
            }
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 32);
            complete(&mut system, 1);
            let mut after = expected;
            after.pc += 1;
            assert_eq!(system.cpu_snapshot(), after);
        }
    }
}
#[test]
fn store_sp_absolute_bytes_round_trip_without_changing_the_active_stack() {
    let bytes = [
        0x31, 0, 0xC1, 0x01, 0xA5, 0x5A, 0xC5, 0x08, 0x80, 0xFF, 0xFA, 0x80, 0xFF, 0x47, 0xFA,
        0x81, 0xFF, 0x57, 0xE1, 0, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [3, 3, 4, 5, 4, 1, 4, 1, 3, 1] {
        complete(&mut system, cycles);
    }
    let s = system.cpu_snapshot();
    assert_eq!(
        (s.af, s.bc, s.de, s.hl, s.sp),
        (0xC0B0, 0xFEA5, 0xC0D8, 0x5AA5, 0xC100)
    );
    assert_eq!(s.pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 116);
}
