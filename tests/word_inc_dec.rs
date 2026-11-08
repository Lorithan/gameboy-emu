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
fn word_updates_have_read_idle_traces_and_resume_every_budget() {
    for (load, inc, dec, pair) in [
        (0x01, 0x03, 0x0B, 0),
        (0x11, 0x13, 0x1B, 1),
        (0x21, 0x23, 0x2B, 2),
        (0x31, 0x33, 0x3B, 3),
    ] {
        for (opcode, start, result) in [(inc, 0xFFFFu16, 0u16), (dec, 0u16, 0xFFFFu16)] {
            for budget in [0, 1, 2, 100] {
                let bytes = [load, start as u8, (start / 256) as u8, opcode, 0x00, 0xD3];
                let mut system = program(&bytes);
                complete(&mut system, 3);
                let before = system.cpu_snapshot();
                let mut expected = before;
                expected.pc += 1;
                match pair {
                    0 => expected.bc = result,
                    1 => expected.de = result,
                    2 => expected.hl = result,
                    _ => expected.sp = result,
                }
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
                    assert_eq!(system.cpu_snapshot(), pending);
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
                assert_eq!(fetch.read_value, Some(opcode));
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
}

#[test]
fn updated_hl_and_sp_feed_memory_and_stack_operations() {
    let bytes = [
        0x21, 0xFF, 0xC0, 0x23, // HL C0FF -> C100
        0x36, 0x42, 0x2B, // store 42, HL -> C0FF
        0x36, 0x24, 0x23, 0x7E, // store 24, HL -> C100, A=42
        0x31, 0x02, 0xC1, 0x3B, // SP C102 -> C101
        0x01, 0x34, 0x12, 0xC5, // BC 1234, push -> SP C0FF
        0x33, 0x3B, 0xD1, // adjust SP up/down, POP DE ->1234
        0x33, 0x00, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [3, 2, 3, 2, 3, 2, 2, 3, 2, 3, 4, 2, 2, 3, 2, 1] {
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
        (0x42B0, 0x1234, 0x1234, 0xC100, 0xC102)
    );
    assert_eq!(snapshot.pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 156);
}
