//! #88 F4: headless smoke checks for the pinned interactive demo ROMs
//! (tests/fixtures/demo-roms.json). They show that each game boots, draws,
//! keeps running without the APU, and reacts to a button. They do not judge
//! whether the game plays correctly; that is the manual play-through.
//! Run tests/scripts/run-demo-roms.ps1, which checks the hashes first.

use std::{
    collections::{HashSet, hash_map::DefaultHasher},
    env, fs,
    hash::Hasher,
    path::PathBuf,
};

use gameboy_emu::{
    memory::{Button, Cartridge},
    system::{MachineStep, System},
    video::Shade,
};

/// One LCD frame of M-cycles, as the frontend runs them.
const MCYCLES_PER_FRAME: u32 = 17_556;

fn root() -> PathBuf {
    env::var_os("DEMO_ROM_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-roms"))
}

struct Run {
    system: System,
    /// Hashes of every distinct published frame that is not one flat shade.
    pictures: HashSet<u64>,
    stopped_frames: u32,
}

impl Run {
    fn new(relative: &str) -> Self {
        let path = root().join(relative);
        let rom = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        Self {
            system: System::new(Cartridge::from_bytes(&rom).unwrap()),
            pictures: HashSet::new(),
            stopped_frames: 0,
        }
    }

    /// Run `frames` frames like the frontend, recording each new picture.
    fn frames(&mut self, frames: u32) {
        for _ in 0..frames {
            let published = self.system.frames_completed();
            for _ in 0..MCYCLES_PER_FRAME {
                match self.system.step_machine_cycle() {
                    Ok(MachineStep::Cycle(_)) => {}
                    Ok(MachineStep::Stopped) => {
                        self.stopped_frames += 1;
                        break;
                    }
                    Err(error) => panic!("core error: {error}"),
                }
            }
            if self.system.frames_completed() != published {
                let pixels = self.system.frame().pixels();
                if pixels.iter().any(|&shade| shade != pixels[0]) {
                    let mut hasher = DefaultHasher::new();
                    for &shade in pixels {
                        hasher.write_u8(shade as u8);
                    }
                    self.pictures.insert(hasher.finish());
                }
            }
        }
    }

    fn hold(&mut self, button: Button, frames: u32) {
        self.system.set_button(button, true);
        self.frames(frames);
        self.system.set_button(button, false);
    }

    fn shades_in_view(&self) -> usize {
        let pixels = self.system.frame().pixels();
        [
            Shade::White,
            Shade::LightGray,
            Shade::DarkGray,
            Shade::Black,
        ]
        .iter()
        .filter(|shade| pixels.contains(shade))
        .count()
    }
}

/// Boot for `boot` frames (games may turn the LCD off while loading, so not
/// every frame is published), then press each of `presses` for 10 frames with
/// 20 frames between, checking that the game keeps drawing new pictures.
fn smoke(relative: &str, boot: u32, presses: &[Button]) {
    let mut run = Run::new(relative);
    run.frames(boot);
    let frames_after_boot = run.system.frames_completed();
    let pictures_after_boot = run.pictures.len();
    println!(
        "{relative}: after {boot} frames, {frames_after_boot} published, {pictures_after_boot} distinct pictures, {} shades in view",
        run.shades_in_view()
    );
    assert!(pictures_after_boot >= 1, "{relative}: nothing drawn");

    for &button in presses {
        run.hold(button, 10);
        run.frames(20);
    }
    run.frames(120);
    println!(
        "{relative}: after the presses, {} published, {} distinct pictures, {} shades in view, {} stopped frames",
        run.system.frames_completed(),
        run.pictures.len(),
        run.shades_in_view(),
        run.stopped_frames
    );
    assert!(
        run.pictures.len() > pictures_after_boot,
        "{relative}: no new picture after pressing {presses:?}"
    );
}

#[test]
#[ignore = "Requires the pinned demo ROMs; run tests/scripts/run-demo-roms.ps1"]
fn maxpirate_boots_draws_and_reacts_to_start() {
    smoke(
        "maxpirate/maxpirate.gb",
        400,
        &[
            Button::Start,
            Button::Start,
            Button::Right,
            Button::B,
            Button::A,
        ],
    );
}

#[test]
#[ignore = "Requires the pinned demo ROMs; run tests/scripts/run-demo-roms.ps1"]
fn gbsnake_boots_draws_and_reacts_to_the_d_pad() {
    smoke(
        "gbsnake/snake.gb",
        300,
        &[Button::Right, Button::Down, Button::Left, Button::Select],
    );
}
