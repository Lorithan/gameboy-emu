use super::System;
use crate::{cpu::CpuSnapShot, memory::Cartridge};

fn assert_post_boot_registers(system: &System, expected_af: u16) {
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: expected_af,
            bc: 0x0013,
            de: 0x00D8,
            hl: 0x014D,
            pc: 0x0100,
            sp: 0xFFFE,
        }
    );
}

fn construct_system(title_byte: u8, checksum: u8) -> (System, Vec<u8>) {
    let mut rom = vec![0; 32_768];
    rom[0x0134] = title_byte;
    rom[0x014D] = checksum;
    rom[0] = 0x35;
    rom[0x7FFF] = 0x76;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic cartridge");
    (System::new(cartridge), rom)
}

#[test]
fn system_uses_zero_checksum_boot_flags() {
    // -(0xE7 + 25) modulo 256 is zero.
    let (system, _) = construct_system(0xE7, 0);
    assert_post_boot_registers(&system, 0x0180);
}

#[test]
fn system_uses_nonzero_checksum_boot_flags() {
    // Independent known checksums for two synthetic headers.
    for (title_byte, checksum) in [(0, 0xE7), (0xE8, 0xFF)] {
        let (system, _) = construct_system(title_byte, checksum);
        assert_post_boot_registers(&system, 0x01B0);
    }
}

#[test]
fn system_moves_the_validated_cartridge_into_its_bus_unchanged() {
    let (system, rom) = construct_system(0, 0xE7);
    for address in 0u16..=0x7FFF {
        assert_eq!(system.memory.read(address), rom[usize::from(address)]);
    }
}

#[test]
fn systems_have_independent_zero_initialized_work_ram() {
    let (mut first, _) = construct_system(0, 0xE7);
    let (second, _) = construct_system(0xE7, 0);
    for address in 0xC000u16..=0xFDFF {
        assert_eq!(first.memory.read(address), 0);
        assert_eq!(second.memory.read(address), 0);
    }
    first.memory.write(0xC123, 0xA5);
    assert_eq!(first.memory.read(0xE123), 0xA5);
    assert_eq!(second.memory.read(0xC123), 0);
    assert_eq!(second.memory.read(0xE123), 0);
    assert_post_boot_registers(&first, 0x01B0);
    assert_post_boot_registers(&second, 0x0180);
}

fn program_system(program: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + program.len()].copy_from_slice(program);
    System::new(Cartridge::from_bytes(&rom).expect("valid synthetic program"))
}

#[test]
fn clock_starts_at_zero_and_inspection_does_not_advance_time() {
    let system = program_system(&[0]);
    let original = system.cpu_snapshot();
    for _ in 0..3 {
        assert_eq!(system.elapsed_ticks(), 0);
        assert_eq!(system.cpu_snapshot(), original);
    }
}

#[test]
fn immediate_values_are_data_and_preserve_flags() {
    for immediate in [0, 0x3E, 0x80, 0xFF] {
        let mut system = program_system(&[0x3E, immediate]);
        let initial = system.cpu_snapshot();
        assert!(
            !system
                .step_machine_cycle()
                .expect("LD opcode cycle")
                .complete
        );
        assert_eq!(system.cpu_snapshot().af, initial.af);
        assert_eq!(system.elapsed_ticks(), 4);
        assert!(
            system
                .step_machine_cycle()
                .expect("LD operand cycle")
                .complete
        );
        let mut expected = initial;
        expected.pc = 0x0102;
        expected.af = u16::from(immediate) << 8 | (initial.af & 0xFF);
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.elapsed_ticks(), 8);
    }
}

#[test]
fn system_clocks_and_execution_states_are_independent() {
    let mut first = program_system(&[0, 0]);
    let second = program_system(&[0x3E, 0x42]);
    let untouched = second.cpu_snapshot();
    first.step_machine_cycle().expect("first system NOP");
    first.step_machine_cycle().expect("second NOP");
    assert_eq!(first.elapsed_ticks(), 8);
    assert_eq!(second.elapsed_ticks(), 0);
    assert_eq!(second.cpu_snapshot(), untouched);
}

#[test]
fn each_trace_records_its_own_bus_action_and_read_byte() {
    use crate::cpu::CycleAction;

    let mut system = program_system(&[0x00, 0x3E, 0x42, 0x00]);
    for (address, value, complete) in [
        (0x0100, 0x00, true),
        (0x0101, 0x3E, false),
        (0x0102, 0x42, true),
        (0x0103, 0x00, true),
    ] {
        let trace = system.step_machine_cycle().expect("supported cycle");
        match trace.action {
            CycleAction::Read { addr } => assert_eq!(addr, address),
            _ => panic!("expected a read cycle at {address:#06X}"),
        }
        assert_eq!(trace.read_value, Some(value));
        assert_eq!(trace.complete, complete);
    }
}

#[test]
fn each_trace_counts_one_machine_cycle_not_the_whole_instruction() {
    let mut system = program_system(&[0x00, 0x3E, 0x42, 0x00]);
    for expected_elapsed in [4, 8, 12, 16] {
        let trace = system.step_machine_cycle().expect("supported cycle");
        assert_eq!(trace.tick_before, expected_elapsed - 4);
        assert_eq!(trace.tick_after, expected_elapsed);
        assert_eq!(system.elapsed_ticks(), expected_elapsed);
    }
}

#[test]
fn failing_read_retains_its_byte_and_tick_span_in_the_error_trace() {
    use super::SystemError;
    use crate::cpu::{CpuError, CycleAction};

    let mut system = program_system(&[0xD3]);
    match system.step_machine_cycle().expect_err("unsupported opcode") {
        SystemError::CpuCompletionError(CpuError::UnsupportedOpcode(_), trace) => {
            assert_eq!(trace.tick_before, 0);
            assert_eq!(trace.tick_after, 4);
            assert!(matches!(trace.action, CycleAction::Read { addr: 0x0100 }));
            assert_eq!(trace.read_value, Some(0xD3));
            assert!(!trace.complete);
        }
        other => panic!("wrong execution error: {other:?}"),
    }
    assert_eq!(system.elapsed_ticks(), 4);
}
