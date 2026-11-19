use super::{Button, Joypad, JoypadEvent};

// Independent table: host-state bit, physical input line and row-select bit.
const BUTTONS: [(Button, u8, u8, u8); 8] = [
    (Button::Right, 1, 1, 0x10),
    (Button::Left, 2, 2, 0x10),
    (Button::Up, 4, 4, 0x10),
    (Button::Down, 8, 8, 0x10),
    (Button::A, 16, 1, 0x20),
    (Button::B, 32, 2, 0x20),
    (Button::Select, 64, 4, 0x20),
    (Button::Start, 128, 8, 0x20),
];

fn lines(held: u8, selection: u8) -> u8 {
    let mut result = 15;
    for (_, host_bit, input_bit, row) in BUTTONS {
        if held & host_bit != 0 && selection & row == 0 {
            result &= !input_bit;
        }
    }
    result
}

fn has_falling_edge(before: u8, after: u8) -> bool {
    (0..4).any(|bit| before & (1 << bit) != 0 && after & (1 << bit) == 0)
}

fn configured(held: u8, selection: u8) -> Joypad {
    let mut joypad = Joypad::dmg_post_boot();
    for (button, host_bit, _, _) in BUTTONS {
        joypad.set_button(button, held & host_bit != 0);
    }
    joypad.write(selection);
    joypad
}

#[test]
fn joypad_startup_masks_and_instances_are_independent() {
    for (button, host_bit, _, _) in BUTTONS {
        assert_eq!(button.mask(), host_bit);
    }
    let mut first = Joypad::dmg_post_boot();
    let second = Joypad::dmg_post_boot();
    assert_eq!((first.selection, first.pressed, first.read()), (0, 0, 0xCF));
    first.set_button(Button::A, true);
    first.write(0x30);
    assert_eq!(
        (first.selection, first.pressed, first.read()),
        (0x30, 16, 0xFF)
    );
    assert_eq!(
        (second.selection, second.pressed, second.read()),
        (0, 0, 0xCF)
    );
}

#[test]
fn joypad_matrix_readback_and_selection_edges_exhaust_all_masks_and_writes() {
    for held in 0u8..=255 {
        for selection in [0, 0x10, 0x20, 0x30] {
            let initial = configured(held, selection);
            for value in 0u8..=255 {
                let mut joypad = initial;
                let before = lines(held, selection);
                let selected = value & 0x30;
                let after = lines(held, selected);
                assert_eq!(
                    matches!(joypad.write(value), JoypadEvent::FallingEdge),
                    has_falling_edge(before, after),
                    "held {held:02X}, selection {selection:02X}, write {value:02X}"
                );
                assert_eq!((joypad.selection, joypad.pressed), (selected, held));
                for _ in 0..2 {
                    assert_eq!(joypad.read(), 0xC0 | selected | after);
                    assert_eq!((joypad.selection, joypad.pressed), (selected, held));
                }
                assert!(matches!(joypad.write(value ^ 0xCF), JoypadEvent::None));
                assert_eq!(joypad.read(), 0xC0 | selected | after);
            }
        }
    }
}

#[test]
fn joypad_press_release_edges_exhaust_masks_and_preserve_unrelated_buttons() {
    for held in 0u8..=255 {
        for selection in [0, 0x10, 0x20, 0x30] {
            let initial = configured(held, selection);
            for (button, host_bit, _, _) in BUTTONS {
                for pressed in [false, true] {
                    let mut joypad = initial;
                    let before = lines(held, selection);
                    let after_held = if pressed {
                        held | host_bit
                    } else {
                        held & !host_bit
                    };
                    let after = lines(after_held, selection);
                    assert_eq!(
                        matches!(joypad.set_button(button, pressed), JoypadEvent::FallingEdge),
                        has_falling_edge(before, after),
                        "held {held:02X}, selection {selection:02X}, {button:?}={pressed}"
                    );
                    assert_eq!((joypad.selection, joypad.pressed), (selection, after_held));
                    assert_eq!(joypad.input_lines(), after);
                    assert!(matches!(
                        joypad.set_button(button, pressed),
                        JoypadEvent::None
                    ));
                    assert_eq!((joypad.selection, joypad.pressed), (selection, after_held));
                }
            }
        }
    }
}
