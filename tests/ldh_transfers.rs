#[path = "support/steps.rs"]
mod step_support;
use gameboy_emu::system::{CycleTrace, MachineStep};
use step_support::expect_cycle;

#[path = "support/outcomes.rs"]
mod outcome_support;
use gameboy_emu::cpu::CycleOutcome as InstructionCycleOutcome;
use outcome_support::instruction_complete;

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
fn ldh_exact_traces_and_all_budget_resumes_match_each_form() {
    for (opcode, store, immediate) in [
        (0xE2, true, false),
        (0xF2, false, false),
        (0xE0, true, true),
        (0xF0, false, true),
    ] {
        for offset in [
            0u8, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x7F, 0x80, 0xFE, 0xFF,
        ] {
            let mut bytes = vec![0x0E, offset, 0x3E, 0x5A, opcode];
            if immediate {
                bytes.push(offset);
            }
            bytes.extend_from_slice(&[0, 0xD3]);
            let cycles = if immediate { 3u64 } else { 2 };
            let read = match offset {
                0 => 0xCF,
                0x42..=0x43 | 0x45 | 0x4A..=0x4B | 0x80..=0xFF => 0,
                0x47 => 0xFC,
                _ => 0xFF,
            };
            let mut system = program(&bytes);
            complete(&mut system, 2);
            complete(&mut system, 2);
            let before = system.cpu_snapshot();
            let mut expected = before;
            expected.pc += if immediate { 2 } else { 1 };
            if !store {
                expected.af = u16::from(read) * 256 + (before.af & 0xF0);
            }
            for cycle in 0..cycles {
                let trace = system.step_machine_cycle().map(expect_cycle).unwrap();
                if cycle == 0 {
                    assert!(matches!(
                        trace.record.action,
                        CycleAction::Read { addr: 0x0104 }
                    ));
                    assert_eq!(trace.record.read_value, Some(opcode));
                } else if immediate && cycle == 1 {
                    assert!(matches!(
                        trace.record.action,
                        CycleAction::Read { addr: 0x0105 }
                    ));
                    assert_eq!(trace.record.read_value, Some(offset));
                } else if store {
                    assert!(
                        matches!(trace.record.action, CycleAction::Write { addr, value: 0x5A } if addr == 0xFF00 + u16::from(offset))
                    );
                    assert_eq!(trace.record.read_value, None);
                } else {
                    assert!(
                        matches!(trace.record.action, CycleAction::Read { addr } if addr == 0xFF00 + u16::from(offset))
                    );
                    assert_eq!(trace.record.read_value, Some(read));
                }
                assert_eq!(
                    (
                        trace.record.tick_before,
                        trace.record.tick_after,
                        instruction_complete(trace.outcome)
                    ),
                    (16 + cycle * 4, 20 + cycle * 4, cycle + 1 == cycles)
                );
                if !instruction_complete(trace.outcome) {
                    let mut pending = before;
                    pending.pc += cycle as u16 + 1;
                    assert_eq!(system.cpu_snapshot(), pending);
                }
            }
            assert_eq!(system.cpu_snapshot(), expected);
            for budget in [0, 1, 2, 3, 100] {
                let mut system = program(&bytes);
                complete(&mut system, 2);
                complete(&mut system, 2);
                let outcome = system.step_instruction(budget).unwrap();
                if budget < cycles {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                    let mut pending = before;
                    pending.pc += budget as u16;
                    assert_eq!(system.cpu_snapshot(), pending);
                    assert_eq!(system.elapsed_ticks(), 16 + budget * 4);
                    assert!(matches!(
                        system.step_instruction(0).unwrap(),
                        InstructionStep::BudgetExhausted {
                            mcycles: 0,
                            ticks: 0
                        }
                    ));
                    complete(&mut system, cycles - budget);
                } else {
                    assert!(
                        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
                    );
                }
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), 16 + cycles * 4);
                complete(&mut system, 1);
                let mut after = expected;
                after.pc += 1;
                assert_eq!(system.cpu_snapshot(), after);
            }
        }
    }
}
#[test]
fn ldh_c_and_immediate_forms_agree_with_absolute_addressing_for_every_offset() {
    for immediate in [false, true] {
        for offset in 0u8..=255 {
            for value in [0u8, 0x5A, 0xFF] {
                let mut bytes = vec![
                    0x0E,
                    offset,
                    0x3E,
                    value,
                    if immediate { 0xE0 } else { 0xE2 },
                ];
                if immediate {
                    bytes.push(offset);
                }
                bytes.extend_from_slice(&[
                    0xFA,
                    offset,
                    0xFF,
                    0x3E,
                    255 - value,
                    0xEA,
                    offset,
                    0xFF,
                    if immediate { 0xF0 } else { 0xF2 },
                ]);
                if immediate {
                    bytes.push(offset);
                }
                bytes.extend_from_slice(&[0, 0xD3]);
                let mut system = program(&bytes);
                complete(&mut system, 2);
                complete(&mut system, 2);
                complete(&mut system, if immediate { 3 } else { 2 });
                complete(&mut system, 4);
                let mapped =
                    matches!(offset, 5 | 6 | 0x42..=0x43 | 0x45 | 0x47..=0x4B | 0x80..=0xFF);
                assert_eq!(
                    system.cpu_snapshot().af,
                    u16::from(if offset == 0 {
                        0xCF | (value & 0x30)
                    } else if offset == 0x04 {
                        0 // A DIV write resets the counter instead of storing the byte.
                    } else if offset == 0x07 {
                        248 + value % 8
                    } else if offset == 0x0F {
                        value | 0xE0
                    } else if mapped {
                        value
                    } else {
                        0xFF
                    }) * 256
                        + 0xB0
                );
                complete(&mut system, 2);
                complete(&mut system, 4);
                complete(&mut system, if immediate { 3 } else { 2 });
                complete(&mut system, 1);
                assert_eq!(
                    system.cpu_snapshot().af,
                    u16::from(if offset == 0 {
                        0xCF | ((255 - value) & 0x30)
                    } else if offset == 0x04 {
                        0
                    } else if offset == 0x07 {
                        248 + (255 - value) % 8
                    } else if offset == 0x0F {
                        (255 - value) | 0xE0
                    } else if mapped {
                        255 - value
                    } else {
                        0xFF
                    }) * 256
                        + 0xB0
                );
                assert_eq!(system.cpu_snapshot().bc & 0xFF, u16::from(offset));
                assert_eq!(system.cpu_snapshot().pc, 0x0100 + bytes.len() as u16 - 1);
            }
        }
    }
}
