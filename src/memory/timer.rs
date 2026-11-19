#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Timer {
    counter: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    phase: TimerPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimerPhase {
    Counting,
    Overflowing { ticks_left: u8 },
    Reloading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TimerEvent {
    None,
    InterruptRequested,
}

impl Timer {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            counter: 0xABC8,
            tima: 0,
            tma: 0,
            tac: 0,
            phase: TimerPhase::Counting,
        }
    }

    pub(super) fn tick(&mut self) -> TimerEvent {
        let old_input = self.timer_input();
        self.counter = self.counter.wrapping_add(1);
        let new_input = self.timer_input();

        match self.phase {
            TimerPhase::Counting => {
                if old_input && !new_input {
                    let (new_tima, wrapped) = self.tima.overflowing_add(1);
                    self.tima = new_tima;
                    if wrapped {
                        self.phase = TimerPhase::Overflowing { ticks_left: 4 };
                    }
                }

                TimerEvent::None
            }
            TimerPhase::Overflowing { ticks_left } => {
                if ticks_left == 1 {
                    self.tima = self.tma;
                    self.phase = TimerPhase::Reloading;
                    TimerEvent::InterruptRequested
                } else {
                    self.phase = TimerPhase::Overflowing {
                        ticks_left: ticks_left - 1,
                    };
                    TimerEvent::None
                }
            }
            TimerPhase::Reloading => {
                self.phase = TimerPhase::Counting;
                TimerEvent::None
            }
        }
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
        match self.phase {
            TimerPhase::Counting => self.tima = value,
            TimerPhase::Overflowing { .. } => {
                self.phase = TimerPhase::Counting;
                self.tima = value;
            }
            TimerPhase::Reloading => {} // ignore write during reloading
        }
    }

    pub(super) fn read_tma(&self) -> u8 {
        self.tma
    }

    pub(super) fn write_tma(&mut self, value: u8) {
        self.tma = value;
        if self.phase == TimerPhase::Reloading {
            self.tima = value;
        }
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

    fn timer_input(&self) -> bool {
        if self.tac & 0b100 == 0 {
            return false;
        }

        let bit_index = match self.tac & 0b11 {
            0b00 => 9,
            0b01 => 3,
            0b10 => 5,
            0b11 => 7,
            _ => unreachable!(),
        };

        let mask = 1 << bit_index;

        self.counter & mask != 0
    }
}

#[cfg(test)]
mod tests;
