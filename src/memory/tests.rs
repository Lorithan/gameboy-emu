use super::{Bus, Cartridge};

fn fixture() -> (Bus, Vec<u8>) {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    for (address, value) in [(0, 0x12), (0x3FFF, 0x34), (0x4000, 0x56), (0x7FFF, 0x78)] {
        rom[address] = value;
    }
    let cart = Cartridge::from_bytes(&rom).expect("valid synthetic cartridge");
    (Bus::new(cart), rom)
}

#[test]
fn constructor_initializes_work_ram_and_echo_to_zero() {
    let (bus, _) = fixture();
    for address in 0xC000u16..=0xFDFF {
        assert_eq!(bus.read(address), 0, "address {address:#06X}");
    }
}

#[test]
fn bus_reads_rom_at_bank_boundaries_and_header() {
    let (bus, rom) = fixture();
    for address in [0, 0x014D, 0x3FFF, 0x4000, 0x7FFF] {
        assert_eq!(bus.read(address), rom[usize::from(address)]);
    }
}

#[test]
fn bus_ignores_rom_writes() {
    let (mut bus, rom) = fixture();
    for address in [0, 0x014D, 0x3FFF, 0x4000, 0x7FFF] {
        bus.write(address, 0xA5);
    }
    for address in 0u16..=0x7FFF {
        assert_eq!(bus.read(address), rom[usize::from(address)]);
    }
}

#[test]
fn work_ram_locations_are_independent() {
    let (mut bus, _) = fixture();
    let addresses = [0xC000, 0xCFFF, 0xD000, 0xDDFF, 0xDE00, 0xDFFF];
    for (index, &address) in addresses.iter().enumerate() {
        bus.write(address, 0x20 + index as u8);
    }
    for (index, &address) in addresses.iter().enumerate() {
        assert_eq!(bus.read(address), 0x20 + index as u8);
    }
}

#[test]
fn echo_aliases_work_ram_in_both_directions() {
    let (mut bus, _) = fixture();
    for (work, echo) in [(0xC000, 0xE000), (0xC123, 0xE123), (0xDDFF, 0xFDFF)] {
        bus.write(work, 0x35);
        assert_eq!(bus.read(echo), 0x35);
        bus.write(echo, 0x76);
        assert_eq!(bus.read(work), 0x76);
    }
    bus.write(0xDFFF, 0x9A);
    assert_eq!(bus.read(0xDDFF), 0x76);
    assert_eq!(bus.read(0xFDFF), 0x76);
}

#[test]
fn every_unimplemented_address_ignores_writes_without_corrupting_storage() {
    let (mut bus, rom) = fixture();
    bus.write(0xC000, 0x35);
    bus.write(0xDFFF, 0x76);
    for address in 0u16..=u16::MAX {
        if matches!(address, 0x0000..=0x7FFF | 0xC000..=0xFDFF) {
            continue;
        }
        assert_eq!(bus.read(address), 0xFF, "address {address:#06X}");
        bus.write(address, 0xA5);
        assert_eq!(bus.read(address), 0xFF, "address {address:#06X}");
    }
    assert_eq!(bus.read(0xC000), 0x35);
    assert_eq!(bus.read(0xDFFF), 0x76);
    assert_eq!(bus.cart.rom(), rom.as_slice());
}

#[test]
fn full_address_space_routes_to_expected_storage() {
    let (mut bus, rom) = fixture();
    let expected_ram: Vec<u8> = (0usize..8_192).map(|index| (index % 251) as u8).collect();
    for (offset, &value) in expected_ram.iter().enumerate() {
        bus.write(0xC000 + offset as u16, value);
    }
    for address in 0u16..=u16::MAX {
        let expected = match address {
            0x0000..=0x7FFF => rom[usize::from(address)],
            0xC000..=0xDFFF => expected_ram[usize::from(address - 0xC000)],
            0xE000..=0xFDFF => expected_ram[usize::from(address - 0xE000)],
            _ => 0xFF,
        };
        assert_eq!(bus.read(address), expected, "address {address:#06X}");
    }
}

#[test]
fn bus_instances_do_not_share_work_ram() {
    let (mut first, _) = fixture();
    let (second, _) = fixture();
    first.write(0xC000, 0x35);
    assert_eq!(first.read(0xE000), 0x35);
    assert_eq!(second.read(0xC000), 0);
    assert_eq!(second.read(0xE000), 0);
}
