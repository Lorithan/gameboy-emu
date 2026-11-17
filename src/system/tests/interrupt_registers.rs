use crate::outcome_support::instruction_complete;

use super::super::{InstructionStep, System};
use crate::{cpu::CycleAction, memory::Cartridge};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

#[test]
fn interrupt_register_transfers_have_exact_cycles_and_resumable_writes() {
    for addr in [0xFF0Fu16, 0xFFFF] {
        for (store, opcode, cycles) in [
            (true, 0xEA, 4),
            (false, 0xFA, 4),
            (true, 0xE0, 3),
            (false, 0xF0, 3),
            (true, 0xE2, 2),
            (false, 0xF2, 2),
        ] {
            for value in [0, 1, 0x1F, 0x80, 0xA5, 0xFF] {
                let mut bytes = vec![0x3E, value, 0x0E, addr as u8, opcode];
                if cycles >= 3 {
                    bytes.push(addr as u8);
                }
                if cycles == 4 {
                    bytes.push(0xFF);
                }
                bytes.push(0);
                for budget in 0..=cycles {
                    let mut s = machine(&bytes);
                    s.step_instruction(2).unwrap();
                    s.step_instruction(2).unwrap();
                    s.memory.write(addr, 0x12);
                    let before = s.cpu_snapshot();
                    let outcome = s.step_instruction(budget).unwrap();
                    assert_eq!(
                        matches!(outcome, InstructionStep::Complete { .. }),
                        budget == cycles
                    );
                    assert_eq!(s.elapsed_ticks(), 16 + budget * 4);
                    let readback = if addr == 0xFF0F {
                        value % 32 + 224
                    } else {
                        value
                    };
                    assert_eq!(
                        s.memory.read(addr),
                        if store && budget == cycles {
                            readback
                        } else if addr == 0xFF0F {
                            0xF2
                        } else {
                            0x12
                        }
                    );
                    let paused = s.cpu_snapshot();
                    s.step_instruction(0).unwrap();
                    assert_eq!(s.cpu_snapshot(), paused);
                    assert_eq!(s.elapsed_ticks(), 16 + budget * 4);
                    for cycle in budget..cycles {
                        let t = s.step_machine_cycle().unwrap();
                        assert_eq!(
                            (t.record.tick_before, t.record.tick_after),
                            (16 + cycle * 4, 20 + cycle * 4)
                        );
                        assert_eq!(instruction_complete(t.outcome), cycle + 1 == cycles);
                        if cycle + 1 == cycles {
                            if store {
                                assert!(
                                    matches!(t.record.action, CycleAction::Write { addr: a, value: v } if a == addr && v == value)
                                );
                            } else {
                                assert!(
                                    matches!(t.record.action, CycleAction::Read { addr: a } if a == addr)
                                );
                                assert_eq!(
                                    t.record.read_value,
                                    Some(if addr == 0xFF0F { 0xF2 } else { 0x12 })
                                );
                            }
                        } else {
                            assert!(
                                matches!(t.record.action, CycleAction::Read { addr: a } if a == before.pc + cycle as u16)
                            );
                        }
                    }
                    let mut expected = before;
                    expected.pc += (cycles - 1) as u16;
                    if !store {
                        expected.af = if addr == 0xFF0F { 0xF2B0 } else { 0x12B0 };
                    }
                    assert_eq!(s.cpu_snapshot(), expected);
                    assert_eq!(s.cpu.test_interrupt_status(), (false, false));
                    assert_eq!(
                        s.memory.read(addr),
                        if store {
                            readback
                        } else if addr == 0xFF0F {
                            0xF2
                        } else {
                            0x12
                        }
                    );
                    s.step_instruction(1).unwrap();
                    assert_eq!(s.cpu_snapshot().pc, expected.pc + 1);
                }
            }
        }
    }
}

#[test]
fn enabled_ime_and_pending_request_do_not_dispatch_before_entry_is_implemented() {
    let mut s = machine(&[0xFB, 0, 0, 0]);
    s.memory.write(0xFF0F, 0x1F);
    s.memory.write(0xFFFF, 0xFF);
    for _ in 0..4 {
        s.step_instruction(1).unwrap();
    }
    assert_eq!(s.cpu.test_interrupt_status(), (false, true));
    assert_eq!(s.cpu_snapshot().pc, 0x104);
    assert_eq!(s.cpu_snapshot().sp, 0xFFFE);
    assert_eq!(s.memory.pending_interrupts(), 0x1F);
    assert_eq!(s.elapsed_ticks(), 16);
}
