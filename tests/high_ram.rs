use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(bytes: &[u8]) -> System {
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
fn cpu_stores_and_loads_every_high_ram_address_with_exact_traces() {
    for address in 0xFF80u16..=0xFFFE {
        for value in [0u8, 0x5A, 0xFF] {
            let mut system = machine(&[0x21, address as u8, 0xFF, 0x36, value, 0x7E, 0xD3]);
            complete(&mut system, 3);
            let mut expected = system.cpu_snapshot();
            for cycle in 0u64..3 {
                let trace = system.step_machine_cycle().unwrap();
                assert_eq!(
                    (trace.tick_before, trace.tick_after),
                    (12 + cycle * 4, 16 + cycle * 4)
                );
                assert_eq!(trace.complete, cycle == 2);
                match cycle {
                    0 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0103 }));
                        assert_eq!(trace.read_value, Some(0x36));
                        expected.pc = 0x0104;
                    }
                    1 => {
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0104 }));
                        assert_eq!(trace.read_value, Some(value));
                        expected.pc = 0x0105;
                    }
                    _ => {
                        assert!(
                            matches!(trace.action, CycleAction::Write { addr, value: byte } if addr == address && byte == value)
                        );
                        assert_eq!(trace.read_value, None);
                    }
                }
                assert_eq!(system.cpu_snapshot(), expected);
            }
            let opcode = system.step_machine_cycle().unwrap();
            assert!(matches!(opcode.action, CycleAction::Read { addr: 0x0105 }));
            assert_eq!(opcode.read_value, Some(0x7E));
            assert!(!opcode.complete);
            let data = system.step_machine_cycle().unwrap();
            assert!(matches!(data.action, CycleAction::Read { addr } if addr == address));
            assert_eq!(data.read_value, Some(value));
            assert!(data.complete);
            expected.pc = 0x0106;
            expected.af = (u16::from(value) << 8) | (expected.af & 0xFF);
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 32);
        }
    }
}

#[test]
fn cpu_writes_an_instruction_then_fetches_and_executes_it_in_high_ram() {
    for address in [0xFF80u16, 0xFFC0, 0xFFFE] {
        let mut system = machine(&[0x21, address as u8, 0xFF, 0x36, 0x04, 0xE9, 0xD3]);
        complete(&mut system, 3); // LD HL,address
        complete(&mut system, 3); // LD (HL),INC B opcode
        complete(&mut system, 1); // JP HL
        let mut expected = system.cpu_snapshot();
        assert_eq!(expected.pc, address);
        let trace = system.step_machine_cycle().unwrap();
        assert!(matches!(trace.action, CycleAction::Read { addr } if addr == address));
        assert_eq!(trace.read_value, Some(0x04));
        assert_eq!(
            (trace.tick_before, trace.tick_after, trace.complete),
            (28, 32, true)
        );
        expected.bc = 0x0113;
        expected.af = (expected.af & 0xFF00) | 0x10;
        expected.pc = address + 1;
        assert_eq!(system.cpu_snapshot(), expected);
    }
}
