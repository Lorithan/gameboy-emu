use std::{
    error::Error,
    time::{Duration, Instant},
};

use gameboy_emu::{
    memory::{Button, Cartridge},
    system::{MachineStep, System},
};
use sdl3::{
    event::Event as Sdl3Event,
    keyboard::{Mod, Scancode},
};

use crate::{
    display::{RGBA_LEN, write_rgba32},
    input::Input,
    pacing::{MCYCLES_PER_FRAME, Pacer},
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
struct Stopwatch {
    total: Duration,                // time from finished running stretches
    running_since: Option<Instant>, // Some while running
}

impl Stopwatch {
    fn start(&mut self, now: Instant) {
        if self.running_since.is_none() {
            self.running_since = Some(now);
        }
    }

    fn stop(&mut self, now: Instant) {
        if let Some(since) = self.running_since {
            self.total += now - since;
            self.running_since = None;
        }
    }

    fn elapsed(&self, now: Instant) -> Duration {
        match self.running_since {
            Some(since) => self.total + (now - since),
            None => self.total,
        }
    }
}

pub struct Session {
    name: String,
    system: System,
    pacer: Pacer,
    last_frame: u64,
    paused: bool,
    stopwatch: Stopwatch,
    frame_count: u64,
}

impl Session {
    pub fn new(cartridge: Cartridge, name: String) -> Self {
        let system = System::new(cartridge);
        let pacer = Pacer::new(Instant::now());
        let last_frame = u64::MAX;

        let stopwatch = Stopwatch {
            total: Duration::ZERO,
            running_since: Some(Instant::now()),
        };

        Session {
            name,
            system,
            pacer,
            last_frame,
            paused: false,
            stopwatch,
            frame_count: 0,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn pause(&mut self) {
        self.paused = true;
        self.stopwatch.stop(Instant::now());
    }

    pub fn unpause(&mut self) {
        if self.paused {
            self.paused = false;
            self.stopwatch.start(Instant::now());
            self.pacer = Pacer::new(Instant::now());
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn handle_input(&mut self, event: &Sdl3Event, input: &mut Input<Scancode>) {
        match event {
            Sdl3Event::KeyDown {
                scancode: Some(Scancode::P),
                keymod,
                repeat: false,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                if self.paused {
                    self.unpause();
                } else {
                    self.pause();
                }
            }
            Sdl3Event::KeyDown {
                scancode: Some(key),
                ..
            } => {
                input.key_down(*key).apply(&mut self.system);
            }
            Sdl3Event::KeyUp {
                scancode: Some(key),
                ..
            } => {
                input.key_up(*key).apply(&mut self.system);
            }
            _ => {}
        }
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.paused {
            let frames = self.pacer.frames_due(Instant::now());

            'frames: for _ in 0..frames {
                for _ in 0..MCYCLES_PER_FRAME {
                    match self.system.step_machine_cycle() {
                        Ok(MachineStep::Cycle(_)) => {} // continue
                        Ok(MachineStep::Stopped) => {
                            break 'frames;
                        }
                        Err(e) => return Err(Box::new(e)),
                    }
                }
                self.frame_count += 1;
            }
        }
        Ok(())
    }

    pub fn take_frame(&mut self, out: &mut [u8; RGBA_LEN]) -> bool {
        if self.last_frame != self.system.frames_completed() {
            self.last_frame = self.system.frames_completed();
            write_rgba32(self.system.frame(), out);
            true
        } else {
            false
        }
    }

    pub fn release_buttons(&mut self, buttons: &[Button]) {
        for &button in buttons {
            self.system.set_button(button, false);
        }
    }

    pub fn wake_at(&self) -> Option<Instant> {
        if !self.paused {
            Some(self.pacer.next_due())
        } else {
            None
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let elapsed = self.stopwatch.elapsed(Instant::now());
        let secs = elapsed.as_secs_f64();
        eprintln!(
            "frames {} in {secs:.2} s ({:.2}/s)",
            self.frame_count,
            self.frame_count as f64 / secs
        );
    }
}
