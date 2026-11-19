use super::super::{InstructionStep, MachineStep, System, SystemError};
use crate::{
    cpu::{CpuError, CycleAction, CycleOutcome},
    memory::{Button, Cartridge},
    step_support::expect_cycle,
};

fn machine(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn state_case(case: u8) -> System {
    match case {
        0 => machine(&[0]), // Fetch boundary, IME disabled.
        1 => {
            let mut s = machine(&[0xFB, 0]);
            s.step_instruction(1).unwrap(); // Delayed EI.
            s
        }
        2 => {
            let mut s = machine(&[0x3E, 0x12]);
            s.step_machine_cycle().unwrap(); // Pending immediate operand.
            s
        }
        3 => {
            let mut s = machine(&[0xFB, 0]);
            s.step_instruction(1).unwrap();
            s.step_instruction(1).unwrap();
            s.memory.write(0xFFFF, 1);
            s.memory.write(0xFF0F, 1);
            s.step_machine_cycle().unwrap(); // Partial interrupt entry.
            s
        }
        4 => {
            let mut s = machine(&[0x76, 0]);
            s.step_instruction(1).unwrap(); // HALTed, IME disabled.
            s
        }
        5 => {
            let mut s = machine(&[0xFB, 0x76, 0]);
            s.step_instruction(1).unwrap();
            s.step_instruction(1).unwrap(); // HALTed, IME enabled.
            s
        }
        6 => {
            let mut s = machine(&[0x10, 0]);
            s.step_instruction(1).unwrap(); // STOPped.
            s
        }
        7 => {
            let mut s = machine(&[0xD3]);
            assert!(s.step_machine_cycle().is_err()); // Sticky fault.
            s
        }
        8 => {
            let mut s = machine(&[0x76, 0x3E, 0x12]);
            s.memory.write(0xFFFF, 1);
            s.memory.write(0xFF0F, 1);
            s.step_instruction(1).unwrap(); // Armed HALT suppression latch.
            s
        }
        _ => unreachable!(),
    }
}

#[test]
fn joypad_system_input_preserves_cpu_and_time_across_execution_states() {
    for case in 0..9 {
        let mut s = state_case(case);
        let before = s.cpu_snapshot();
        let cpu_debug = format!("{:?}", s.cpu); // Includes phase, IME and suppression.
        let ticks = s.elapsed_ticks();
        let original: Vec<_> = (0u16..=65535).map(|addr| s.memory.read(addr)).collect();
        for (button, pressed, readback) in [
            (Button::A, true, 0xCE),
            (Button::Right, true, 0xCE),
            (Button::A, false, 0xCE),
            (Button::Right, false, 0xCF),
        ] {
            s.set_button(button, pressed);
            assert_eq!(s.cpu_snapshot(), before);
            assert_eq!(format!("{:?}", s.cpu), cpu_debug, "CPU state case {case}");
            assert_eq!(s.elapsed_ticks(), ticks);
            for addr in 0u16..=65535 {
                let expected = match addr {
                    0xFF00 => readback,
                    0xFF0F => original[0xFF0F] | 0x10,
                    _ => original[usize::from(addr)],
                };
                assert_eq!(
                    s.memory.read(addr),
                    expected,
                    "case {case}, address {addr:04X}"
                );
            }
        }
        if case == 6 {
            assert!(matches!(
                s.step_machine_cycle().unwrap(),
                MachineStep::Stopped
            ));
            assert_eq!(s.elapsed_ticks(), ticks);
        } else if case == 7 {
            assert!(matches!(
                s.step_machine_cycle(),
                Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
            ));
            assert_eq!(s.elapsed_ticks(), ticks);
        }
    }
}

#[test]
fn joypad_system_input_preserves_hidden_div_phase_and_instance_isolation() {
    let mut first = machine(&[0]);
    let mut second = machine(&[0]);
    for _ in 0..13 {
        first.step_instruction(1).unwrap();
    }
    for button in [
        Button::Right,
        Button::Left,
        Button::Up,
        Button::Down,
        Button::A,
        Button::B,
        Button::Select,
        Button::Start,
    ] {
        first.set_button(button, true);
        first.set_button(button, false);
    }
    first.set_button(Button::A, true);
    second.set_button(Button::Start, true);
    assert_eq!(
        (first.memory.read(0xFF00), second.memory.read(0xFF00)),
        (0xCE, 0xC7)
    );
    assert_eq!((first.elapsed_ticks(), second.elapsed_ticks()), (52, 0));
    assert_eq!(
        (first.memory.read(0xFF04), second.memory.read(0xFF04)),
        (0xAB, 0xAB)
    );
    first.step_instruction(1).unwrap();
    second.step_instruction(1).unwrap();
    assert_eq!((first.elapsed_ticks(), second.elapsed_ticks()), (56, 4));
    assert_eq!(
        (first.memory.read(0xFF04), second.memory.read(0xFF04)),
        (0xAC, 0xAB)
    );
}

#[test]
fn joypad_system_input_halt_wake_and_service_wait_for_the_next_cycle() {
    for ime in [false, true] {
        for enabled in [false, true] {
            for selected in [false, true] {
                let mut s = machine(if ime {
                    &[0xFB, 0x76, 0x04]
                } else {
                    &[0x76, 0x04]
                });
                s.memory.write(0xFF00, if selected { 0x10 } else { 0x20 });
                s.memory.write(0xFF0F, 0);
                s.memory.write(0xFFFF, if enabled { 0x10 } else { 0 });
                if ime {
                    s.step_instruction(1).unwrap();
                }
                s.step_instruction(1).unwrap();
                let before = s.cpu_snapshot();
                let cpu_debug = format!("{:?}", s.cpu);
                let ticks = s.elapsed_ticks();
                s.set_button(Button::A, true);
                assert_eq!(format!("{:?}", s.cpu), cpu_debug);
                assert_eq!(s.elapsed_ticks(), ticks);
                assert_eq!(s.memory.read(0xFF0F), if selected { 0xF0 } else { 0xE0 });
                s.step_instruction(0).unwrap();
                assert_eq!(s.cpu_snapshot(), before);
                assert_eq!(s.elapsed_ticks(), ticks);
                if !(selected && enabled) {
                    let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
                    assert_eq!(trace.record.action, CycleAction::Idle);
                    assert_eq!(trace.outcome, CycleOutcome::InProgress);
                    assert_eq!(s.cpu_snapshot(), before);
                    assert_eq!(s.elapsed_ticks(), ticks + 4);
                } else if ime {
                    assert!(matches!(
                        s.step_instruction(100).unwrap(),
                        InstructionStep::InterruptEntered {
                            vector: 0x60,
                            mcycles: 5,
                            ticks: 20
                        }
                    ));
                    assert_eq!(s.elapsed_ticks(), ticks + 20);
                    assert_eq!(s.memory.read(0xFF0F), 0xE0);
                    assert_eq!(
                        (s.memory.read(0xFFFC), s.memory.read(0xFFFD)),
                        (before.pc as u8, (before.pc / 256) as u8)
                    );
                    assert_eq!(s.cpu_snapshot().bc, before.bc);
                } else {
                    let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
                    assert_eq!(trace.record.action, CycleAction::Read { addr: before.pc });
                    assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
                    assert_eq!(s.cpu_snapshot().pc, before.pc + 1);
                    assert_eq!(s.cpu_snapshot().bc, before.bc + 0x100);
                    assert_eq!(s.cpu_snapshot().sp, before.sp);
                    assert_eq!(s.memory.read(0xFF0F), 0xF0);
                    assert_eq!(s.elapsed_ticks(), ticks + 4);
                }
            }
        }
    }
}

#[test]
fn joypad_system_selection_write_requests_at_the_bus_cycle_and_reads_back() {
    let mut s = machine(&[0x3E, 0x30, 0xE0, 0, 0x3E, 0x10, 0xE0, 0, 0xF0, 0]);
    s.step_instruction(2).unwrap();
    s.step_instruction(3).unwrap();
    s.memory.write(0xFF0F, 0);
    let before = s.cpu_snapshot();
    s.set_button(Button::A, true); // Stored while neither row is selected.
    assert_eq!(s.cpu_snapshot(), before);
    assert_eq!(s.elapsed_ticks(), 20);
    assert_eq!(s.memory.read(0xFF00), 0xFF);
    assert_eq!(s.memory.read(0xFF0F), 0xE0);
    s.step_instruction(2).unwrap();
    s.step_instruction(2).unwrap(); // Opcode and immediate, not the write yet.
    assert_eq!(s.elapsed_ticks(), 36);
    assert_eq!(s.memory.read(0xFF0F), 0xE0);
    s.step_instruction(0).unwrap();
    let trace = s.step_machine_cycle().map(expect_cycle).unwrap();
    assert_eq!(
        trace.record.action,
        CycleAction::Write {
            addr: 0xFF00,
            value: 0x10
        }
    );
    assert_eq!(
        (trace.record.tick_before, trace.record.tick_after),
        (36, 40)
    );
    assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
    assert_eq!((s.memory.read(0xFF00), s.memory.read(0xFF0F)), (0xDE, 0xF0));
    assert!(matches!(
        s.step_instruction(3).unwrap(),
        InstructionStep::Complete {
            mcycles: 3,
            ticks: 12
        }
    ));
    assert_eq!(s.cpu_snapshot().af / 256, 0xDE);
    assert_eq!(s.elapsed_ticks(), 52);
}
