//! #81 rendering R1: frame publication and the background
//! (docs/tasks/RENDERING.md, rules 1, 2 and 7, decisions 1-3). Written ahead
//! of implementation. Expected frames come from the independent background
//! model below, never from production code.
//!
//! The CPU spins on `JR -2` in ROM with IME 0, so it never touches video
//! state. Setup writes go straight to the Bus between M-cycles.

use super::super::System;
use crate::{
    memory::Cartridge,
    step_support::expect_cycle,
    video::{Frame, Shade},
};

const W: usize = Frame::WIDTH;
const H: usize = Frame::HEIGHT;
/// One frame: 154 lines of 456 dots.
const FRAME_MCYCLES: u64 = 154 * 456 / 4;
const SHADES: [Shade; 4] = [
    Shade::White,
    Shade::LightGray,
    Shade::DarkGray,
    Shade::Black,
];

const LCDC: u16 = 0xFF40;
const STAT: u16 = 0xFF41;
const SCY: u16 = 0xFF42;
const SCX: u16 = 0xFF43;
const LY: u16 = 0xFF44;
const BGP: u16 = 0xFF47;
const IF: u16 = 0xFF0F;

fn machine() -> System {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7;
    rom[0x100] = 0x18; // JR -2
    rom[0x101] = 0xFE;
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

/// Patterned VRAM: every colour index appears, and the two maps differ.
fn vram(addr: u16) -> u8 {
    if addr < 0x9800 {
        (addr as u8).wrapping_mul(29) ^ (addr >> 4) as u8 ^ 0x6B
    } else {
        let salt = if addr >= 0x9C00 { 0xA5 } else { 0x3C };
        (addr as u8).wrapping_mul(37) ^ ((addr >> 5) as u8).wrapping_mul(11) ^ salt
    }
}

#[derive(Debug, Clone, Copy)]
struct Background {
    /// LCDC without bit 7; only bits 0, 3 and 4 are used in R1.
    lcdc: u8,
    scx: u8,
    scy: u8,
    bgp: u8,
}

const BASE: Background = Background {
    lcdc: 0x11,
    scx: 0,
    scy: 0,
    bgp: 0xE4,
};

/// Rules 1 and 2 for one line.
fn expected_line(bg: Background, ly: usize) -> Vec<Shade> {
    (0..W)
        .map(|x| {
            let index = if bg.lcdc & 0x01 == 0 {
                0
            } else {
                let px = (usize::from(bg.scx) + x) % 256;
                let py = (usize::from(bg.scy) + ly) % 256;
                let map = if bg.lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 };
                let tile = vram(map + (py / 8 * 32 + px / 8) as u16);
                let base = if bg.lcdc & 0x10 != 0 {
                    0x8000 + u16::from(tile) * 16
                } else {
                    (0x9000 + i32::from(tile as i8) * 16) as u16
                };
                let row = base + (py % 8 * 2) as u16;
                let bit = 7 - px % 8;
                let low = vram(row) >> bit & 1;
                let high = vram(row + 1) >> bit & 1;
                high << 1 | low
            };
            SHADES[usize::from(bg.bgp >> (2 * index) & 3)]
        })
        .collect()
}

/// A frame whose line `ly` is drawn with `line_bg(ly)`.
fn expected_frame_by_line(line_bg: impl Fn(usize) -> Background) -> Vec<Shade> {
    (0..H)
        .flat_map(|ly| expected_line(line_bg(ly), ly))
        .collect()
}

fn expected_frame(bg: Background) -> Vec<Shade> {
    expected_frame_by_line(|_| bg)
}

fn assert_frame(system: &System, expected: &[Shade], context: &str) {
    let actual = system.frame().pixels();
    assert_eq!(actual.len(), W * H, "{context}: pixel count");
    if let Some(i) = (0..W * H).find(|&i| actual[i] != expected[i]) {
        panic!(
            "{context}: first mismatch at x {}, y {}: got {:?}, expected {:?}",
            i % W,
            i / W,
            actual[i],
            expected[i]
        );
    }
}

fn write_background_registers(system: &mut System, bg: Background) {
    system.memory.write(SCY, bg.scy);
    system.memory.write(SCX, bg.scx);
    system.memory.write(BGP, bg.bgp);
}

/// Turn the LCD off (publishing the blank frame), fill VRAM, set the
/// registers, and turn it back on with `bg.lcdc`.
fn enabled_with(bg: Background) -> System {
    let mut system = machine();
    system.memory.write(LCDC, 0x00);
    for addr in 0x8000..=0x9FFF {
        system.memory.write(addr, vram(addr));
    }
    write_background_registers(&mut system, bg);
    system.memory.write(LCDC, 0x80 | bg.lcdc);
    system
}

fn step(system: &mut System) {
    let _ = expect_cycle(system.step_machine_cycle().unwrap());
}

/// Step until the next publication and return the M-cycles taken.
fn run_to_publication(system: &mut System) -> u64 {
    let start = system.frames_completed();
    for mcycles in 1..=FRAME_MCYCLES + 1 {
        step(system);
        if system.frames_completed() != start {
            assert_eq!(system.frames_completed(), start + 1, "one publication");
            return mcycles;
        }
    }
    panic!("no publication within one frame");
}

/// Step until the first M-cycle with LY == `ly` and STAT mode `mode`.
fn run_to(system: &mut System, ly: u8, mode: u8) {
    for _ in 0..=FRAME_MCYCLES {
        if system.memory.read(LY) == ly && system.memory.read(STAT) & 3 == mode {
            return;
        }
        step(system);
    }
    panic!("LY {ly} mode {mode} never reached");
}

#[test]
fn a_new_system_shows_a_white_frame_and_no_completed_frames() {
    let system = machine();
    assert_eq!((W, H), (160, 144));
    assert_eq!(system.frames_completed(), 0);
    assert_eq!(system.frame().pixels().len(), W * H);
    assert!(system.frame().pixels().iter().all(|&s| s == Shade::White));
}

#[test]
#[should_panic]
fn pixel_panics_past_the_right_edge() {
    let _ = machine().frame().pixel(W, 0);
}

#[test]
#[should_panic]
fn pixel_panics_past_the_bottom_edge() {
    let _ = machine().frame().pixel(0, H);
}

#[test]
fn background_matches_the_model_for_both_tile_data_modes_maps_and_scrolls() {
    for lcdc in [0x11, 0x19, 0x01, 0x09] {
        for (scx, scy) in [(0, 0), (3, 5), (255, 255), (100, 200), (161, 90)] {
            let bg = Background {
                lcdc,
                scx,
                scy,
                ..BASE
            };
            let mut system = enabled_with(bg);
            run_to_publication(&mut system);
            assert_frame(&system, &expected_frame(bg), &format!("{bg:?}"));
        }
    }
}

#[test]
fn pixel_reads_the_row_major_position() {
    let bg = Background {
        scx: 3,
        scy: 5,
        ..BASE
    };
    let mut system = enabled_with(bg);
    run_to_publication(&mut system);
    let frame = system.frame();
    let expected = expected_frame(bg);
    for y in 0..H {
        for x in 0..W {
            assert_eq!(frame.pixel(x, y), frame.pixels()[y * W + x], "x {x}, y {y}");
            assert_eq!(frame.pixel(x, y), expected[y * W + x], "x {x}, y {y}");
        }
    }
}

#[test]
fn bgp_maps_each_colour_index_through_its_own_field() {
    for bgp in [0x1B, 0x9C, 0x27, 0x00, 0xFF] {
        let bg = Background { bgp, ..BASE };
        let mut system = enabled_with(bg);
        run_to_publication(&mut system);
        assert_frame(&system, &expected_frame(bg), &format!("BGP {bgp:02X}"));
    }
}

#[test]
fn lcdc_bit_0_clear_shows_bgp_index_0_everywhere() {
    // Rule 2: index 0 through BGP, so a BGP whose index 0 is 3 gives Black.
    for (lcdc, bgp, shade) in [(0x10, 0xE7, Shade::Black), (0x18, 0x1A, Shade::DarkGray)] {
        let mut system = enabled_with(Background { lcdc, bgp, ..BASE });
        run_to_publication(&mut system);
        assert_frame(&system, &[shade; W * H], &format!("LCDC {lcdc:02X}"));
    }
}

#[test]
fn publication_happens_with_the_vblank_request_every_154_lines() {
    // Rule 7: the frame is published in the same M-cycle as the VBlank
    // request, and never at any other M-cycle.
    let mut system = enabled_with(BASE);
    assert_eq!(
        system.frames_completed(),
        1,
        "the LCD disable published once"
    );
    system.memory.write(IF, 0);
    for publication in 0..3u64 {
        let mut mcycles = 0;
        loop {
            step(&mut system);
            mcycles += 1;
            let published = system.frames_completed() == 2 + publication;
            let vblank = system.memory.read(IF) & 1 != 0;
            assert_eq!(
                published, vblank,
                "M-cycle {mcycles} after publication {publication}"
            );
            if published {
                break;
            }
            assert!(mcycles <= FRAME_MCYCLES, "publication {publication} late");
        }
        if publication > 0 {
            assert_eq!(mcycles, FRAME_MCYCLES, "publication {publication}");
        }
        system.memory.write(IF, 0);
    }
}

#[test]
fn the_published_frame_never_shows_a_frame_in_progress() {
    // Decision 2: changes made after a publication appear only at the next.
    let first = BASE;
    let second = Background {
        scx: 77,
        scy: 31,
        bgp: 0x1B,
        ..BASE
    };
    let mut system = enabled_with(first);
    run_to_publication(&mut system);
    let first_frame = expected_frame(first);
    assert_frame(&system, &first_frame, "first");
    write_background_registers(&mut system, second); // During VBlank.
    let published = system.frames_completed();
    let mut mcycles = 0;
    while system.frames_completed() == published {
        if mcycles % 61 == 0 {
            assert_frame(&system, &first_frame, &format!("M-cycle {mcycles}"));
        }
        step(&mut system);
        mcycles += 1;
        assert!(mcycles <= FRAME_MCYCLES, "no second publication");
    }
    assert_frame(&system, &expected_frame(second), "second");
}

#[test]
fn register_writes_take_effect_from_the_next_line_rendered() {
    // Decision 1: each line is rendered when Drawing starts (dot 80), so a
    // write during OAM scan applies to its own line, and a write during
    // Drawing or HBlank applies from the next line.
    let scx = 99;
    let bgp = 0x6C;
    let scy = 140;
    let mut system = enabled_with(BASE);
    run_to(&mut system, 40, 2);
    system.memory.write(SCX, scx);
    run_to(&mut system, 90, 3);
    system.memory.write(BGP, bgp);
    run_to(&mut system, 120, 3);
    run_to(&mut system, 120, 0);
    system.memory.write(SCY, scy);
    run_to_publication(&mut system);
    let expected = expected_frame_by_line(|ly| Background {
        scx: if ly >= 40 { scx } else { BASE.scx },
        bgp: if ly >= 91 { bgp } else { BASE.bgp },
        scy: if ly >= 121 { scy } else { BASE.scy },
        ..BASE
    });
    assert_frame(&system, &expected, "split frame");
}

#[test]
fn turning_the_lcd_off_publishes_one_white_frame_and_then_nothing() {
    // Decision 3. BGP index 0 is Black here, so the blank frame cannot be
    // a background rendered through BGP.
    let bg = Background { bgp: 0x1B, ..BASE };
    let mut system = enabled_with(bg);
    run_to_publication(&mut system);
    assert_frame(&system, &expected_frame(bg), "before disable");
    run_to(&mut system, 70, 3); // Mid-frame.
    let before = system.frames_completed();
    system.memory.write(LCDC, bg.lcdc);
    assert_eq!(system.frames_completed(), before + 1, "disable publishes");
    assert_frame(&system, &[Shade::White; W * H], "after disable");
    system.memory.write(LCDC, 0x00);
    assert_eq!(system.frames_completed(), before + 1, "already off");
    for _ in 0..2 * FRAME_MCYCLES {
        step(&mut system);
    }
    assert_eq!(system.frames_completed(), before + 1, "nothing while off");
    assert_frame(&system, &[Shade::White; W * H], "while off");
    system.memory.write(LCDC, 0x80 | bg.lcdc);
    assert_eq!(
        system.frames_completed(),
        before + 1,
        "enable does not publish"
    );
    run_to_publication(&mut system);
    assert_frame(&system, &expected_frame(bg), "first frame after enable");
}
