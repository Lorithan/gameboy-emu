use super::{Bus, Cartridge};

mod joypad;
mod lcd_registers;
mod ppu;
mod timer_registers;

#[test]
fn interrupt_registers_exhaustive_readback_and_eligibility() {
    let (mut bus, _) = fixture();
    assert_eq!((bus.read(0xFF0F), bus.read(0xFFFF)), (0xE1, 0));
    for flags in 0u8..=255 {
        for enable in 0u8..=255 {
            bus.write(0xFF0F, flags);
            bus.write(0xFFFF, enable);
            for _ in 0..2 {
                assert_eq!(bus.read(0xFF0F), flags % 32 + 224);
                assert_eq!(bus.read(0xFFFF), enable);
                assert_eq!(bus.pending_interrupts(), (flags & enable) % 32);
            }
        }
    }
}

#[test]
fn interrupt_requests_and_acknowledgments_preserve_other_sources() {
    use super::Interrupt;
    fn source(index: u8) -> Interrupt {
        match index {
            0 => Interrupt::VBlank,
            1 => Interrupt::LCDStat,
            2 => Interrupt::Timer,
            3 => Interrupt::Serial,
            4 => Interrupt::Joypad,
            _ => panic!("invalid test source"),
        }
    }
    let (mut bus, _) = fixture();
    for flags in 0u8..32 {
        for enable in 0u8..=255 {
            for index in 0..5 {
                let mask = 2u8.pow(u32::from(index));
                bus.write(0xFF0F, flags);
                bus.write(0xFFFF, enable);
                for _ in 0..2 {
                    bus.request_interrupt(source(index));
                    assert_eq!(bus.read(0xFF0F), (flags | mask) + 224);
                    assert_eq!(bus.pending_interrupts(), (flags | mask) & enable & 31);
                    assert_eq!(bus.read(0xFFFF), enable);
                }
                for _ in 0..2 {
                    bus.acknowledge_interrupt(source(index));
                    assert_eq!(bus.read(0xFF0F), (flags & !mask) + 224);
                    assert_eq!(bus.pending_interrupts(), flags & !mask & enable & 31);
                    assert_eq!(bus.read(0xFFFF), enable);
                }
            }
        }
    }
    bus.write(0xFFFF, 0);
    bus.write(0xFF0F, 0);
    bus.request_interrupt(Interrupt::Timer);
    assert_eq!(bus.pending_interrupts(), 0);
    bus.write(0xFFFF, 4);
    assert_eq!(bus.pending_interrupts(), 4);
}

#[test]
fn interrupt_register_writes_have_no_aliases_and_instances_are_independent() {
    let (mut bus, _) = fixture();
    let (other, _) = fixture();
    bus.write(0xC123, 0x87);
    bus.write(0xFFFE, 0x64);
    for target in [0xFF0Fu16, 0xFFFF] {
        let original: Vec<_> = (0u16..=65535).map(|a| bus.read(a)).collect();
        for value in 0u8..=255 {
            bus.write(target, value);
            for addr in 0u16..=65535 {
                let expected = if addr == target {
                    if target == 0xFF0F {
                        value % 32 + 224
                    } else {
                        value
                    }
                } else {
                    original[usize::from(addr)]
                };
                assert_eq!(
                    bus.read(addr),
                    expected,
                    "target {target:04X}, address {addr:04X}"
                );
            }
        }
    }
    for addr in [0xFF0E, 0xFF10, 0xFF7F] {
        bus.write(addr, 0);
        assert_eq!(bus.read(addr), 255);
    }
    assert_eq!((other.read(0xFF0F), other.read(0xFFFF)), (0xE1, 0));
}

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
        if matches!(address, 0x0000..=0x7FFF | 0x8000..=0x9FFF | 0xC000..=0xFDFF | 0xFE00..=0xFE9F | 0xFF80..=0xFFFF | 0xFF00 | 0xFF04..=0xFF07 | 0xFF0F | 0xFF40..=0xFF45 | 0xFF47..=0xFF4B)
        {
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
    let expected_hram: Vec<u8> = (0u8..127).map(|offset| offset ^ 0xA5).collect();
    for (offset, &value) in expected_hram.iter().enumerate() {
        bus.write(0xFF80 + offset as u16, value);
    }
    for address in 0u16..=u16::MAX {
        let expected = match address {
            0x0000..=0x7FFF => rom[usize::from(address)],
            0x8000..=0x9FFF | 0xFE00..=0xFE9F => 0,
            0xC000..=0xDFFF => expected_ram[usize::from(address - 0xC000)],
            0xE000..=0xFDFF => expected_ram[usize::from(address - 0xE000)],
            0xFF80..=0xFFFE => expected_hram[usize::from(address - 0xFF80)],
            0xFF00 => 0xCF,
            0xFF04 => 0xAB,
            0xFF05 | 0xFF06 => 0,
            0xFF07 => 0xF8,
            0xFF0F => 0xE1,
            0xFF40 => 0x91,
            0xFF41 => 0x85, // No ticks: post-boot LY 0 == LYC 0, VBlank.
            0xFF44 => 0,
            0xFF42 | 0xFF43 | 0xFF45 | 0xFF4A | 0xFF4B => 0,
            0xFF47 => 0xFC,
            0xFF48 | 0xFF49 => 0xFF, // Deterministic object-palette policy.
            0xFFFF => 0,
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

#[test]
fn divider_bus_advances_four_base_ticks_per_cycle_and_is_instance_local() {
    let (mut first, _) = fixture();
    let (second, _) = fixture();
    assert_eq!(first.read(0xFF04), 0xAB);
    for mcycles in 0u32..=16_384 {
        let expected = ((43_976 + mcycles * 4) % 65_536 / 256) as u8;
        for _ in 0..2 {
            assert_eq!(first.read(0xFF04), expected, "cycle {mcycles}");
        }
        assert_eq!(second.read(0xFF04), 0xAB);
        if mcycles < 16_384 {
            first.advance_mcycle();
        }
    }
    first.write(0xFF04, 0xA5);
    assert_eq!(first.read(0xFF04), 0);
    assert_eq!(second.read(0xFF04), 0xAB);
}

#[test]
fn divider_bus_resets_phase_and_preserves_every_other_address() {
    let (mut bus, _) = fixture();
    bus.write(0xC123, 0x35);
    bus.write(0xFF80, 0xA6);
    bus.write(0xFF0F, 0x15);
    bus.write(0xFFFF, 0xA7);
    let before: Vec<_> = (0u16..=65535).map(|addr| bus.read(addr)).collect();
    for value in 0u8..=255 {
        for _ in 0..63 {
            bus.advance_mcycle();
        }
        bus.write(0xFF04, value);
        assert_eq!(bus.read(0xFF04), 0);
        for _ in 0..63 {
            bus.advance_mcycle();
        }
        assert_eq!(bus.read(0xFF04), 0);
        bus.advance_mcycle();
        assert_eq!(bus.read(0xFF04), 1);
        // Enter STOP with a nonzero hidden phase to verify this forwarding
        // operation also clears more than the visible upper byte.
        bus.advance_mcycle();
        bus.on_stop_entered();
        assert_eq!(bus.read(0xFF04), 0);
        for _ in 0..63 {
            bus.advance_mcycle();
        }
        assert_eq!(bus.read(0xFF04), 0);
        bus.advance_mcycle();
        assert_eq!(bus.read(0xFF04), 1);
    }
    // Since #77, VRAM/OAM reads depend on the PPU mode; disable the LCD so
    // the stored bytes are readable (memory::ppu::tests::access covers modes).
    bus.write(0xFF40, 0x11);
    for addr in 0u16..=65535 {
        // Live LY/STAT advance with time (memory::ppu::tests::status), and
        // LCDC was just changed above.
        if matches!(addr, 0xFF40 | 0xFF41 | 0xFF44) {
            continue;
        }
        let expected = if addr == 0xFF04 {
            1
        } else {
            before[usize::from(addr)]
        };
        assert_eq!(bus.read(addr), expected, "address {addr:04X}");
    }
}

#[test]
fn high_ram_is_zeroed_and_independent_across_bus_instances() {
    let (mut first, _) = fixture();
    let (second, _) = fixture();
    for address in 0xFF80u16..=0xFFFE {
        assert_eq!(first.read(address), 0);
        first.write(address, 0xA5);
        assert_eq!(first.read(address), 0xA5);
        assert_eq!(second.read(address), 0);
    }
}

#[test]
fn every_high_ram_location_stores_every_byte_without_aliasing_other_locations() {
    let (mut bus, _) = fixture();
    let mut expected = [0u8; 127];
    for address in 0xFF80u16..=0xFFFE {
        for value in 0u8..=255 {
            bus.write(address, value);
            expected[usize::from(address - 0xFF80)] = value;
            for (offset, &byte) in expected.iter().enumerate() {
                assert_eq!(
                    bus.read(0xFF80 + offset as u16),
                    byte,
                    "write {value:02X} at {address:04X}, read offset {offset}"
                );
            }
        }
    }
}

#[test]
fn high_ram_has_no_aliases_to_work_ram_echo_rom_or_neighbor_registers() {
    let (mut bus, rom) = fixture();
    for offset in 0u16..127 {
        bus.write(0xC000 + offset, 0x35);
        bus.write(0xFF80 + offset, 0xA6);
        assert_eq!(
            bus.read(0xC000 + offset),
            0x35,
            "HRAM write corrupted WRAM at offset {offset}"
        );
        assert_eq!(bus.read(0xE000 + offset), 0x35);
        bus.write(0xE000 + offset, 0x72);
        assert_eq!(bus.read(0xFF80 + offset), 0xA6);
    }
    bus.write(0xFF7F, 0x42);
    assert_eq!(bus.read(0xFF7F), 0xFF);
    assert_eq!(bus.read(0xFF80), 0xA6);
    assert_eq!(bus.read(0xFFFE), 0xA6);
    assert_eq!(bus.cart.rom(), rom.as_slice());
}
