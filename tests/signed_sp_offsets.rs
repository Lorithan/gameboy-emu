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
fn signed_sp_exact_cycles_no_early_changes_and_all_budget_resumes() {
    for (opcode, cycles) in [(0xF8, 3u64), (0xE8, 4)] {
        for (sp, raw, result, flags) in [
            (0u16, 0xFFu8, 0xFFFFu16, 0u16),
            (1, 0xFF, 0, 0x30),
            (0x000F, 1, 0x10, 0x20),
            (0x00FF, 1, 0x100, 0x30),
            (0x0100, 0x80, 0x0080, 0),
            (0xFFFF, 0x7F, 0x007E, 0x30),
        ] {
            let bytes = [0x31, sp as u8, (sp / 256) as u8, opcode, raw, 0, 0xD3];
            let mut system = program(&bytes);
            complete(&mut system, 3);
            let before = system.cpu_snapshot();
            let mut expected = before;
            expected.pc += 2;
            expected.af = (before.af & 0xFF00) + flags;
            if opcode == 0xF8 {
                expected.hl = result;
            } else {
                expected.sp = result;
            }
            for cycle in 0..cycles {
                let trace = system.step_machine_cycle().unwrap();
                match cycle {
                    0 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0103 }));
                        assert_eq!(trace.read_value, Some(opcode));
                    }
                    1 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0104 }));
                        assert_eq!(trace.read_value, Some(raw));
                    }
                    _ => {
                        assert!(matches!(trace.action, CycleAction::Idle));
                        assert_eq!(trace.read_value, None);
                    }
                }
                assert_eq!(
                    (trace.tick_before, trace.tick_after),
                    (12 + 4 * cycle, 16 + 4 * cycle)
                );
                assert_eq!(trace.complete, cycle + 1 == cycles);
                if !trace.complete {
                    let mut pending = before;
                    pending.pc += if cycle == 0 { 1 } else { 2 };
                    assert_eq!(system.cpu_snapshot(), pending);
                }
            }
            assert_eq!(system.cpu_snapshot(), expected);
            for budget in [0, 1, 2, 3, 4, 100] {
                let mut system = program(&bytes);
                complete(&mut system, 3);
                let outcome = system.step_instruction(budget).unwrap();
                if budget < cycles {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                    let mut pending = before;
                    pending.pc += budget.min(2) as u16;
                    assert_eq!(system.cpu_snapshot(), pending);
                    assert_eq!(system.elapsed_ticks(), 12 + budget * 4);
                    assert!(matches!(
                        system.step_instruction(0).unwrap(),
                        InstructionStep::BudgetExhausted {
                            mcycles: 0,
                            ticks: 0
                        }
                    ));
                    assert_eq!(system.cpu_snapshot(), pending);
                    complete(&mut system, cycles - budget);
                } else {
                    assert!(
                        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
                    );
                }
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 12 + cycles * 4);
                complete(&mut system, 1);
                let mut after_nop = expected;
                after_nop.pc += 1;
                assert_eq!(system.cpu_snapshot(), after_nop);
            }
        }
    }
}
#[test]
fn signed_sp_results_feed_hl_memory_and_sp_stack_operations() {
    let bytes = [
        0x31, 0x02, 0xC1, // SP=C102
        0xF8, 0xFE, // HL=C100, SP unchanged
        0x36, 0x42, 0x7E, // store then read A=42
        0xE8, 0xFE, // SP=C100, HL unchanged
        0x01, 0x34, 0x12, 0xC5, // push BC=1234 -> SP=C0FE
        0xE8, 2, 0xE8, 0xFE, // adjust SP up/down
        0xD1, // pop DE=1234, SP=C100
        0xF8, 0xFF, // HL=C0FF, SP unchanged
        0x00, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [3, 3, 3, 2, 4, 3, 4, 4, 4, 3, 3, 1] {
        complete(&mut system, cycles);
    }
    let snapshot = system.cpu_snapshot();
    assert_eq!(
        (
            snapshot.af,
            snapshot.bc,
            snapshot.de,
            snapshot.hl,
            snapshot.sp
        ),
        (0x4200, 0x1234, 0x1234, 0xC0FF, 0xC100)
    );
    assert_eq!(snapshot.pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 148);
}
