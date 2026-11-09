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
fn word_add_has_read_idle_traces_and_resumes_every_budget() {
    for (opcode, load) in [(0x09, 0x01), (0x19, 0x11), (0x29, 0x21), (0x39, 0x31)] {
        for budget in [0, 1, 2, 100] {
            let self_source = opcode == 0x29;
            let mut bytes = vec![0x21, 0xFF, 0x0F];
            if !self_source {
                bytes.extend_from_slice(&[load, 1, 0]);
            }
            bytes.extend_from_slice(&[opcode, 0, 0xD3]);
            let mut system = program(&bytes);
            complete(&mut system, 3);
            if !self_source {
                complete(&mut system, 3);
            }
            let before = system.cpu_snapshot();
            let ticks = system.elapsed_ticks();
            let mut expected = before;
            expected.hl = if self_source { 0x1FFE } else { 0x1000 };
            expected.af = 0x01A0;
            expected.pc += 1;
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
            assert_eq!(system.elapsed_ticks(), ticks + 8);
            complete(&mut system, 1);
            expected.pc += 1;
            assert_eq!(system.cpu_snapshot(), expected);
            let mut system = program(&bytes);
            complete(&mut system, 3);
            if !self_source {
                complete(&mut system, 3);
            }
            let fetch = system.step_machine_cycle().unwrap();
            assert!(matches!(fetch.action, CycleAction::Read { addr } if addr == before.pc));
            assert_eq!(fetch.read_value, Some(opcode));
            assert_eq!(
                (fetch.tick_before, fetch.tick_after, fetch.complete),
                (ticks, ticks + 4, false)
            );
            let mut pending = before;
            pending.pc += 1;
            assert_eq!(system.cpu_snapshot(), pending);
            let idle = system.step_machine_cycle().unwrap();
            assert!(matches!(idle.action, CycleAction::Idle));
            assert_eq!(idle.read_value, None);
            assert_eq!(
                (idle.tick_before, idle.tick_after, idle.complete),
                (ticks + 4, ticks + 8, true)
            );
            expected.pc -= 1;
            assert_eq!(system.cpu_snapshot(), expected);
        }
    }
}
#[test]
fn word_add_result_is_a_pointer_and_preserved_z_and_c_drive_branches() {
    let bytes = [
        0xAF, // Z=1
        0x21, 0xFF, 0xBF, 0x01, 1, 0,    // HL BFFF BC 1
        0x09, // HL C000, Z remains set despite nonzero result
        0x36, 0x42, 0x7E, // store and load at C000
        0x28, 1, 0xD3, // JR Z taken
        0x21, 0xFF, 0xFF, 0x09, // HL 0000 C=1
        0x38, 1, 0xD3, // JR C taken
        0x00, 0xD3,
    ];
    let mut system = program(&bytes);
    for cycles in [1, 3, 3, 2, 3, 2, 3, 3, 2, 3, 1] {
        complete(&mut system, cycles);
    }
    let snapshot = system.cpu_snapshot();
    assert_eq!((snapshot.af, snapshot.hl, snapshot.bc), (0x42B0, 0, 1));
    assert_eq!(snapshot.pc, 0x0100 + bytes.len() as u16 - 1);
    assert_eq!(system.elapsed_ticks(), 104);
}
