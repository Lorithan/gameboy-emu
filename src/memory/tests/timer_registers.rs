use super::fixture;

#[test]
fn tima_cancellation_preserves_existing_if_bits_and_prevents_a_new_timer_request() {
    for flags in [0x11, 0x15] {
        let (mut bus, _) = fixture();
        bus.write(0xFF0F, flags);
        bus.write(0xFF05, 0xFF);
        bus.write(0xFF06, 0x42);
        bus.write(0xFF07, 5);
        bus.advance_mcycle();
        bus.advance_mcycle();
        assert_eq!(bus.read(0xFF05), 0);
        bus.write(0xFF05, 0x99);
        bus.advance_mcycle();
        assert_eq!(bus.read(0xFF05), 0x99);
        assert_eq!(bus.read(0xFF0F), 0xE0 + flags);
    }
}

#[test]
fn timer_reload_requests_if_at_the_next_mcycle_without_repeating_or_requiring_ie() {
    let (mut bus, _) = fixture();
    bus.write(0xFF0F, 0x11);
    bus.write(0xFFFF, 0);
    bus.write(0xFF05, 0xFF);
    bus.write(0xFF06, 0x42);
    bus.write(0xFF07, 5);
    // ABC8 reaches the fast timer's falling edge after eight base ticks.
    bus.advance_mcycle();
    assert_eq!((bus.read(0xFF05), bus.read(0xFF0F)), (0xFF, 0xF1));
    bus.advance_mcycle();
    assert_eq!((bus.read(0xFF05), bus.read(0xFF0F)), (0, 0xF1));
    bus.advance_mcycle();
    assert_eq!((bus.read(0xFF05), bus.read(0xFF0F)), (0x42, 0xF5));
    assert_eq!(
        bus.pending_interrupts(),
        0,
        "IF request must not depend on IE"
    );
    bus.write(0xFF0F, 0x11);
    bus.advance_mcycle();
    assert_eq!((bus.read(0xFF05), bus.read(0xFF0F)), (0x42, 0xF1));
}

#[test]
fn timer_bus_writes_exhaust_bytes_and_preserve_irq_storage_and_divider_phase() {
    for target in [0xFF05, 0xFF06, 0xFF07] {
        for value in 0u8..=255 {
            let (mut bus, _) = fixture();
            bus.write(0xFF0F, 0x15);
            bus.write(0xFFFF, 0xA7);
            bus.write(0xC000, 0x96);
            bus.write(0xFF80, 0x69);
            // Stop just before the first visible DIV increment. A reset or
            // extra advancement during the following writes must be visible.
            for _ in 0..13 {
                bus.advance_mcycle();
            }
            bus.write(0xFF05, 0xA5);
            bus.write(0xFF06, 0x5A);
            bus.write(0xFF07, 3);
            bus.write(target, value);
            assert_eq!(
                bus.read(0xFF05),
                if target == 0xFF05 { value } else { 0xA5 }
            );
            assert_eq!(
                bus.read(0xFF06),
                if target == 0xFF06 { value } else { 0x5A }
            );
            assert_eq!(
                bus.read(0xFF07),
                if target == 0xFF07 {
                    248 + value % 8
                } else {
                    251
                }
            );
            assert_eq!((bus.read(0xFF0F), bus.read(0xFFFF)), (0xF5, 0xA7));
            assert_eq!(
                (bus.read(0xC000), bus.read(0xE000), bus.read(0xFF80)),
                (0x96, 0x96, 0x69)
            );
            assert_eq!(bus.read(0xFF04), 0xAB);
            bus.advance_mcycle();
            assert_eq!(bus.read(0xFF04), 0xAC);
        }
    }
}

#[test]
fn timer_bus_full_address_space_preserves_storage_and_accounts_for_control_write_edges() {
    let (mut bus, _) = fixture();
    let (other, _) = fixture();
    let mut expected: Vec<u8> = (0u16..=u16::MAX).map(|addr| bus.read(addr)).collect();
    assert_eq!(&expected[0xFF04..=0xFF07], &[0xAB, 0, 0, 0xF8]);
    for target in [0xFF05u16, 0xFF06, 0xFF07] {
        for value in [0, 7, 0xF8, 0xFF] {
            bus.write(target, value);
            expected[usize::from(target)] = if target == 0xFF07 {
                248 + value % 8
            } else {
                value
            };
            if target == 0xFF07 && value == 0xF8 {
                // In this fixed sequence, TAC 07 -> 00 disables the high
                // selected input at startup counter ABC8. TIMA was FF, so
                // the write overflows it to 00. No ticks elapse to reload it.
                expected[0xFF05] = 0;
            }
            for addr in 0u16..=u16::MAX {
                assert_eq!(
                    bus.read(addr),
                    expected[usize::from(addr)],
                    "write {target:04X}, value {value:02X}, read {addr:04X}"
                );
            }
            assert_eq!(
                (other.read(0xFF05), other.read(0xFF06), other.read(0xFF07)),
                (0, 0, 0xF8)
            );
        }
    }
}
