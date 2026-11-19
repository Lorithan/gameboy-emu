use gameboy_emu::{
    cpu::{CycleAction, CycleOutcome},
    memory::{Button, Cartridge},
    system::{InstructionStep, MachineStep, System},
};

#[test]
fn public_input_pulse_wakes_stopped_execution_without_an_extra_cycle() {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x103].copy_from_slice(&[0x10, 0, 0x04]); // STOP; INC B
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    assert!(matches!(
        system.step_instruction(1).unwrap(),
        InstructionStep::Stopped {
            mcycles: 1,
            ticks: 4
        }
    ));
    let before = system.cpu_snapshot();
    assert_eq!(before.pc, 0x102);
    assert!(matches!(
        system.step_machine_cycle().unwrap(),
        MachineStep::Stopped
    ));
    system.set_button(Button::A, true);
    system.set_button(Button::A, false);
    assert_eq!(system.cpu_snapshot(), before);
    assert_eq!(system.elapsed_ticks(), 4);
    assert!(matches!(
        system.step_instruction(0).unwrap(),
        InstructionStep::BudgetExhausted {
            mcycles: 0,
            ticks: 0
        }
    ));
    let trace = match system.step_machine_cycle().unwrap() {
        MachineStep::Cycle(trace) => trace,
        other => panic!("selected input pulse failed to wake STOP: {other:?}"),
    };
    assert_eq!(trace.record.action, CycleAction::Read { addr: 0x102 });
    assert_eq!(trace.record.read_value, Some(0x04));
    assert_eq!((trace.record.tick_before, trace.record.tick_after), (4, 8));
    assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
    assert_eq!(system.cpu_snapshot().pc, 0x103);
    assert_eq!(system.cpu_snapshot().bc >> 8, (before.bc >> 8) + 1);
}

#[test]
fn public_joypad_input_changes_software_readback_without_executing_cycles() {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x108].copy_from_slice(&[0x3E, 0x10, 0xE0, 0, 0xF0, 0, 0xF0, 0]);
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    let before = system.cpu_snapshot();
    system.set_button(Button::A, true);
    assert_eq!(system.cpu_snapshot(), before);
    assert_eq!(system.elapsed_ticks(), 0);
    for cycles in [2, 3, 3] {
        assert!(
            matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4)
        );
    }
    assert_eq!(system.cpu_snapshot().af / 256, 0xDE);
    assert_eq!(system.elapsed_ticks(), 32);
    let before = system.cpu_snapshot();
    system.set_button(Button::A, false);
    assert_eq!(system.cpu_snapshot(), before);
    assert_eq!(system.elapsed_ticks(), 32);
    for cycle in 0..3 {
        let trace = match system.step_machine_cycle().unwrap() {
            MachineStep::Cycle(trace) => trace,
            other => panic!("expected JOYP read cycle, got {other:?}"),
        };
        assert_eq!(
            (trace.record.tick_before, trace.record.tick_after),
            (32 + cycle * 4, 36 + cycle * 4)
        );
        if cycle == 2 {
            assert_eq!(trace.record.action, CycleAction::Read { addr: 0xFF00 });
            assert_eq!(trace.record.read_value, Some(0xDF));
            assert_eq!(trace.outcome, CycleOutcome::InstructionComplete);
        }
    }
    assert_eq!(system.cpu_snapshot().af / 256, 0xDF);
    assert_eq!(system.elapsed_ticks(), 44);
}
