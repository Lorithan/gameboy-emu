pub(super) struct Timer {
    counter: u16,
    tima: u8,
    tma: u8,
    tac: u8,
}

#[cfg(test)]
mod tests;

impl Timer {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            counter: 0xABC8,
            tima: 0,
            tma: 0,
            tac: 0,
        }
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

    pub(super) fn read_tima(&self) -> u8 {
        self.tima
    }

    pub(super) fn write_tima(&mut self, value: u8) {
        self.tima = value;
    }

    pub(super) fn read_tma(&self) -> u8 {
        self.tma
    }

    pub(super) fn write_tma(&mut self, value: u8) {
        self.tma = value;
    }

    pub(super) fn read_tac(&self) -> u8 {
        self.tac | 0xF8
    }

    pub(super) fn write_tac(&mut self, value: u8) {
        self.tac = value & 0x07;
    }

    pub(super) fn on_stop_entered(&mut self) {
        self.counter = 0;
    }
}
