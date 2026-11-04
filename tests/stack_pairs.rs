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

#[test]
fn all_pairs_round_trip_with_each_push_and_pop_budget() {
    for sp in [0xFFFEu16, 0xC100] {
        for (push, pop) in [(0xC5, 0xC1), (0xD5, 0xD1), (0xE5, 0xE1), (0xF5, 0xF1)] {
            for push_budget in [0u64, 1, 2, 3, 4, 100] {
                for pop_budget in [0u64, 1, 2, 3, 100] {
                    let mut system = machine(&[0x31, sp as u8, (sp / 256) as u8, push, pop, 0xD3]);
                    completed(system.step_instruction(100).unwrap(), 3);
                    let initial = system.cpu_snapshot();
                    for (budget, cycles, pc, start_sp, is_push) in [
                        (push_budget, 4u64, 0x0103, sp, true),
                        (pop_budget, 3, 0x0104, sp - 2, false),
                    ] {
                        let outcome = system.step_instruction(budget).unwrap();
                        if budget < cycles {
                            assert!(
                                matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                            );
                            let mut expected = initial;
                            expected.pc = pc + u16::from(budget > 0);
                            expected.sp = start_sp;
                            if is_push && budget == 3 {
                                expected.sp -= 1;
                            }
                            if !is_push && budget == 2 {
                                expected.sp += 1;
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
                            completed(system.step_instruction(100).unwrap(), cycles - budget);
                        } else {
                            completed(outcome, cycles);
                        }
                    }
                    let mut expected = initial;
                    expected.pc = 0x0105;
                    assert_eq!(system.cpu_snapshot(), expected);
                    assert_eq!(system.elapsed_ticks(), 40);
                }
            }
        }
    }
}

#[test]
fn pop_af_masks_all_low_bytes_in_memory_and_push_af_writes_masked_flags() {
    for sp in [0xFFFEu16, 0xC100] {
        for low in 0u8..=255 {
            for high in [0u8, 0x42, 0xFF] {
                let mut system = machine(&[
                    0x31,
                    sp as u8,
                    (sp / 256) as u8,
                    0x01,
                    low,
                    high,
                    0xC5,
                    0xF1,
                    0xF5,
                    0xD3,
                ]);
                for cycles in [3, 3, 4] {
                    completed(system.step_instruction(100).unwrap(), cycles);
                }
                let mut expected = system.cpu_snapshot();
                let opcode = system.step_machine_cycle().unwrap();
                assert_eq!(opcode.read_value, Some(0xF1));
                assert!(!opcode.complete);
                let low_read = system.step_machine_cycle().unwrap();
                assert!(matches!(low_read.action, CycleAction::Read { addr } if addr == sp - 2));
                assert_eq!(low_read.read_value, Some(low));
                assert!(!low_read.complete);
                expected.pc = 0x0108;
                expected.sp = sp - 1;
                assert_eq!(system.cpu_snapshot(), expected);
                let high_read = system.step_machine_cycle().unwrap();
                assert!(matches!(high_read.action, CycleAction::Read { addr } if addr == sp - 1));
                assert_eq!(high_read.read_value, Some(high));
                assert!(high_read.complete);
                expected.sp = sp;
                expected.af = u16::from(high) * 256 + u16::from(low & 0xF0);
                assert_eq!(system.cpu_snapshot(), expected);
                assert!(!system.step_machine_cycle().unwrap().complete); // PUSH AF opcode
                assert!(matches!(
                    system.step_machine_cycle().unwrap().action,
                    CycleAction::Idle
                ));
                let high_write = system.step_machine_cycle().unwrap();
                assert!(
                    matches!(high_write.action, CycleAction::Write { addr, value } if addr == sp - 1 && value == high)
                );
                let low_write = system.step_machine_cycle().unwrap();
                assert!(
                    matches!(low_write.action, CycleAction::Write { addr, value } if addr == sp - 2 && value == low & 0xF0)
                );
                assert!(low_write.complete);
                expected.sp = sp - 2;
                expected.pc = 0x0109;
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 68);
            }
        }
    }
}

#[test]
fn nested_distinct_pairs_restore_registers_and_sp_in_lifo_order() {
    for sp in [0xFFFEu16, 0xC100] {
        let mut system = machine(&[
            0x31,
            sp as u8,
            (sp / 256) as u8,
            0x01,
            0x34,
            0x12,
            0x11,
            0x78,
            0x56,
            0x21,
            0xBC,
            0x9A,
            0xC5,
            0xD5,
            0xE5,
            0xF5,
            0x01,
            0,
            0,
            0x11,
            0,
            0,
            0x21,
            0,
            0,
            0x3E,
            0,
            0xF1,
            0xE1,
            0xD1,
            0xC1,
            0xD3,
        ]);
        for _ in 0..4 {
            completed(system.step_instruction(100).unwrap(), 3);
        }
        let initial = system.cpu_snapshot();
        for depth in 1..=4 {
            completed(system.step_instruction(100).unwrap(), 4);
            assert_eq!(system.cpu_snapshot().sp, sp - depth * 2);
        }
        for cycles in [3, 3, 3, 2] {
            completed(system.step_instruction(100).unwrap(), cycles);
        }
        for (index, expected_af, expected_hl, expected_de, expected_bc) in [
            (0, initial.af, 0, 0, 0),
            (1, initial.af, initial.hl, 0, 0),
            (2, initial.af, initial.hl, initial.de, 0),
            (3, initial.af, initial.hl, initial.de, initial.bc),
        ] {
            completed(system.step_instruction(100).unwrap(), 3);
            let snapshot = system.cpu_snapshot();
            assert_eq!(
                (snapshot.af, snapshot.hl, snapshot.de, snapshot.bc),
                (expected_af, expected_hl, expected_de, expected_bc)
            );
            assert_eq!(snapshot.sp, sp - 6 + index * 2);
        }
        let mut expected = initial;
        expected.pc = 0x011F;
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.elapsed_ticks(), 204);
    }
}
