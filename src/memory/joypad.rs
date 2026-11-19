#[derive(Debug, Clone, Copy)]
pub enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Start,
    Select,
}

impl Button {
    fn mask(self) -> u8 {
        match self {
            Button::Right => 1 << 0,
            Button::Left => 1 << 1,
            Button::Up => 1 << 2,
            Button::Down => 1 << 3,
            Button::A => 1 << 4,
            Button::B => 1 << 5,
            Button::Select => 1 << 6,
            Button::Start => 1 << 7,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Joypad {
    selection: u8,
    pressed: u8,
}

#[cfg(test)]
mod tests;

impl Joypad {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            selection: 0,
            pressed: 0,
        }
    }

    pub(super) fn read(&self) -> u8 {
        0xC0 | self.selection | self.input_lines()
    }

    pub(super) fn write(&mut self, value: u8) -> JoypadEvent {
        let old_lines = self.input_lines();
        self.selection = value & 0x30;

        let new_lines = self.input_lines();
        if old_lines & !new_lines != 0 {
            JoypadEvent::FallingEdge
        } else {
            JoypadEvent::None
        }
    }

    pub(super) fn set_button(&mut self, button: Button, pressed: bool) -> JoypadEvent {
        let old_lines = self.input_lines();

        if pressed {
            self.pressed |= button.mask();
        } else {
            self.pressed &= !button.mask();
        }

        let new_lines = self.input_lines();
        if old_lines & !new_lines != 0 {
            JoypadEvent::FallingEdge
        } else {
            JoypadEvent::None
        }
    }

    fn input_lines(&self) -> u8 {
        let mut pressed_lines = 0u8;

        if self.selection & 0x10 == 0 {
            pressed_lines |= self.pressed & 0x0F;
        }

        if self.selection & 0x20 == 0 {
            pressed_lines |= self.pressed >> 4;
        }

        !pressed_lines & 0x0F
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum JoypadEvent {
    None,
    FallingEdge,
}
