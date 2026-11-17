use super::super::{InstructionStep, System, SystemError};
use crate::{
    cpu::{CpuError, CycleAction, CycleOutcome},
    memory::{Cartridge, Interrupt},
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn ready(pc: u16, sp: u16) -> System {
    let mut s = machine(&[
        0x31,
        sp as u8,
        (sp / 256) as u8,
        0xFB,
        0,
        0xC3,
        pc as u8,
        (pc / 256) as u8,
    ]);
    for _ in 0..4 {
        s.step_instruction(100).unwrap();
    }
    assert_eq!(s.cpu_snapshot().pc, pc);
    assert_eq!(s.cpu.test_interrupt_status(), (false, true));
    s
}

// Independent model of externally observable writes, including IF read masks.
fn write_expected(memory: &mut [u8], addr: u16, value: u8) {
    match addr {
        0xC000..=0xDFFF => {
            memory[usize::from(addr)] = value;
            if addr <= 0xDDFF {
                memory[usize::from(addr + 0x2000)] = value;
            }
        }
        0xE000..=0xFDFF => {
            memory[usize::from(addr)] = value;
            memory[usize::from(addr - 0x2000)] = value;
        }
        0xFF0F => memory[usize::from(addr)] = value % 32 + 224,
        0xFF80..=0xFFFF => memory[usize::from(addr)] = value,
        _ => {}
    }
}

#[test]
fn interrupt_entry_exact_traces_budgets_and_full_memory_effects() {
    for (mask, vector) in [(1, 0x40), (2, 0x48), (4, 0x50), (8, 0x58), (16, 0x60)] {
        for sp in [
            0u16, 1, 0x8001, 0xC002, 0xE002, 0xFF10, 0xFF11, 0xFF81, 0xFFFE, 0xFFFF,
        ] {
            for budget in [0, 1, 2, 3, 4, 5, 100] {
                let pc = 0xA53C;
                let mut s = ready(pc, sp);
                s.memory.write(0xFF0F, 31);
                s.memory.write(0xFFFF, mask);
                let original: Vec<_> = (0u16..=65535).map(|a| s.memory.read(a)).collect();
                let before = s.cpu_snapshot();
                let start = s.elapsed_ticks();
                let result = s.step_instruction(budget).unwrap();
                let completed = budget.min(5);
                if budget < 5 {
                    assert!(
                        matches!(result, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                } else {
                    assert!(
                        matches!(result, InstructionStep::InterruptEntered { vector: v, mcycles: 5, ticks: 20 } if v == vector)
                    );
                }
                for phase in completed..=5 {
                    let mut expected_memory = original.clone();
                    if phase > 0 {
                        expected_memory[0xFF0F] = (31 - mask) + 224;
                    }
                    if phase >= 3 {
                        write_expected(&mut expected_memory, sp.wrapping_sub(1), (pc / 256) as u8);
                    }
                    if phase >= 4 {
                        write_expected(&mut expected_memory, sp.wrapping_sub(2), pc as u8);
                    }
                    for addr in 0u16..=65535 {
                        assert_eq!(
                            s.memory.read(addr),
                            expected_memory[usize::from(addr)],
                            "SP {sp:04X} budget {budget} phase {phase} address {addr:04X}"
                        );
                    }
                    let mut expected = before;
                    if phase >= 3 {
                        expected.sp = sp.wrapping_sub(if phase == 3 { 1 } else { 2 });
                    }
                    if phase == 5 {
                        expected.pc = vector;
                    }
                    assert_eq!(s.cpu_snapshot(), expected);
                    assert_eq!(s.elapsed_ticks(), start + phase * 4);
                    assert_eq!(s.cpu.test_interrupt_status(), (false, phase == 0));
                    s.step_instruction(0).unwrap();
                    assert_eq!(s.cpu_snapshot(), expected);
                    assert_eq!(s.elapsed_ticks(), start + phase * 4);
                    assert_eq!(s.memory.read(0xFF0F), expected_memory[0xFF0F]);
                    if phase == 5 {
                        break;
                    }
                    let trace = s.step_machine_cycle().unwrap();
                    let action = match phase {
                        2 => CycleAction::Write {
                            addr: sp.wrapping_sub(1),
                            value: (pc / 256) as u8,
                        },
                        3 => CycleAction::Write {
                            addr: sp.wrapping_sub(2),
                            value: pc as u8,
                        },
                        _ => CycleAction::Idle,
                    };
                    assert_eq!(trace.record.action, action);
                    assert_eq!(trace.record.read_value, None);
                    assert_eq!(
                        (trace.record.tick_before, trace.record.tick_after),
                        (start + phase * 4, start + (phase + 1) * 4)
                    );
                    assert_eq!(
                        trace.outcome,
                        if phase == 4 {
                            CycleOutcome::InterruptEntryComplete { vector }
                        } else {
                            CycleOutcome::InProgress
                        }
                    );
                }
                let trace = s.step_machine_cycle().unwrap();
                assert_eq!(trace.record.action, CycleAction::Read { addr: vector });
                assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
                assert_eq!(s.cpu_snapshot().pc, vector + 1);
            }
        }
    }
}

#[test]
fn interrupt_entry_resume_counts_and_acknowledgment_happen_once() {
    for pause in 1..5 {
        let mut s = ready(0x1234, 0xFFFE);
        s.memory.write(0xFF0F, 4);
        s.memory.write(0xFFFF, 4);
        s.step_instruction(pause).unwrap();
        assert_eq!(s.memory.read(0xFF0F), 0xE0);
        // Model-specific: capture/ack at acceptance means later requests survive.
        s.memory.request_interrupt(Interrupt::Timer);
        s.memory.request_interrupt(Interrupt::VBlank);
        s.memory.write(0xFFFF, 0);
        assert!(
            matches!(s.step_instruction(100).unwrap(), InstructionStep::InterruptEntered { vector: 0x50, mcycles, ticks } if mcycles == 5 - pause && ticks == (5 - pause) * 4)
        );
        assert_eq!(s.memory.read(0xFF0F), 0xE5);
        assert_eq!(s.memory.read(0xFFFF), 0);
        assert_eq!(s.cpu.test_interrupt_status(), (false, false));
    }
}

#[test]
fn system_interrupt_priority_uses_only_enabled_requested_low_bits() {
    for flags in 0u8..32 {
        for enable in 0u8..32 {
            let mut s = ready(0x1234, 0xFFFE);
            s.memory.write(0xFF0F, flags);
            s.memory.write(0xFFFF, enable | 0xE0);
            let pending = flags & enable;
            if pending == 0 {
                assert!(matches!(
                    s.step_instruction(1).unwrap(),
                    InstructionStep::Complete {
                        mcycles: 1,
                        ticks: 4
                    }
                ));
                assert_eq!(s.cpu_snapshot().pc, 0x1235);
                assert_eq!(s.memory.read(0xFF0F), flags + 224);
            } else {
                let bit = (0..5).find(|i| pending & (1 << i) != 0).unwrap();
                assert!(
                    matches!(s.step_instruction(100).unwrap(), InstructionStep::InterruptEntered { vector, mcycles: 5, ticks: 20 } if vector == 0x40 + bit * 8)
                );
                assert_eq!(s.memory.read(0xFF0F), (flags - (1 << bit)) + 224);
            }
            assert_eq!(s.memory.read(0xFFFF), enable | 0xE0);
        }
    }
}

#[test]
fn pending_interrupts_wait_for_ei_follower_and_instruction_completion() {
    for follower in [
        &[0x3E, 0x42][..],
        &[0xCB, 0x00][..],
        &[0xCB, 0x46][..],
        &[0xCB, 0x06][..],
        &[0xFB][..],
    ] {
        let mut bytes = vec![0x21, 0, 0xC0, 0xFB];
        bytes.extend_from_slice(follower);
        let mut s = machine(&bytes);
        s.step_instruction(3).unwrap();
        s.memory.write(0xFF0F, 4);
        s.memory.write(0xFFFF, 4);
        s.step_instruction(1).unwrap(); // EI
        let cycles = match follower {
            [0xCB, 0x46] => 3,
            [0xCB, 0x06] => 4,
            [0xFB] => 1,
            _ => 2,
        };
        for phase in 0..cycles {
            let trace = s.step_machine_cycle().unwrap();
            assert_eq!(
                trace.outcome,
                if phase + 1 == cycles {
                    CycleOutcome::InstructionComplete
                } else {
                    CycleOutcome::InProgress
                }
            );
            assert_eq!(s.memory.read(0xFF0F), 0xE4);
        }
        let return_pc = s.cpu_snapshot().pc;
        assert!(matches!(
            s.step_instruction(5).unwrap(),
            InstructionStep::InterruptEntered { vector: 0x50, .. }
        ));
        assert_eq!(s.memory.read(0xFFFC), return_pc as u8);
        assert_eq!(s.memory.read(0xFFFD), (return_pc / 256) as u8);
    }
    let mut s = machine(&[0xFB, 0xF3, 0]);
    s.memory.write(0xFF0F, 31);
    s.memory.write(0xFFFF, 31);
    for _ in 0..3 {
        assert!(matches!(
            s.step_instruction(1).unwrap(),
            InstructionStep::Complete { .. }
        ));
    }
    assert_eq!(s.memory.read(0xFF0F), 0xFF);
}

#[test]
fn interrupts_do_not_clear_sticky_faults_or_mutate_on_zero_budget() {
    let mut s = machine(&[0xD3]);
    assert!(matches!(
        s.step_instruction(1),
        Err(SystemError::CpuCompletionError(
            CpuError::UnsupportedOpcode(_),
            _
        ))
    ));
    s.memory.write(0xFF0F, 31);
    s.memory.write(0xFFFF, 31);
    let before = s.cpu_snapshot();
    for _ in 0..2 {
        s.step_instruction(0).unwrap();
        assert!(matches!(
            s.step_machine_cycle(),
            Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
        ));
        assert_eq!(s.cpu_snapshot(), before);
        assert_eq!(s.elapsed_ticks(), 4);
        assert_eq!(s.memory.read(0xFF0F), 0xFF);
    }
}

#[test]
fn requests_arriving_mid_instruction_wait_even_when_ime_is_already_enabled() {
    for follower in [&[0x3E, 0x42][..], &[0xCB, 0x46][..], &[0xCB, 0x06][..]] {
        let mut bytes = vec![0x21, 0, 0xC0, 0xFB, 0];
        bytes.extend_from_slice(follower);
        let cycles = if follower == [0xCB, 0x06] {
            4
        } else if follower == [0xCB, 0x46] {
            3
        } else {
            2
        };
        for pause in 1..cycles {
            let mut s = machine(&bytes);
            for _ in 0..3 {
                s.step_instruction(100).unwrap();
            }
            assert_eq!(s.cpu.test_interrupt_status(), (false, true));
            s.step_instruction(pause).unwrap();
            s.memory.write(0xFF0F, 4);
            s.memory.write(0xFFFF, 4);
            assert!(
                matches!(s.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, .. } if mcycles == cycles - pause)
            );
            assert_eq!(s.memory.read(0xFF0F), 0xE4);
            assert!(matches!(
                s.step_instruction(100).unwrap(),
                InstructionStep::InterruptEntered {
                    vector: 0x50,
                    mcycles: 5,
                    ticks: 20
                }
            ));
        }
    }
}
