use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use gameboy_emu::memory::Button;
use gameboy_emu::system::System;
use sdl3::keyboard::Scancode;

const ALL_BUTTONS: [Button; 8] = [
    Button::Right,
    Button::Left,
    Button::Up,
    Button::Down,
    Button::A,
    Button::B,
    Button::Start,
    Button::Select,
];

pub const DEFAULT_BINDINGS: [(Scancode, Button); 15] = [
    (Scancode::W, Button::Up),
    (Scancode::A, Button::Left),
    (Scancode::S, Button::Down),
    (Scancode::D, Button::Right),
    (Scancode::Up, Button::Up),
    (Scancode::Left, Button::Left),
    (Scancode::Down, Button::Down),
    (Scancode::Right, Button::Right),
    (Scancode::J, Button::B),
    (Scancode::K, Button::A),
    (Scancode::Z, Button::B),
    (Scancode::X, Button::A),
    (Scancode::Return, Button::Start),
    (Scancode::RShift, Button::Select),
    (Scancode::Backspace, Button::Select),
];

#[derive(Debug, Clone)]
pub struct Input<K> {
    bindings: HashMap<K, Button>,
    held_keys: HashSet<K>,
    last_horizontal: Option<Button>, // Left or Right: whichever was pressed most recently
    last_vertical: Option<Button>,   // Up or Down
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pressed: Option<Button>,
    released: Option<Button>,
}

impl InputEvent {
    pub fn apply(self, system: &mut System) {
        if let Some(button) = self.released {
            system.set_button(button, false);
        }
        if let Some(button) = self.pressed {
            system.set_button(button, true);
        }
    }
}

impl<K: Copy + Eq + Hash> Input<K> {
    pub fn new(initial_bindings: impl IntoIterator<Item = (K, Button)>) -> Self {
        let mut bindings: HashMap<K, Button> = HashMap::new();
        for (key, button) in initial_bindings {
            if let Some(old) = bindings.insert(key, button) {
                panic!("double binding, buttons {button:?} and {old:?} were bound to the same key")
            }
        }
        let held_keys = HashSet::with_capacity(bindings.len());

        Self {
            bindings,
            held_keys,
            last_horizontal: None,
            last_vertical: None,
        }
    }

    pub fn key_down(&mut self, key: K) -> InputEvent {
        match self.bindings.get(&key) {
            Some(&button) => {
                let before = self.effective();
                let already_held = self.physical() & bit(button) != 0;

                if !self.held_keys.insert(key) {
                    return InputEvent::default();
                }

                if !already_held {
                    match button {
                        Button::Left | Button::Right => self.last_horizontal = Some(button),
                        Button::Up | Button::Down => self.last_vertical = Some(button),
                        _ => {}
                    }
                }

                Self::event(before, self.effective())
            }
            None => InputEvent::default(),
        }
    }

    pub fn key_up(&mut self, key: K) -> InputEvent {
        let before = self.effective();
        if !self.held_keys.remove(&key) {
            return InputEvent::default();
        }

        Self::event(before, self.effective())
    }

    pub fn release_all(&mut self) -> Vec<Button> {
        let buttons = self.effective();
        self.held_keys.clear();
        buttons_from_mask(buttons)
    }

    fn physical(&self) -> u8 {
        self.held_keys
            .iter()
            .filter_map(|key| self.bindings.get(key))
            .fold(0, |acc, button| acc | bit(*button))
    }

    fn effective(&self) -> u8 {
        let mut bits = self.physical();

        for (a, b, last) in [
            (Button::Left, Button::Right, self.last_horizontal),
            (Button::Up, Button::Down, self.last_vertical),
        ] {
            let both_held = bits & bit(a) != 0 && bits & bit(b) != 0;
            if both_held
                && let Some(last) = last
                && let Some(loser) = dpad_opposite(last)
            {
                bits &= !bit(loser);
            }
        }

        bits
    }

    fn event(before: u8, after: u8) -> InputEvent {
        let changed = before ^ after;
        let released = button_of(changed & before);
        let pressed = button_of(changed & after);

        InputEvent { pressed, released }
    }
}

fn bit(button: Button) -> u8 {
    1 << ALL_BUTTONS.iter().position(|&b| b == button).unwrap()
}

fn button_of(mask: u8) -> Option<Button> {
    (mask != 0).then(|| ALL_BUTTONS[mask.trailing_zeros() as usize])
}

fn buttons_from_mask(mask: u8) -> Vec<Button> {
    ALL_BUTTONS
        .into_iter()
        .filter(|&b| mask & bit(b) != 0)
        .collect()
}

fn dpad_opposite(button: Button) -> Option<Button> {
    match button {
        Button::Right => Some(Button::Left),
        Button::Left => Some(Button::Right),
        Button::Up => Some(Button::Down),
        Button::Down => Some(Button::Up),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
