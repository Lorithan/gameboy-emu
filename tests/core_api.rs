use gameboy_emu::memory::{Bus, Cartridge};
use gameboy_emu::{cpu::CpuSnapShot, system::System};

#[test]
fn metadata_is_public_and_mutating_the_snapshot_does_not_change_the_cartridge() {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");

    // Integration tests exercise the library as an external consumer.
    let mut metadata = cartridge.metadata();
    assert_eq!(metadata.cartridge_type_code, 0);
    assert_eq!(metadata.rom_size, 32_768);
    assert_eq!(metadata.ram_size, 0);

    metadata.cartridge_type_code = 0xFF;
    metadata.rom_size = 1;
    metadata.ram_size = 8_192;
    assert_eq!(metadata.cartridge_type_code, 0xFF);
    assert_eq!(metadata.rom_size, 1);
    assert_eq!(metadata.ram_size, 8_192);

    let unchanged = cartridge.metadata();
    assert_eq!(unchanged.cartridge_type_code, 0);
    assert_eq!(unchanged.rom_size, 32_768);
    assert_eq!(unchanged.ram_size, 0);
    assert_eq!(cartridge.rom(), rom.as_slice());
}

#[test]
fn public_bus_api_routes_rom_and_work_ram() {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x7FFF] = 0x35;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");
    let mut bus = Bus::new(cartridge);

    assert_eq!(bus.read(0x014D), 0xE7);
    assert_eq!(bus.read(0x7FFF), 0x35);
    bus.write(0x7FFF, 0x76);
    assert_eq!(bus.read(0x7FFF), 0x35);
    bus.write(0xC123, 0xA5);
    assert_eq!(bus.read(0xE123), 0xA5);
    bus.write(0xE123, 0x5A);
    assert_eq!(bus.read(0xC123), 0x5A);
    assert_eq!(bus.read(0x8000), 0xFF);
}

#[test]
fn public_system_snapshot_reports_both_dmg_startup_profiles() {
    for (title_byte, checksum, af) in [(0xE7, 0, 0x0180), (0, 0xE7, 0x01B0)] {
        let mut rom = vec![0; 32_768];
        rom[0x0134] = title_byte;
        rom[0x014D] = checksum;
        let system = System::new(Cartridge::from_bytes(&rom).expect("valid synthetic ROM"));
        let expected = CpuSnapShot {
            af,
            bc: 0x0013,
            de: 0x00D8,
            hl: 0x014D,
            pc: 0x0100,
            sp: 0xFFFE,
        };
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.cpu_snapshot(), expected);
    }
}

#[test]
fn changing_public_cpu_snapshot_cannot_change_system_state() {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    let system = System::new(Cartridge::from_bytes(&rom).expect("valid synthetic ROM"));
    let original = system.cpu_snapshot();
    let mut changed = original;
    changed.af = 0xFFFF;
    changed.bc = 0xFFFF;
    changed.de = 0xFFFF;
    changed.hl = 0xFFFF;
    changed.pc = 0xFFFF;
    changed.sp = 0xFFFF;
    assert_ne!(changed, original);
    assert_eq!(system.cpu_snapshot(), original);
}

fn program_system(program: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + program.len()].copy_from_slice(program);
    System::new(Cartridge::from_bytes(&rom).expect("valid synthetic program"))
}

#[test]
fn public_machine_steps_advance_one_cycle_at_a_time() {
    let mut system = program_system(&[0x00, 0x3E, 0x42, 0x00]);
    let initial = system.cpu_snapshot();
    for (pc, ticks, a, read_value, complete) in [
        (0x0101, 4, 0x01, 0x00, true),
        (0x0102, 8, 0x01, 0x3E, false),
        (0x0103, 12, 0x42, 0x42, true),
        (0x0104, 16, 0x42, 0x00, true),
    ] {
        let trace = system.step_machine_cycle().expect("supported cycle");
        assert_eq!(trace.tick_before, ticks - 4);
        assert_eq!(trace.tick_after, ticks);
        assert_eq!(trace.read_value, Some(read_value));
        assert_eq!(trace.complete, complete);
        assert!(
            matches!(trace.action, gameboy_emu::cpu::CycleAction::Read { addr } if addr == pc - 1)
        );
        assert_eq!(system.elapsed_ticks(), ticks);
        let mut expected = initial;
        expected.pc = pc;
        expected.af = a << 8 | (initial.af & 0xFF);
        assert_eq!(system.cpu_snapshot(), expected);
    }
}

#[test]
fn unsupported_fetch_consumes_time_but_subsequent_faults_do_not() {
    use gameboy_emu::cpu::CpuError;
    use gameboy_emu::system::SystemError;

    let mut system = program_system(&[0x00, 0xD3]);
    system.step_machine_cycle().expect("NOP");
    assert!(matches!(
        system.step_machine_cycle(),
        Err(SystemError::CpuCompletionError(
            CpuError::UnsupportedOpcode(_),
            _
        ))
    ));
    assert_eq!(system.elapsed_ticks(), 8);
    assert_eq!(system.cpu_snapshot().pc, 0x0102);
    let faulted = system.cpu_snapshot();
    for _ in 0..3 {
        assert!(matches!(
            system.step_machine_cycle(),
            Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
        ));
        assert_eq!(system.elapsed_ticks(), 8);
        assert_eq!(system.cpu_snapshot(), faulted);
    }
}
