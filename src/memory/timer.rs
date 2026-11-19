pub(super) struct Timer {
    counter: u16,
}

#[cfg(test)]
mod tests;

impl Timer {
    pub(super) fn dmg_post_boot() -> Self {
        Self { counter: 0xABC8 }
    }

    pub(super) fn tick(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }

    pub(super) fn read_div(&self) -> u8 {
        (self.counter >> 8) as u8
    }

    pub(super) fn write_div(&mut self, _value: u8) {
        self.counter = 0;
    }

    pub(super) fn on_stop_entered(&mut self) {
        self.counter = 0;
    }
}
