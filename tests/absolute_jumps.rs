use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn completed(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

fn exhausted(outcome: InstructionStep, cycles: u64) {
    assert!(
        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
    );
}

fn prepared(byte: u8, equal: bool) -> System {
    // CP sets Z and C to opposite values, covering both outcomes per condition.
    let mut system = machine(&[0x3E, u8::from(equal), 0xFE, 1, byte, 0x04, 0x01, 0xD3]);
    completed(system.step_instruction(100).unwrap(), 2);
    completed(system.step_instruction(100).unwrap(), 2);
    system
}

#[test]
fn immediate_jump_traces_and_budgets_cover_taken_and_untaken_paths() {
    for byte in [0xC3, 0xC2, 0xCA, 0xD2, 0xDA] {
        for equal in [false, true] {
            let taken = match byte {
                0xC3 => true,
                0xC2 | 0xDA => !equal,
                _ => equal,
            };
            let cycles = if taken { 4u64 } else { 3 };
            let mut system = prepared(byte, equal);
            let before = system.cpu_snapshot();
            for cycle in 0..cycles {
                let trace = system.step_machine_cycle().unwrap();
                assert_eq!(trace.tick_before, 16 + cycle * 4);
                assert_eq!(trace.tick_after, 20 + cycle * 4);
                assert_eq!(trace.complete, cycle + 1 == cycles);
                if cycle < 3 {
                    assert!(
                        matches!(trace.action, CycleAction::Read { addr } if addr == 0x0104 + cycle as u16)
                    );
                    assert_eq!(trace.read_value, Some([byte, 0x04, 0x01][cycle as usize]));
                } else {
                    assert!(matches!(trace.action, CycleAction::Idle));
                    assert_eq!(trace.read_value, None);
                }
                let mut expected = before;
                expected.pc = if cycle < 3 {
                    0x0105 + cycle as u16
                } else {
                    0x0104
                };
                assert_eq!(system.cpu_snapshot(), expected);
            }
            for budget in [0, 1, 2, 3, 4, 100] {
                let mut system = prepared(byte, equal);
                let mut expected = system.cpu_snapshot();
                let outcome = system.step_instruction(budget).unwrap();
                if budget < cycles {
                    exhausted(outcome, budget);
                    let paused = system.cpu_snapshot();
                    exhausted(system.step_instruction(0).unwrap(), 0);
                    assert_eq!(system.cpu_snapshot(), paused);
                    completed(system.step_instruction(100).unwrap(), cycles - budget);
                } else {
                    completed(outcome, cycles);
                }
                expected.pc = if taken { 0x0104 } else { 0x0107 };
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 16 + cycles * 4);
            }
        }
    }
}

#[test]
fn jump_hl_has_one_opcode_read_and_never_dereferences_the_target() {
    for target in [0u16, 0x0103, 0x1234, 0x7FFF, 0xC123, 0xE123, 0xFFFF] {
        for budget in [0, 1, 100] {
            let mut system = machine(&[0x21, target as u8, (target / 256) as u8, 0xE9, 0xD3]);
            completed(system.step_instruction(100).unwrap(), 3);
            let mut expected = system.cpu_snapshot();
            if budget == 0 {
                exhausted(system.step_instruction(0).unwrap(), 0);
                assert_eq!(system.cpu_snapshot(), expected);
                let trace = system.step_machine_cycle().unwrap();
                assert!(matches!(trace.action, CycleAction::Read { addr: 0x0103 }));
                assert_eq!(trace.read_value, Some(0xE9));
                assert_eq!(
                    (trace.tick_before, trace.tick_after, trace.complete),
                    (12, 16, true)
                );
            } else {
                completed(system.step_instruction(budget).unwrap(), 1);
            }
            expected.pc = target;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 16);
        }
    }
}

#[test]
fn mixed_jumps_skip_sentinels_and_stop_at_each_instruction_boundary() {
    let mut bytes = vec![0xD3; 0x32];
    bytes[0..2].copy_from_slice(&[0x18, 2]); // JR -> 0104
    bytes[4..7].copy_from_slice(&[0xC3, 0x20, 1]); // JP -> 0120
    bytes[0x20..0x24].copy_from_slice(&[0x21, 0x30, 1, 0xE9]); // LD HL,0130; JP HL
    bytes[0x30] = 0x00;
    let mut system = machine(&bytes);
    let mut expected = system.cpu_snapshot();
    for (cycles, pc) in [
        (3, 0x0104),
        (4, 0x0120),
        (3, 0x0123),
        (1, 0x0130),
        (1, 0x0131),
    ] {
        completed(system.step_instruction(100).unwrap(), cycles);
        expected.pc = pc;
        if pc == 0x0123 {
            expected.hl = 0x0130;
        }
        assert_eq!(system.cpu_snapshot(), expected);
    }
    assert_eq!(system.elapsed_ticks(), 48);
}
