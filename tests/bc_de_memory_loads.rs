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
fn indirect_pair_transfers_have_exact_bus_traces_and_all_budget_resumes() {
    for (load_pair, opcode, store) in [
        (0x01, 0x02, true),
        (0x01, 0x0A, false),
        (0x11, 0x12, true),
        (0x11, 0x1A, false),
    ] {
        for addr in [
            0u16, 0x7FFF, 0x8000, 0xC000, 0xDFFF, 0xE000, 0xFDFF, 0xFE00, 0xFF80, 0xFFFE, 0xFFFF,
        ] {
            let bytes = [
                load_pair,
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
fn indirect_pair_round_trips_every_byte_in_wram_echo_and_hram() {
    for (load_pair, store, load) in [(0x01, 0x02, 0x0A), (0x11, 0x12, 0x1A)] {
        for addr in [0xC123u16, 0xE123, 0xFF80, 0xFFFE] {
            for value in 0u8..=255 {
                let bytes = [
                    load_pair,
                    addr as u8,
                    (addr / 256) as u8,
                    0x3E,
                    value,
                    store,
                    0x3E,
                    0,
                    load,
                    0,
                    0xD3,
                ];
                let mut system = program(&bytes);
                for cycles in [3, 2, 2, 2, 2, 1] {
                    complete(&mut system, cycles);
                }
                assert_eq!(system.cpu_snapshot().af, u16::from(value) * 256 + 0xB0);
                assert_eq!(
                    if load_pair == 1 {
                        system.cpu_snapshot().bc
                    } else {
                        system.cpu_snapshot().de
                    },
                    addr
                );
                assert_eq!(system.cpu_snapshot().pc, 0x010A);
                assert_eq!(system.elapsed_ticks(), 48);
            }
        }
    }
}
