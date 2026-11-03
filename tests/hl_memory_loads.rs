use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(tail: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    // HL=C000, A=42, followed by the instruction under test.
    let setup = [0x21, 0x00, 0xC0, 0x3E, 0x42];
    rom[0x0100..0x0105].copy_from_slice(&setup);
    rom[0x0105..0x0105 + tail.len()].copy_from_slice(tail);
    let mut system = System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"));
    for cycles in [3, 2] {
        assert!(matches!(
            system.step_instruction(cycles),
            Ok(InstructionStep::Complete { .. })
        ));
    }
    system
}

#[test]
fn register_store_returns_to_fetch_instead_of_repeating_the_write() {
    let mut system = machine(&[0x77, 0x00, 0xD3]);
    assert!(matches!(
        system.step_instruction(2),
        Ok(InstructionStep::Complete {
            mcycles: 2,
            ticks: 8
        })
    ));
    let next = system.step_machine_cycle().expect("next opcode");
    assert!(matches!(next.action, CycleAction::Read { addr: 0x0106 }));
    assert_eq!(next.read_value, Some(0x00));
    assert!(next.complete);
    assert_eq!(system.cpu_snapshot().pc, 0x0107);
}

#[test]
fn data_read_preserves_pc_and_only_updates_the_destination() {
    let mut system = machine(&[0x7E, 0x00, 0xD3]);
    let mut expected = system.cpu_snapshot();
    expected.pc = 0x0106;
    expected.af &= 0x00FF; // WRAM starts at zero in the current bus model.
    let fetch = system.step_machine_cycle().expect("opcode");
    assert!(!fetch.complete);
    let read = system.step_machine_cycle().expect("WRAM read");
    assert!(matches!(read.action, CycleAction::Read { addr: 0xC000 }));
    assert_eq!(read.read_value, Some(0));
    assert!(read.complete);
    assert_eq!(system.cpu_snapshot(), expected);
}

#[test]
fn immediate_store_fetches_an_operand_then_writes_and_resumes_fetching() {
    let mut system = machine(&[0x36, 0x00, 0x7E, 0xD3]);
    let opcode = system.step_machine_cycle().expect("opcode");
    assert!(!opcode.complete);
    let operand = system.step_machine_cycle().expect("immediate operand");
    assert!(matches!(operand.action, CycleAction::Read { addr: 0x0106 }));
    assert_eq!(operand.read_value, Some(0));
    assert!(!operand.complete, "the write cycle is still pending");
    let write = system.step_machine_cycle().expect("immediate write");
    assert!(matches!(
        write.action,
        CycleAction::Write {
            addr: 0xC000,
            value: 0
        }
    ));
    assert_eq!(write.read_value, None);
    assert!(write.complete);
    assert_eq!(system.cpu_snapshot().pc, 0x0107);
    let next = system.step_machine_cycle().expect("next opcode");
    assert!(matches!(next.action, CycleAction::Read { addr: 0x0107 }));
    assert_eq!(next.read_value, Some(0x7E));
}

#[test]
fn every_byte_round_trips_through_both_store_forms_with_exact_cycle_traces() {
    for opcode in [0x77, 0x36] {
        for value in 0u8..=255 {
            let mut tail = vec![0x3E, value, opcode];
            if opcode == 0x36 {
                tail.push(value);
            }
            // Overwrite A so the later load must retrieve data from RAM.
            tail.extend_from_slice(&[0x3E, !value, 0x7E, 0xD3]);
            let mut system = machine(&tail);
            system.step_instruction(2).expect("set source value");
            let mut expected = system.cpu_snapshot();
            let mut expected_tick = system.elapsed_ticks();
            let mut cycles = vec![(expected.pc, Some(opcode), false, false)];
            if opcode == 0x36 {
                cycles.push((expected.pc + 1, Some(value), false, false));
            }
            cycles.push((0xC000, None, true, true));
            for (address, read_value, write, complete) in cycles {
                let trace = system.step_machine_cycle().expect("store cycle");
                assert_eq!(
                    (trace.tick_before, trace.tick_after),
                    (expected_tick, expected_tick + 4)
                );
                expected_tick += 4;
                assert_eq!(trace.read_value, read_value);
                assert_eq!(trace.complete, complete);
                if write {
                    assert!(
                        matches!(trace.action, CycleAction::Write { addr, value: data } if addr == address && data == value)
                    );
                } else {
                    assert!(matches!(trace.action, CycleAction::Read { addr } if addr == address));
                    expected.pc += 1;
                }
                assert_eq!(system.cpu_snapshot(), expected);
            }
            system.step_instruction(2).expect("overwrite A");
            expected.pc += 2;
            expected.af = u16::from(!value) << 8 | (expected.af & 0xFF);
            assert_eq!(system.cpu_snapshot(), expected);
            expected_tick += 8;
            for (address, byte, complete) in [(expected.pc, 0x7E, false), (0xC000, value, true)] {
                let trace = system.step_machine_cycle().expect("load cycle");
                assert!(matches!(trace.action, CycleAction::Read { addr } if addr == address));
                assert_eq!(trace.read_value, Some(byte));
                assert_eq!(trace.complete, complete);
                assert_eq!(
                    (trace.tick_before, trace.tick_after),
                    (expected_tick, expected_tick + 4)
                );
                expected_tick += 4;
                if complete {
                    expected.af = u16::from(value) << 8 | (expected.af & 0xFF);
                } else {
                    expected.pc += 1;
                }
                assert_eq!(system.cpu_snapshot(), expected);
            }
            assert_eq!(system.elapsed_ticks(), expected_tick);
        }
    }
}
