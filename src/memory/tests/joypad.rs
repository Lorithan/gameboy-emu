use super::fixture;
use crate::memory::{Bus, Button};

#[test]
fn stop_wake_bus_level_ignores_interrupt_flags_and_enables_for_every_input_mask() {
    let (mut bus, _) = fixture();
    let buttons = [
        (Button::Right, 1, 0x10),
        (Button::Left, 2, 0x10),
        (Button::Up, 4, 0x10),
        (Button::Down, 8, 0x10),
        (Button::A, 16, 0x20),
        (Button::B, 32, 0x20),
        (Button::Select, 64, 0x20),
        (Button::Start, 128, 0x20),
    ];
    for held in 0u8..=255 {
        for (button, bit, _) in buttons {
            bus.set_button(button, held & bit != 0);
        }
        for selection in [0, 0x10, 0x20, 0x30] {
            bus.write(0xFF00, selection);
            let expected = buttons
                .iter()
                .any(|(_, bit, row)| held & bit != 0 && selection & row == 0);
            for flags in [0, 16, 31, 0xE0, 0xFF] {
                for enable in [0, 16, 31, 0xE0, 0xFF] {
                    bus.write(0xFF0F, flags);
                    bus.write(0xFFFF, enable);
                    let joyp = bus.read(0xFF00);
                    for _ in 0..2 {
                        assert_eq!(
                            bus.stop_wake_active(),
                            expected,
                            "held {held:02X}, selection {selection:02X}, IF {flags:02X}, IE {enable:02X}"
                        );
                        assert_eq!(bus.read(0xFF00), joyp);
                        assert_eq!(bus.read(0xFF0F), flags | 0xE0);
                        assert_eq!(bus.read(0xFFFF), enable);
                        assert_eq!(bus.pending_interrupts(), flags & enable & 31);
                    }
                }
            }
        }
    }
}

#[test]
fn stop_wake_queries_preserve_memory_hidden_divider_phase_and_instance_isolation() {
    let (mut bus, _) = fixture();
    let (other, _) = fixture();
    bus.set_button(Button::A, true);
    bus.write(0xFF0F, 0); // An active level remains active after IF is cleared.
    bus.write(0xFFFF, 0);
    bus.write(0xC123, 0x35);
    bus.write(0xFF80, 0xA6);
    for _ in 0..13 {
        bus.advance_mcycle(); // ABC8 + 52 = ABFC, immediately before DIV changes.
    }
    let before: Vec<_> = (0u16..=65535).map(|addr| bus.read(addr)).collect();
    for _ in 0..4096 {
        assert!(bus.stop_wake_active());
        assert!(!other.stop_wake_active());
    }
    for addr in 0u16..=65535 {
        assert_eq!(bus.read(addr), before[usize::from(addr)], "{addr:04X}");
    }
    assert_eq!(bus.read(0xFF04), 0xAB);
    bus.advance_mcycle();
    assert_eq!(bus.read(0xFF04), 0xAC);
    assert_eq!(other.read(0xFF04), 0xAB);
    bus.set_button(Button::A, false);
    bus.write(0xFF0F, 0x10); // A persistent interrupt flag is not an active level.
    bus.write(0xFFFF, 0x10);
    assert!(!bus.stop_wake_active());
}

#[test]
fn joypad_bus_edges_request_if_independently_of_all_ie_and_if_values() {
    let (mut bus, _) = fixture();
    for flags in 0u8..=255 {
        for enable in 0u8..=255 {
            bus.write(0xFF00, 0x30);
            bus.set_button(Button::A, false);
            bus.write(0xFF0F, flags);
            bus.write(0xFFFF, enable);
            bus.set_button(Button::A, true);
            assert_eq!(bus.read(0xFF00), 0xFF);
            assert_eq!(bus.read(0xFF0F), flags | 0xE0, "unselected press");

            bus.write(0xFF00, 0x10);
            assert_eq!(bus.read(0xFF00), 0xDE);
            assert_eq!(bus.read(0xFF0F), flags | 0xF0);
            assert_eq!(bus.read(0xFFFF), enable);
            assert_eq!(bus.pending_interrupts(), enable & (flags | 0x10) & 31);

            bus.write(0xFF0F, flags & !0x10);
            for _ in 0..2 {
                bus.write(0xFF00, 0xDF); // Same selection, different ignored bits.
                bus.set_button(Button::A, true);
                assert_eq!(bus.read(0xFF00), 0xDE);
                assert_eq!(bus.read(0xFF0F), (flags & !0x10) | 0xE0);
            }
            bus.set_button(Button::A, false);
            assert_eq!(bus.read(0xFF00), 0xDF);
            assert_eq!(bus.read(0xFF0F), (flags & !0x10) | 0xE0);
            bus.set_button(Button::A, true);
            assert_eq!(bus.read(0xFF0F), flags | 0xF0);
            assert_eq!(bus.read(0xFFFF), enable);
            assert_eq!(bus.read(0xFF04), 0xAB);
        }
    }
}

fn check_memory(bus: &Bus, original: &[u8], joyp: u8, flags: u8) {
    for addr in 0u16..=65535 {
        let expected = match addr {
            0xFF00 => joyp,
            0xFF0F => flags,
            _ => original[usize::from(addr)],
        };
        assert_eq!(bus.read(addr), expected, "address {addr:04X}");
    }
}

#[test]
fn joypad_bus_shared_lines_and_selection_preserve_every_other_address() {
    enum Operation {
        Button(Button, bool),
        Select(u8),
    }
    use Operation::{Button as Press, Select};
    let (mut bus, _) = fixture();
    bus.write(0xC123, 0x35);
    bus.write(0xFF80, 0xA6);
    bus.write(0xFFFF, 0xA7);
    let original: Vec<_> = (0u16..=65535).map(|addr| bus.read(addr)).collect();
    for (operation, readback, falling) in [
        (Press(Button::A, true), 0xCE, true),
        (Press(Button::Right, true), 0xCE, false),
        (Press(Button::Right, false), 0xCE, false),
        (Press(Button::A, false), 0xCF, false),
        (Press(Button::Right, true), 0xCE, true),
        (Press(Button::Left, true), 0xCC, true),
        (Press(Button::A, true), 0xCC, false),
        (Select(0x30), 0xFF, false),
        (Select(0x10), 0xDE, true),
        (Select(0x20), 0xEC, true),
        (Select(0), 0xCC, false),
    ] {
        bus.write(0xFF0F, 5);
        match operation {
            Press(button, pressed) => bus.set_button(button, pressed),
            Select(value) => bus.write(0xFF00, value),
        }
        let flags = if falling { 0xF5 } else { 0xE5 };
        for _ in 0..2 {
            assert_eq!(bus.read(0xFF00), readback);
            check_memory(&bus, &original, readback, flags);
        }
    }
}

#[test]
fn joypad_bus_instances_keep_inputs_selection_and_requests_independent() {
    let (mut first, _) = fixture();
    let (mut second, _) = fixture();
    first.set_button(Button::Start, true);
    assert_eq!((first.read(0xFF00), first.read(0xFF0F)), (0xC7, 0xF1));
    assert_eq!((second.read(0xFF00), second.read(0xFF0F)), (0xCF, 0xE1));
    first.write(0xFF00, 0x30);
    second.write(0xFF00, 0x20);
    second.set_button(Button::Right, true);
    first.write(0xFF0F, 0);
    for _ in 0..2 {
        assert_eq!((first.read(0xFF00), first.read(0xFF0F)), (0xFF, 0xE0));
        assert_eq!((second.read(0xFF00), second.read(0xFF0F)), (0xEE, 0xF1));
    }
    assert_eq!((first.read(0xFFFF), second.read(0xFFFF)), (0, 0));
}
