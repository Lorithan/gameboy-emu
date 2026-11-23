//! #87 F3: keyboard bindings, held keys and last-press-wins D-pad
//! (docs/tasks/FRONTEND.md). Written ahead of implementation. The rules are
//! tested with `char` keys; the default table with SDL scancodes.

use gameboy_emu::memory::Button;
use sdl3::keyboard::Scancode;

use super::{DEFAULT_BINDINGS, Input, InputEvent};

const ALL: [Button; 8] = [
    Button::Right,
    Button::Left,
    Button::Up,
    Button::Down,
    Button::A,
    Button::B,
    Button::Start,
    Button::Select,
];

/// Two keys for every direction and for A, one for the others.
const BINDINGS: [(char, Button); 12] = [
    ('w', Button::Up),
    ('^', Button::Up),
    ('s', Button::Down),
    ('v', Button::Down),
    ('a', Button::Left),
    ('<', Button::Left),
    ('d', Button::Right),
    ('>', Button::Right),
    ('k', Button::A),
    ('x', Button::A),
    ('j', Button::B),
    ('\n', Button::Start),
];

fn input() -> Input<char> {
    Input::new(BINDINGS)
}

/// The changes in one event in the order the loop applies them: the release,
/// then the press.
fn changes(event: InputEvent) -> Vec<(Button, bool)> {
    event
        .released
        .map(release)
        .into_iter()
        .chain(event.pressed.map(press))
        .collect()
}

fn down(input: &mut Input<char>, key: char) -> Vec<(Button, bool)> {
    changes(input.key_down(key))
}

fn up(input: &mut Input<char>, key: char) -> Vec<(Button, bool)> {
    changes(input.key_up(key))
}

fn press(button: Button) -> (Button, bool) {
    (button, true)
}

fn release(button: Button) -> (Button, bool) {
    (button, false)
}

#[test]
fn the_default_table_maps_wasd_arrows_and_the_action_keys() {
    let expected = [
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
    assert_eq!(DEFAULT_BINDINGS.len(), expected.len(), "binding count");
    for (key, button) in expected {
        assert!(
            DEFAULT_BINDINGS.contains(&(key, button)),
            "{key:?} -> {button:?} missing"
        );
    }
    // Every button is reachable, and Esc stays free for quitting.
    for button in ALL {
        assert!(
            DEFAULT_BINDINGS.iter().any(|&(_, b)| b == button),
            "{button:?}"
        );
    }
    assert!(
        DEFAULT_BINDINGS
            .iter()
            .all(|&(key, _)| key != Scancode::Escape)
    );
    // The default table is accepted by `new`.
    let _ = Input::new(DEFAULT_BINDINGS);
}

#[test]
#[should_panic]
fn a_key_bound_twice_is_rejected() {
    let _ = Input::new([('k', Button::A), ('k', Button::B)]);
}

#[test]
fn a_key_press_and_release_report_one_change_each() {
    let mut input = input();
    assert_eq!(down(&mut input, 'k'), [press(Button::A)]);
    assert_eq!(up(&mut input, 'k'), [release(Button::A)]);
    assert_eq!(down(&mut input, '\n'), [press(Button::Start)]);
    assert_eq!(up(&mut input, '\n'), [release(Button::Start)]);
}

#[test]
fn unbound_keys_and_repeats_change_nothing() {
    let mut input = input();
    assert!(down(&mut input, 'q').is_empty(), "unbound down");
    assert!(up(&mut input, 'q').is_empty(), "unbound up");
    assert!(
        up(&mut input, 'k').is_empty(),
        "release of a key never pressed"
    );
    assert_eq!(down(&mut input, 'k'), [press(Button::A)]);
    // SDL key repeat arrives as more key-downs of a held key.
    for _ in 0..5 {
        assert!(down(&mut input, 'k').is_empty(), "repeat");
    }
    assert_eq!(up(&mut input, 'k'), [release(Button::A)]);
    assert!(up(&mut input, 'k').is_empty(), "second release");
}

#[test]
fn a_button_stays_held_while_any_of_its_keys_is_held() {
    let mut input = input();
    assert_eq!(down(&mut input, 'k'), [press(Button::A)]);
    assert!(down(&mut input, 'x').is_empty(), "second key for A");
    assert!(up(&mut input, 'k').is_empty(), "x still holds A");
    assert_eq!(up(&mut input, 'x'), [release(Button::A)]);
    // The same through the D-pad, with the keys released in press order.
    assert_eq!(down(&mut input, 'w'), [press(Button::Up)]);
    assert!(down(&mut input, '^').is_empty());
    assert!(up(&mut input, 'w').is_empty());
    assert_eq!(up(&mut input, '^'), [release(Button::Up)]);
}

#[test]
fn the_last_pressed_direction_wins_on_each_axis() {
    let mut input = input();
    assert_eq!(down(&mut input, 'a'), [press(Button::Left)]);
    // Right while Left is held: Left is released, then Right pressed.
    assert_eq!(
        down(&mut input, 'd'),
        [release(Button::Left), press(Button::Right)]
    );
    // Releasing the winner hands the axis back to the still-held key.
    assert_eq!(
        up(&mut input, 'd'),
        [release(Button::Right), press(Button::Left)]
    );
    assert_eq!(up(&mut input, 'a'), [release(Button::Left)]);

    // Releasing the losing direction changes nothing.
    assert_eq!(down(&mut input, 's'), [press(Button::Down)]);
    assert_eq!(
        down(&mut input, 'w'),
        [release(Button::Down), press(Button::Up)]
    );
    assert!(up(&mut input, 's').is_empty(), "Down was not active");
    assert_eq!(up(&mut input, 'w'), [release(Button::Up)]);
}

#[test]
fn the_two_axes_and_the_action_buttons_are_independent() {
    let mut input = input();
    assert_eq!(down(&mut input, 'a'), [press(Button::Left)]);
    assert_eq!(down(&mut input, 'w'), [press(Button::Up)], "diagonal");
    assert_eq!(down(&mut input, 'k'), [press(Button::A)]);
    assert_eq!(
        down(&mut input, 'j'),
        [press(Button::B)],
        "A and B together"
    );
    assert_eq!(
        down(&mut input, '>'),
        [release(Button::Left), press(Button::Right)],
        "only the horizontal axis changes"
    );
    assert_eq!(up(&mut input, 'k'), [release(Button::A)]);
}

#[test]
fn recency_follows_the_button_not_the_key() {
    // A second key for an already-held direction does not make it newer.
    let mut input = input();
    assert_eq!(down(&mut input, 'a'), [press(Button::Left)]);
    assert_eq!(
        down(&mut input, 'd'),
        [release(Button::Left), press(Button::Right)]
    );
    assert!(down(&mut input, '<').is_empty(), "Left was already held");
    assert!(up(&mut input, 'a').is_empty(), "'<' still holds Left");
    assert_eq!(
        up(&mut input, 'd'),
        [release(Button::Right), press(Button::Left)]
    );
}

#[test]
fn release_all_releases_every_active_button_and_forgets_held_keys() {
    // For focus loss, when SDL may never deliver the key-ups.
    let mut input = input();
    assert!(input.release_all().is_empty(), "nothing held");
    down(&mut input, 'a');
    down(&mut input, 'd');
    down(&mut input, 'w');
    down(&mut input, 'x');
    let mut released = input.release_all();
    released.sort_by_key(|&button| ALL.iter().position(|&b| b == button));
    assert_eq!(released, [Button::Right, Button::Up, Button::A]);
    for key in ['a', 'd', 'w', 'x'] {
        assert!(up(&mut input, key).is_empty(), "{key:?} was forgotten");
    }
    assert_eq!(down(&mut input, 'a'), [press(Button::Left)], "fresh start");
}

/// Independent model of the rules for the random sequences below.
struct Model {
    held: Vec<char>,
    /// Press order of each button's physical (any key) hold.
    stamp: [u64; 8],
    clock: u64,
}

impl Model {
    fn index(button: Button) -> usize {
        ALL.iter().position(|&b| b == button).unwrap()
    }

    fn physical(&self, button: Button) -> bool {
        BINDINGS
            .iter()
            .any(|&(key, b)| b == button && self.held.contains(&key))
    }

    fn opposite(button: Button) -> Option<Button> {
        match button {
            Button::Left => Some(Button::Right),
            Button::Right => Some(Button::Left),
            Button::Up => Some(Button::Down),
            Button::Down => Some(Button::Up),
            _ => None,
        }
    }

    fn effective(&self, button: Button) -> bool {
        self.physical(button)
            && Self::opposite(button).is_none_or(|other| {
                !self.physical(other)
                    || self.stamp[Self::index(other)] < self.stamp[Self::index(button)]
            })
    }

    fn state(&self) -> [bool; 8] {
        ALL.map(|b| self.effective(b))
    }

    fn down(&mut self, key: char) {
        if self.held.contains(&key) {
            return;
        }
        let Some(&(_, button)) = BINDINGS.iter().find(|&&(k, _)| k == key) else {
            return;
        };
        if !self.physical(button) {
            self.clock += 1;
            self.stamp[Self::index(button)] = self.clock;
        }
        self.held.push(key);
    }

    fn up(&mut self, key: char) {
        self.held.retain(|&k| k != key);
    }
}

#[test]
fn random_key_sequences_report_exactly_the_effective_changes() {
    let keys: Vec<char> = BINDINGS
        .iter()
        .map(|&(key, _)| key)
        .chain(['q', 'r'])
        .collect();
    for seed in 1..=20u32 {
        let mut state = seed.wrapping_mul(0x9E37_79B9) | 1;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let mut input = input();
        let mut model = Model {
            held: Vec::new(),
            stamp: [0; 8],
            clock: 0,
        };
        let mut reported = [false; 8];
        for step in 0..2_000 {
            let key = keys[next() as usize % keys.len()];
            let is_down = next() % 2 == 0;
            let changes = if is_down {
                model.down(key);
                down(&mut input, key)
            } else {
                model.up(key);
                up(&mut input, key)
            };
            let context = format!("seed {seed}, step {step}, {key:?} down {is_down}");
            // Releases come before presses, each button at most once, and
            // each change really changes the reported state.
            let first_press = changes
                .iter()
                .position(|&(_, p)| p)
                .unwrap_or(changes.len());
            assert!(
                changes[first_press..].iter().all(|&(_, p)| p),
                "{context}: release after a press in {changes:?}"
            );
            for (i, &(button, pressed)) in changes.iter().enumerate() {
                assert!(
                    changes[..i].iter().all(|&(b, _)| b != button),
                    "{context}: {button:?} twice in {changes:?}"
                );
                let slot = &mut reported[Model::index(button)];
                assert_ne!(*slot, pressed, "{context}: no-op change {changes:?}");
                *slot = pressed;
            }
            assert_eq!(reported, model.state(), "{context}");
            if step % 500 == 499 {
                for button in input.release_all() {
                    reported[Model::index(button)] = false;
                }
                model.held.clear();
                assert_eq!(reported, [false; 8], "{context}: after release_all");
            }
        }
    }
}
