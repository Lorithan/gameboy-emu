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
fn updated(addr: u16, increment: bool) -> u16 {
    (i32::from(addr) + if increment { 1 } else { -1 }).rem_euclid(65536) as u16
}
#[test]
fn auto_update_transfers_have_exact_traces_and_single_updates_across_all_budgets() {
    for (opcode, store, increment) in [
        (0x22, true, true),
        (0x2A, false, true),
        (0x32, true, false),
        (0x3A, false, false),
    ] {
        for addr in [
            0u16, 0x7FFF, 0x8000, 0xC000, 0xDFFF, 0xE000, 0xFDFF, 0xFE00, 0xFF80, 0xFFFE, 0xFFFF,
        ] {
            let bytes = [
                0x21,
                addr as u8,
                (addr / 256) as u8,
                0x3E,
                0x5A,
                opcode,
                0,
                0xD3,
            ];
            let read = if addr <= 0x7FFF || matches!(addr, 0xC000..=0xFDFF | 0xFF80..=0xFFFE) {
                0
            } else {
                0xFF
            };
            for budget in [0, 1, 2, 100] {
                let mut system = program(&bytes);
                complete(&mut system, 3);
                complete(&mut system, 2);
                let before = system.cpu_snapshot();
                let mut expected = before;
                expected.pc += 1;
                expected.hl = updated(addr, increment);
                if !store {
                    expected.af = u16::from(read) * 256 + (before.af & 0xF0);
                }
                let outcome = system.step_instruction(budget).unwrap();
                if budget < 2 {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                    let mut pending = before;
                    pending.pc += budget as u16;
                    assert_eq!(system.cpu_snapshot(), pending);
                    assert_eq!(system.elapsed_ticks(), 20 + budget * 4);
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
                assert_eq!(system.elapsed_ticks(), 28);
                complete(&mut system, 1);
                expected.pc += 1;
                assert_eq!(system.cpu_snapshot(), expected);
                let mut system = program(&bytes);
                complete(&mut system, 3);
                complete(&mut system, 2);
                let fetch = system.step_machine_cycle().unwrap();
                assert!(matches!(fetch.action, CycleAction::Read { addr: 0x0105 }));
                assert_eq!(fetch.read_value, Some(opcode));
                assert_eq!(
                    (fetch.tick_before, fetch.tick_after, fetch.complete),
                    (20, 24, false)
                );
                let mut pending = before;
                pending.pc += 1;
                assert_eq!(system.cpu_snapshot(), pending);
                let data = system.step_machine_cycle().unwrap();
                if store {
                    assert!(
                        matches!(data.action, CycleAction::Write { addr: actual, value: 0x5A } if actual == addr)
                    );
                    assert_eq!(data.read_value, None);
                } else {
                    assert!(
                        matches!(data.action, CycleAction::Read { addr: actual } if actual == addr)
                    );
                    assert_eq!(data.read_value, Some(read));
                }
                assert_eq!(
                    (data.tick_before, data.tick_after, data.complete),
                    (24, 28, true)
                );
                expected.pc -= 1;
                assert_eq!(system.cpu_snapshot(), expected);
            }
        }
    }
}
#[test]
fn auto_update_round_trips_every_byte_in_both_directions() {
    for (store, load, restore, increment) in [(0x22, 0x2A, 0x2B, true), (0x32, 0x3A, 0x23, false)] {
        for addr in [0xC123u16, 0xE123, 0xFF80, 0xFFFE] {
            for value in 0u8..=255 {
                let bytes = [
                    0x21,
                    addr as u8,
                    (addr / 256) as u8,
                    0x3E,
                    value,
                    store,
                    restore,
                    0x3E,
                    0,
                    load,
                    0,
                    0xD3,
                ];
                let mut system = program(&bytes);
                for cycles in [3, 2, 2, 2, 2, 2, 1] {
                    complete(&mut system, cycles);
                }
                let snapshot = system.cpu_snapshot();
                assert_eq!(snapshot.af, u16::from(value) * 256 + 0xB0);
                assert_eq!(snapshot.hl, updated(addr, increment));
                assert_eq!(snapshot.pc, 0x010B);
                assert_eq!(system.elapsed_ticks(), 56);
            }
        }
    }
}
#[test]
fn auto_update_fills_forward_then_reads_the_buffer_backwards() {
    let bytes = [
        0x21, 0, 0xC0, 0x3E, 0x11, 0x22, 0x3E, 0x22, 0x22, 0x3E, 0x33, 0x22, 0x2B, 0x3A, 0x3A,
        0x3A, 0, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [3, 2, 2, 2, 2, 2, 2, 2] {
        complete(&mut system, cycles);
    }
    assert_eq!(system.cpu_snapshot().hl, 0xC002);
    for (a, hl) in [(0x33u16, 0xC001), (0x22, 0xC000), (0x11, 0xBFFF)] {
        complete(&mut system, 2);
        assert_eq!(
            (system.cpu_snapshot().af, system.cpu_snapshot().hl),
            (a * 256 + 0xB0, hl)
        );
    }
    complete(&mut system, 1);
    assert_eq!(system.cpu_snapshot().pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 96);
}
