use gameboy_emu::{
    cpu::{CpuError, CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System, SystemError},
};

#[derive(Debug, Clone, Copy)]
enum Destination {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
}

// Explicit reference mapping, independent of the production decoder.
const CASES: [(u8, Destination); 7] = [
    (0x06, Destination::B),
    (0x0E, Destination::C),
    (0x16, Destination::D),
    (0x1E, Destination::E),
    (0x26, Destination::H),
    (0x2E, Destination::L),
    (0x3E, Destination::A),
];

fn machine(opcode: u8, value: u8) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0103].copy_from_slice(&[opcode, value, 0xD3]);
    System::new(Cartridge::from_bytes(&rom).expect("valid synthetic program"))
}

fn expected_load(mut snapshot: CpuSnapShot, destination: Destination, byte: u8) -> CpuSnapShot {
    let value = u16::from(byte);
    match destination {
        Destination::B => snapshot.bc = (value << 8) | (snapshot.bc & 0xFF),
        Destination::C => snapshot.bc = (snapshot.bc & 0xFF00) | value,
        Destination::D => snapshot.de = (value << 8) | (snapshot.de & 0xFF),
        Destination::E => snapshot.de = (snapshot.de & 0xFF00) | value,
        Destination::H => snapshot.hl = (value << 8) | (snapshot.hl & 0xFF),
        Destination::L => snapshot.hl = (snapshot.hl & 0xFF00) | value,
        Destination::A => snapshot.af = (value << 8) | (snapshot.af & 0xFF),
    }
    snapshot.pc = 0x0102;
    snapshot
}

#[test]
fn every_register_loads_every_byte_with_correct_per_cycle_state_and_trace() {
    for (opcode, destination) in CASES {
        for value in 0u8..=u8::MAX {
            let mut system = machine(opcode, value);
            let initial = system.cpu_snapshot();
            let first = system.step_machine_cycle().expect("opcode cycle");
            assert_eq!((first.tick_before, first.tick_after), (0, 4));
            assert!(matches!(first.action, CycleAction::Read { addr: 0x0100 }));
            assert_eq!(first.read_value, Some(opcode));
            assert!(!first.complete);
            let mut pending = initial;
            pending.pc = 0x0101;
            assert_eq!(system.cpu_snapshot(), pending);

            let second = system.step_machine_cycle().expect("operand cycle");
            assert_eq!((second.tick_before, second.tick_after), (4, 8));
            assert!(matches!(second.action, CycleAction::Read { addr: 0x0101 }));
            assert_eq!(second.read_value, Some(value));
            assert!(second.complete);
            assert_eq!(system.elapsed_ticks(), 8);
            assert_eq!(
                system.cpu_snapshot(),
                expected_load(initial, destination, value),
                "opcode {opcode:#04X}, value {value:#04X}, destination {destination:?}"
            );
        }
    }
}

#[test]
fn all_destinations_survive_budget_exhaustion_and_resume() {
    for (opcode, destination) in CASES {
        let mut system = machine(opcode, 0x80);
        let initial = system.cpu_snapshot();
        assert!(matches!(
            system.step_instruction(1),
            Ok(InstructionStep::BudgetExhausted {
                mcycles: 1,
                ticks: 4
            })
        ));
        let mut pending = initial;
        pending.pc = 0x0101;
        assert_eq!(system.cpu_snapshot(), pending);
        assert!(matches!(
            system.step_instruction(1),
            Ok(InstructionStep::Complete {
                mcycles: 1,
                ticks: 4
            })
        ));
        assert_eq!(
            system.cpu_snapshot(),
            expected_load(initial, destination, 0x80)
        );
        assert_eq!(system.elapsed_ticks(), 8);
    }
}

#[test]
fn all_register_loads_stop_at_their_operand_when_budget_is_larger() {
    for (opcode, destination) in CASES {
        let mut system = machine(opcode, 0x42);
        let initial = system.cpu_snapshot();
        assert!(matches!(
            system.step_instruction(100),
            Ok(InstructionStep::Complete {
                mcycles: 2,
                ticks: 8
            })
        ));
        assert_eq!(
            system.cpu_snapshot(),
            expected_load(initial, destination, 0x42)
        );
        assert_eq!(system.elapsed_ticks(), 8);
    }
}

#[test]
fn unrelated_opcodes_remain_unsupported() {
    for opcode in [0xD3, 0xDB] {
        let mut system = machine(opcode, 0x42);
        assert!(matches!(
            system.step_machine_cycle(),
            Err(SystemError::CpuCompletionError(
                CpuError::UnsupportedOpcode(_),
                _
            ))
        ));
        assert_eq!(system.cpu_snapshot().pc, 0x0101);
        assert_eq!(system.elapsed_ticks(), 4);
        assert!(matches!(
            system.step_machine_cycle(),
            Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
        ));
        assert_eq!(system.elapsed_ticks(), 4);
    }
}
