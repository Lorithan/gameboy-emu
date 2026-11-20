use super::fixture;

const REGISTERS: [(u16, u8); 8] = [
    (0xFF42, 0),
    (0xFF43, 0),
    (0xFF45, 0),
    (0xFF47, 0xFC),
    (0xFF48, 0xFF),
    (0xFF49, 0xFF),
    (0xFF4A, 0),
    (0xFF4B, 0),
];

#[test]
fn passive_lcd_startup_and_reads_preserve_all_memory_and_time_state() {
    let (bus, _) = fixture();
    let original: Vec<_> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    let timer = bus.timer;
    for _ in 0..3 {
        for (address, value) in REGISTERS {
            assert_eq!(bus.read(address), value, "startup {address:04X}");
        }
    }
    assert_eq!(bus.timer, timer);
    for address in 0u16..=u16::MAX {
        assert_eq!(bus.read(address), original[usize::from(address)]);
    }
}

#[test]
fn passive_lcd_all_bytes_preserve_other_registers_storage_irq_phase_and_instances() {
    let (mut bus, _) = fixture();
    let (other, _) = fixture();
    for (address, value) in [
        (0x8000, 0x35),
        (0x9FFF, 0x76),
        (0xFE00, 0x91),
        (0xFE9F, 0x28),
        (0xC000, 0xA5),
        (0xDFFF, 0x5A),
        (0xFF80, 0x19),
        (0xFFFE, 0xE2),
    ] {
        bus.write(address, value);
    }
    bus.write(0xFF0F, 0x11);
    bus.write(0xFFFF, 0x1F);
    bus.write(0xFF05, 0xFF);
    bus.write(0xFF06, 0x42);
    bus.write(0xFF07, 5);
    bus.advance_mcycle();
    bus.advance_mcycle(); // Pending Timer reload must survive passive accesses.
    assert_eq!(bus.read(0xFF05), 0);
    let timer = bus.timer;
    let mut expected: Vec<_> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    let untouched: Vec<_> = (0u16..=u16::MAX).map(|addr| other.read(addr)).collect();
    for byte in 0u8..=u8::MAX {
        for (index, &(address, _)) in REGISTERS.iter().enumerate() {
            let value = byte.wrapping_add(index as u8 * 23);
            bus.write(address, value);
            expected[usize::from(address)] = value;
            // Check each access separately so aliases cannot be hidden by
            // writing identical values to all registers in the same pass.
            for (probe, _) in REGISTERS {
                assert_eq!(
                    bus.read(probe),
                    expected[usize::from(probe)],
                    "write {address:04X}={value:02X}, read {probe:04X}"
                );
            }
            assert_eq!(bus.timer, timer);
        }
        for address in 0u16..=u16::MAX {
            assert_eq!(
                bus.read(address),
                expected[usize::from(address)],
                "byte {byte:02X}, memory {address:04X}"
            );
            assert_eq!(
                other.read(address),
                untouched[usize::from(address)],
                "other instance {address:04X}"
            );
        }
    }
}

#[test]
fn passive_lcd_mapping_keeps_status_ly_dma_and_neighbors_unmapped() {
    let (mut bus, _) = fixture();
    for (index, &(address, _)) in REGISTERS.iter().enumerate() {
        bus.write(address, 0x20 + index as u8);
    }
    bus.write(0x8000, 0x35);
    bus.write(0xFE00, 0x76);
    let original: Vec<_> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    let timer = bus.timer;
    // FF40 (LCDC) is mapped by #74; see memory::ppu::tests.
    for address in [0xFF3F, 0xFF41, 0xFF44, 0xFF46, 0xFF4C] {
        assert_eq!(bus.read(address), 0xFF);
        for value in [0, 0x5A, 0xFF] {
            bus.write(address, value);
            for probe in 0u16..=u16::MAX {
                assert_eq!(
                    bus.read(probe),
                    original[usize::from(probe)],
                    "gap write {address:04X}, read {probe:04X}"
                );
            }
            assert_eq!(bus.timer, timer);
        }
    }
}
