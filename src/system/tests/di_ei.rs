use crate::outcome_support::instruction_complete;

use super::super::{InstructionStep, System};
use crate::memory::Cartridge;

#[test]
fn di_ei_system_budgets_do_not_advance_ime_until_instruction_completion() {
    let mut followers = vec![
        (vec![0], 1),
        (vec![0x3E, 0x42], 2),
        (vec![0xF3], 1),
        (vec![0xFB], 1),
    ];
    for opcode in 0u8..=255 {
        let cycles = if opcode % 8 != 6 {
            2
        } else if (0x40..0x80).contains(&opcode) {
            3
        } else {
            4
        };
        followers.push((vec![0xCB, opcode], cycles));
    }
    for (bytes, cycles) in followers {
        for budget in 0..=cycles {
            let mut rom = vec![0; 32768];
            rom[0x14D] = 0xE7;
            rom[0x100] = 0xFB;
            rom[0x101..0x101 + bytes.len()].copy_from_slice(&bytes);
            let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
            assert_eq!(system.cpu.test_interrupt_status(), (false, false));
            system.step_instruction(1).unwrap();
            assert_eq!(system.cpu.test_interrupt_status(), (true, false));
            let outcome = system.step_instruction(budget).unwrap();
            if budget < cycles {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                );
                assert_eq!(system.cpu.test_interrupt_status(), (true, false));
                let snapshot = system.cpu_snapshot();
                let ticks = system.elapsed_ticks();
                system.step_instruction(0).unwrap();
                assert_eq!(system.cpu_snapshot(), snapshot);
                assert_eq!(system.elapsed_ticks(), ticks);
                assert_eq!(system.cpu.test_interrupt_status(), (true, false));
                assert!(
                    matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles - budget && ticks == (cycles - budget) * 4)
                );
            } else {
                assert!(
                    matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
                );
            }
            assert_eq!(
                system.cpu.test_interrupt_status(),
                if bytes[0] == 0xF3 {
                    (false, false)
                } else {
                    (false, true)
                }
            );
            assert_eq!(system.cpu_snapshot().pc, 0x101 + bytes.len() as u16);
            assert_eq!(system.elapsed_ticks(), (1 + cycles) * 4);
        }
    }
}

#[test]
fn di_ei_sequences_have_single_fetch_cycles_and_preserve_registers() {
    use crate::cpu::CycleAction;
    let bytes = [0xFB, 0, 0xF3, 0xFB, 0xFB, 0xF3];
    let expected_status = [
        (true, false),
        (false, true),
        (false, false),
        (true, false),
        (false, true),
        (false, false),
    ];
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x106].copy_from_slice(&bytes);
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    let mut expected = system.cpu_snapshot();
    for (index, opcode) in bytes.into_iter().enumerate() {
        let trace = system.step_machine_cycle().unwrap();
        assert!(matches!(trace.record.action, CycleAction::Read { addr } if addr == expected.pc));
        assert_eq!(trace.record.read_value, Some(opcode));
        assert_eq!(
            (
                trace.record.tick_before,
                trace.record.tick_after,
                instruction_complete(trace.outcome)
            ),
            (index as u64 * 4, (index as u64 + 1) * 4, true)
        );
        expected.pc += 1;
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.cpu.test_interrupt_status(), expected_status[index]);
    }
}
