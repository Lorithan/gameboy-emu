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
fn absolute_a_four_action_traces_and_every_budget_resume() {
    for (opcode, store) in [(0xEA, true), (0xFA, false)] {
        for addr in [
            0u16, 0x7FFF, 0x8000, 0xC000, 0xDFFF, 0xE000, 0xFDFF, 0xFE00, 0xFF80, 0xFFFE, 0xFFFF,
        ] {
            let bytes = [0x3E, 0x5A, opcode, addr as u8, (addr / 256) as u8, 0, 0xD3];
            let read = if addr <= 0x7FFF || matches!(addr, 0xC000..=0xFDFF | 0xFF80..=0xFFFE) {
                0
            } else {
                0xFF
            };
            let mut system = program(&bytes);
            complete(&mut system, 2);
            let before = system.cpu_snapshot();
            let mut expected = before;
            expected.pc += 3;
            if !store {
                expected.af = u16::from(read) * 256 + (before.af & 0xF0);
            }
            for cycle in 0..4u64 {
                let trace = system.step_machine_cycle().unwrap();
                if cycle < 3 {
                    assert!(
                        matches!(trace.action, CycleAction::Read { addr } if addr == before.pc + cycle as u16)
                    );
                    assert_eq!(
                        trace.read_value,
                        Some([opcode, addr as u8, (addr / 256) as u8][cycle as usize])
                    );
                    let mut pending = before;
                    pending.pc += cycle as u16 + 1;
                    assert_eq!(system.cpu_snapshot(), pending);
                } else if store {
                    assert!(
                        matches!(trace.action, CycleAction::Write { addr: actual, value: 0x5A } if actual == addr)
                    );
                    assert_eq!(trace.read_value, None);
                } else {
                    assert!(
                        matches!(trace.action, CycleAction::Read { addr: actual } if actual == addr)
                    );
                    assert_eq!(trace.read_value, Some(read));
                }
                assert_eq!(
                    (trace.tick_before, trace.tick_after, trace.complete),
                    (8 + cycle * 4, 12 + cycle * 4, cycle == 3)
                );
            }
            assert_eq!(system.cpu_snapshot(), expected);
            for budget in [0, 1, 2, 3, 4, 100] {
                let mut system = program(&bytes);
                complete(&mut system, 2);
                let outcome = system.step_instruction(budget).unwrap();
                if budget < 4 {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                    let mut pending = before;
                    pending.pc += budget as u16;
                    assert_eq!(system.cpu_snapshot(), pending);
                    assert_eq!(system.elapsed_ticks(), 8 + budget * 4);
                    assert!(matches!(
                        system.step_instruction(0).unwrap(),
                        InstructionStep::BudgetExhausted {
                            mcycles: 0,
                            ticks: 0
                        }
                    ));
                    complete(&mut system, 4 - budget);
                } else {
                    assert!(matches!(
                        outcome,
                        InstructionStep::Complete {
                            mcycles: 4,
                            ticks: 16
                        }
                    ));
                }
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 24);
                complete(&mut system, 1);
                let mut after_nop = expected;
                after_nop.pc += 1;
                assert_eq!(system.cpu_snapshot(), after_nop);
            }
        }
    }
}
#[test]
fn absolute_a_every_byte_round_trips_through_wram_echo_and_hram() {
    for addr in [0xC123u16, 0xE123, 0xFF80, 0xFFFE] {
        for value in 0u8..=255 {
            let bytes = [
                0x3E,
                value,
                0xEA,
                addr as u8,
                (addr / 256) as u8,
                0x3E,
                0,
                0xFA,
                addr as u8,
                (addr / 256) as u8,
                0,
                0xD3,
            ];
            let mut system = program(&bytes);
            for cycles in [2, 4, 2, 4, 1] {
                complete(&mut system, cycles);
            }
            assert_eq!(system.cpu_snapshot().af, u16::from(value) * 256 + 0xB0);
            assert_eq!(system.cpu_snapshot().pc, 0x010B);
            assert_eq!(system.elapsed_ticks(), 52);
        }
    }
}
#[test]
fn absolute_pair_and_hl_transfers_share_the_same_memory() {
    let bytes = [
        0x3E, 0x42, 0xEA, 0x23, 0xC1, 0x01, 0x23, 0xC1, 0x0A, 0x21, 0x24, 0xC1, 0x22, 0xFA, 0x24,
        0xC1, 0x3E, 0x69, 0x02, 0xFA, 0x23, 0xC1, 0, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [2, 4, 3, 2, 3, 2, 4, 2, 2, 4, 1] {
        complete(&mut system, cycles);
    }
    let s = system.cpu_snapshot();
    assert_eq!(
        (s.af, s.bc, s.de, s.hl, s.sp),
        (0x69B0, 0xC123, 0x00D8, 0xC125, 0xFFFE)
    );
    assert_eq!(s.pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 116);
}
