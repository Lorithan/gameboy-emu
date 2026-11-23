use std::time::{Duration, Instant};

pub const MCYCLES_PER_FRAME: u32 = 17_556;
pub const MAX_CATCH_UP: u32 = 3;

pub struct Pacer {
    start: Instant,
    next: u64,
}

impl Pacer {
    pub fn new(start: Instant) -> Self {
        Self { start, next: 0 }
    }

    pub fn frames_due(&mut self, now: Instant) -> u32 {
        let mut count = 0;

        for frame in self.next..=self.next + MAX_CATCH_UP as u64 {
            if now < self.start + Self::deadline(frame) {
                break;
            }
            count += 1;
        }

        if count > MAX_CATCH_UP as u64 {
            self.start = now;
            self.next = 1;
            MAX_CATCH_UP
        } else {
            self.next += count;
            count as u32
        }
    }

    pub fn next_due(&self) -> Instant {
        self.start + Self::deadline(self.next)
    }

    fn deadline(n: u64) -> Duration {
        let nanos = n as u128 * 70_224 * 1_000_000_000 / 4_194_304;
        Duration::from_nanos_u128(nanos)
    }
}

#[cfg(test)]
mod tests;
