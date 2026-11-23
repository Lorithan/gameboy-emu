//! #84 rendering R4: dmg-acid2 v1.0 (tests/fixtures/dmg-acid2.json).
//!
//! The ROM draws ten frames, then executes `ld b, b` from its LY=0x90
//! handler; the frame published at that line-144 entry must match
//! `img/reference-dmg.png` exactly. tests/scripts/run-dmg-acid2.ps1
//! verifies the pinned hashes and converts the PNG to `reference-dmg.shades`
//! (144 lines of 160 digits, 0 White to 3 Black) before running this.

use std::{
    env, fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use gameboy_emu::{
    memory::Cartridge,
    system::{InstructionStep, System},
    video::{Frame, Shade},
};

const LD_B_B: u8 = 0x40;
/// Ten frames are about 175,560 M-cycles; the cap allows for more than
/// five times that.
const MAX_MCYCLES: u64 = 1_000_000;
const WALL_TIME: Duration = Duration::from_secs(10);

/// `DMG_ACID2_ROOT`, or target/test-roms/dmg-acid2 like the Mooneye roots.
/// The script must have run once to write the reference shades.
fn root() -> PathBuf {
    env::var_os("DMG_ACID2_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-roms/dmg-acid2")
        })
}

fn reference(path: &PathBuf) -> Vec<Shade> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reference {}: {error}", path.display()));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), Frame::HEIGHT, "reference rows");
    lines
        .iter()
        .flat_map(|line| {
            assert_eq!(line.len(), Frame::WIDTH, "reference row width");
            line.bytes().map(|digit| match digit {
                b'0' => Shade::White,
                b'1' => Shade::LightGray,
                b'2' => Shade::DarkGray,
                b'3' => Shade::Black,
                other => panic!("reference digit {other}"),
            })
        })
        .collect()
}

fn digit(shade: Shade) -> char {
    match shade {
        Shade::White => '0',
        Shade::LightGray => '1',
        Shade::DarkGray => '2',
        Shade::Black => '3',
    }
}

#[test]
#[ignore = "Requires the pinned ROM and reference; run tests/scripts/run-dmg-acid2.ps1"]
fn dmg_acid2_matches_the_reference_frame() {
    let root = root();
    let rom_path = root.join("dmg-acid2.gb");
    let rom =
        fs::read(&rom_path).unwrap_or_else(|error| panic!("ROM {}: {error}", rom_path.display()));
    let expected = reference(&root.join("reference-dmg.shades"));
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());

    // Stop before the breakpoint instruction executes. Every instruction the
    // ROM runs is in ROM0, so the opcode at PC is read from the file.
    let started = Instant::now();
    let mut mcycles = 0;
    loop {
        let pc = usize::from(system.cpu_snapshot().pc);
        if rom.get(pc) == Some(&LD_B_B) {
            break;
        }
        assert!(
            mcycles < MAX_MCYCLES,
            "no ld b, b within {MAX_MCYCLES} M-cycles"
        );
        assert!(
            started.elapsed() < WALL_TIME,
            "no ld b, b within {WALL_TIME:?}"
        );
        mcycles += match system.step_instruction(MAX_MCYCLES).unwrap() {
            InstructionStep::Complete { mcycles, .. }
            | InstructionStep::InterruptEntered { mcycles, .. }
            | InstructionStep::InterruptEntryCancelled { mcycles, .. } => mcycles,
            other => panic!("unexpected step {other:?} at PC {pc:04X}"),
        };
    }
    let frames = system.frames_completed();
    let pc = system.cpu_snapshot().pc;
    println!(
        "dmg-acid2: ld b, b at PC {pc:04X} after {mcycles} M-cycles, {frames} frames completed, host {:?}",
        started.elapsed()
    );

    let actual = system.frame().pixels();
    let mismatches: Vec<usize> = (0..actual.len())
        .filter(|&i| actual[i] != expected[i])
        .collect();
    if let Some(&first) = mismatches.first() {
        let dump = root.join("actual-dmg.shades");
        let text: String = actual
            .chunks(Frame::WIDTH)
            .map(|row| row.iter().copied().map(digit).collect::<String>() + "\n")
            .collect();
        fs::write(&dump, text).unwrap();
        let rows: Vec<usize> = {
            let mut rows: Vec<usize> = mismatches.iter().map(|i| i / Frame::WIDTH).collect();
            rows.dedup();
            rows
        };
        panic!(
            "{} pixels differ; first at x {}, y {}: got {:?}, expected {:?}; rows {:?}; actual frame written to {}",
            mismatches.len(),
            first % Frame::WIDTH,
            first / Frame::WIDTH,
            actual[first],
            expected[first],
            rows,
            dump.display()
        );
    }
}
