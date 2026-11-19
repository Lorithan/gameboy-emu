use super::fixture;

#[test]
fn ppu_storage_starts_zero_and_reads_preserve_every_address_and_timer_phase() {
    let (bus, _) = fixture();
    let before: Vec<_> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    let timer = bus.timer;
    for _ in 0..2 {
        for addr in (0x8000..=0x9FFF).chain(0xFE00..=0xFE9F) {
            assert_eq!(bus.read(addr), 0, "startup {addr:04X}");
        }
    }
    for addr in 0u16..=u16::MAX {
        assert_eq!(bus.read(addr), before[usize::from(addr)]);
    }
    assert_eq!(bus.timer, timer);
}

#[test]
fn ppu_storage_preserves_every_offset_without_region_device_or_instance_aliases() {
    let (mut bus, _) = fixture();
    let (other, _) = fixture();
    bus.write(0xC000, 0x35);
    bus.write(0xDFFF, 0x76);
    bus.write(0xFF80, 0x91);
    bus.write(0xFFFE, 0x28);
    bus.write(0xFF05, 0xFF);
    bus.write(0xFF06, 0x42);
    bus.write(0xFF07, 5);
    bus.advance_mcycle();
    bus.advance_mcycle(); // Preserve a pending reload across all PPU accesses.
    let timer = bus.timer;
    let mut expected: Vec<_> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    let other_before: Vec<_> = (0u16..=u16::MAX).map(|addr| other.read(addr)).collect();
    // Separate low- and high-address patterns detect aliases farther apart
    // than one byte's pattern period; no expected value comes from PPU code.
    for high in [false, true] {
        for offset in 0u16..8192 {
            let value = if high {
                (offset / 256) as u8 ^ 0xA5
            } else {
                offset as u8
            };
            bus.write(0x8000 + offset, value);
            expected[usize::from(0x8000 + offset)] = value;
        }
        for offset in 0u16..160 {
            let value = (159 - offset) as u8 ^ if high { 0x5A } else { 0xFF };
            bus.write(0xFE00 + offset, value);
            expected[usize::from(0xFE00 + offset)] = value;
        }
        for addr in 0u16..=u16::MAX {
            assert_eq!(
                bus.read(addr),
                expected[usize::from(addr)],
                "pattern {high}, {addr:04X}"
            );
            assert_eq!(
                other.read(addr),
                other_before[usize::from(addr)],
                "other instance {addr:04X}"
            );
        }
        assert_eq!(bus.timer, timer);
    }
}

#[test]
fn ppu_storage_retains_all_byte_values_at_boundaries_and_interior() {
    let (mut bus, _) = fixture();
    let probes = [
        0x7FFF, 0x8000, 0x8001, 0x80FF, 0x8100, 0x9FFE, 0x9FFF, 0xA000, 0xFDFF, 0xFE00, 0xFE01,
        0xFE4F, 0xFE9E, 0xFE9F, 0xFEA0, 0xFEFF, 0xFF00,
    ];
    let mut expected: Vec<_> = probes.iter().map(|&addr| bus.read(addr)).collect();
    let timer = bus.timer;
    for (index, &addr) in probes.iter().enumerate() {
        if !matches!(addr, 0x8000..=0x9FFF | 0xFE00..=0xFE9F) {
            continue;
        }
        for value in 0u8..=u8::MAX {
            bus.write(addr, value);
            expected[index] = value;
            for (&probe, &value) in probes.iter().zip(&expected) {
                assert_eq!(
                    bus.read(probe),
                    value,
                    "write {addr:04X}, probe {probe:04X}"
                );
            }
        }
    }
    assert_eq!(bus.timer, timer);
}

#[test]
fn unusable_oam_tail_does_not_extend_or_alias_oam_storage() {
    let (mut bus, _) = fixture();
    for offset in 0u16..160 {
        bus.write(0xFE00 + offset, offset as u8 ^ 0xA5);
    }
    let before: Vec<_> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    for addr in 0xFEA0..=0xFEFF {
        for value in [0, 0x5A, 0xFF] {
            bus.write(addr, value);
            assert_eq!(bus.read(addr), 0xFF); // Declared project fallback.
        }
    }
    for addr in 0u16..=u16::MAX {
        assert_eq!(
            bus.read(addr),
            before[usize::from(addr)],
            "tail alias {addr:04X}"
        );
    }
}
