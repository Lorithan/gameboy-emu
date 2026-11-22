//! #81 rendering R1: frame publication and the background, and #82 R2: the
//! window (docs/tasks/RENDERING.md, rules 1-3 and 7, decisions 1-3). Written
//! ahead of implementation. Expected frames come from the independent model
//! below, never from production code.
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
const LYC: u16 = 0xFF45;
const BGP: u16 = 0xFF47;
const WY: u16 = 0xFF4A;
const WX: u16 = 0xFF4B;
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
struct Scene {
    /// LCDC without bit 7: bits 0, 3 and 4 (R1) and 5 and 6 (R2).
    lcdc: u8,
    scx: u8,
    scy: u8,
    bgp: u8,
    wy: u8,
    wx: u8,
}

/// Window disabled (LCDC bit 5 clear); WX 7 and WY 0 would cover the
/// whole screen once it is enabled.
const BASE: Scene = Scene {
    lcdc: 0x11,
    scx: 0,
    scy: 0,
    bgp: 0xE4,
    wy: 0,
    wx: 7,
};

/// Colour index at `(px, py)` of the 256x256 layer whose tile map is `map`,
/// with LCDC bit 4 selecting the tile data.
fn layer_index(lcdc: u8, map: u16, px: usize, py: usize) -> u8 {
    let tile = vram(map + (py / 8 * 32 + px / 8) as u16);
    let base = if lcdc & 0x10 != 0 {
        0x8000 + u16::from(tile) * 16
    } else {
        (0x9000 + i32::from(tile as i8) * 16) as u16
    };
    let row = base + (py % 8 * 2) as u16;
    let bit = 7 - px % 8;
    let low = vram(row) >> bit & 1;
    let high = vram(row + 1) >> bit & 1;
    high << 1 | low
}

/// Rules 1-3 for one line. `window_row` is the window's internal line
/// counter when the window is drawn on this line.
fn expected_line(scene: Scene, ly: usize, window_row: Option<usize>) -> Vec<Shade> {
    let bg_map = if scene.lcdc & 0x08 != 0 {
        0x9C00
    } else {
        0x9800
    };
    let window_map = if scene.lcdc & 0x40 != 0 {
        0x9C00
    } else {
        0x9800
    };
    let wx = usize::from(scene.wx);
    (0..W)
        .map(|x| {
            let index = if scene.lcdc & 0x01 == 0 {
                0
            } else {
                match window_row {
                    Some(row) if x + 7 >= wx => {
                        layer_index(scene.lcdc, window_map, x + 7 - wx, row)
                    }
                    _ => layer_index(
                        scene.lcdc,
                        bg_map,
                        (usize::from(scene.scx) + x) % 256,
                        (usize::from(scene.scy) + ly) % 256,
                    ),
                }
            };
            SHADES[usize::from(scene.bgp >> (2 * index) & 3)]
        })
        .collect()
}

/// A frame whose line `ly` is drawn with `scene(ly)`, the registers as they
/// are when that line starts Drawing. The Y condition and the window line
/// counter start clear for each frame (rule 3).
fn expected_frame_by_line(scene: impl Fn(usize) -> Scene) -> Vec<Shade> {
    let mut y_condition = false;
    let mut window_line = 0;
    let mut frame = Vec::with_capacity(W * H);
    for ly in 0..H {
        let scene = scene(ly);
        // Set by a line with LY == WY whatever LCDC bit 5 is (Pan Docs,
        // Window: DMG; GBC differs).
        y_condition |= usize::from(scene.wy) == ly;
        let drawn = y_condition && scene.lcdc & 0x20 != 0 && scene.wx <= 166;
        frame.extend(expected_line(scene, ly, drawn.then_some(window_line)));
        if drawn {
            window_line += 1;
        }
    }
    frame
}

fn expected_frame(scene: Scene) -> Vec<Shade> {
    expected_frame_by_line(|_| scene)
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

fn write_registers(system: &mut System, bg: Scene) {
    system.memory.write(SCY, bg.scy);
    system.memory.write(SCX, bg.scx);
    system.memory.write(BGP, bg.bgp);
    system.memory.write(WY, bg.wy);
    system.memory.write(WX, bg.wx);
}

/// Turn the LCD off (publishing the blank frame), fill VRAM, set the
/// registers, and turn it back on with `bg.lcdc`.
fn enabled_with(bg: Scene) -> System {
    let mut system = machine();
    system.memory.write(LCDC, 0x00);
    for addr in 0x8000..=0x9FFF {
        system.memory.write(addr, vram(addr));
    }
    write_registers(&mut system, bg);
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
            let bg = Scene {
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
    let bg = Scene {
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
        let bg = Scene { bgp, ..BASE };
        let mut system = enabled_with(bg);
        run_to_publication(&mut system);
        assert_frame(&system, &expected_frame(bg), &format!("BGP {bgp:02X}"));
    }
}

#[test]
fn lcdc_bit_0_clear_shows_bgp_index_0_everywhere() {
    // Rule 2: index 0 through BGP, so a BGP whose index 0 is 3 gives Black.
    for (lcdc, bgp, shade) in [(0x10, 0xE7, Shade::Black), (0x18, 0x1A, Shade::DarkGray)] {
        let mut system = enabled_with(Scene { lcdc, bgp, ..BASE });
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
    let second = Scene {
        scx: 77,
        scy: 31,
        bgp: 0x1B,
        ..BASE
    };
    let mut system = enabled_with(first);
    run_to_publication(&mut system);
    let first_frame = expected_frame(first);
    assert_frame(&system, &first_frame, "first");
    write_registers(&mut system, second); // During VBlank.
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
    let expected = expected_frame_by_line(|ly| Scene {
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
    let bg = Scene { bgp: 0x1B, ..BASE };
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

// R2 (#82): the window, rule 3. Pan Docs (Window) is the source; WX 0 and
// WX 166 have DMG quirks and are left out (see RENDERING.md).

/// Scroll that differs from the window, so a window drawn with SCX/SCY or
/// the background map fails.
const SCROLLED: Scene = Scene {
    scx: 13,
    scy: 200,
    ..BASE
};

/// Step to the first M-cycle of line `ly`'s HBlank.
fn run_to_hblank(system: &mut System, ly: u8) {
    run_to(system, ly, 3);
    run_to(system, ly, 0);
}

#[test]
fn the_window_covers_x_from_wx_minus_7_and_lines_from_wy() {
    // WX 1-6 start the window off the left edge; WX 167+ and WY 144+ hide it.
    let positions = [1, 6, 7, 8, 50, 159, 165, 167, 200, 255]
        .map(|wx| (wx, 0))
        .into_iter()
        .chain([1, 77, 143, 144, 255].map(|wy| (7, wy)))
        .chain([(100, 60)]);
    for (wx, wy) in positions {
        let scene = Scene {
            lcdc: 0x31,
            wx,
            wy,
            ..SCROLLED
        };
        let mut system = enabled_with(scene);
        run_to_publication(&mut system);
        assert_frame(&system, &expected_frame(scene), &format!("{scene:?}"));
    }
}

#[test]
fn the_window_uses_its_own_map_and_the_shared_tile_data() {
    // Bit 6 selects the window map independently of bit 3; bit 4 selects
    // the tile data for both layers.
    for lcdc in [0x31, 0x39, 0x71, 0x79, 0x21, 0x29, 0x61, 0x69] {
        let scene = Scene {
            lcdc,
            wx: 40,
            wy: 30,
            ..SCROLLED
        };
        let mut system = enabled_with(scene);
        run_to_publication(&mut system);
        assert_frame(&system, &expected_frame(scene), &format!("{scene:?}"));
    }
}

#[test]
fn lcdc_bit_5_clear_hides_the_window() {
    for lcdc in [0x11, 0x51, 0x19, 0x59] {
        let scene = Scene { lcdc, ..SCROLLED };
        let mut system = enabled_with(scene);
        run_to_publication(&mut system);
        let background = expected_frame(Scene {
            lcdc: lcdc & !0x60,
            ..scene
        });
        assert_frame(&system, &background, &format!("LCDC {lcdc:02X}"));
    }
}

#[test]
fn lcdc_bit_0_clear_blanks_the_window_too() {
    // Rule 2 (LCDC, bit 0, DMG: "the Window Display Bit is ignored").
    for lcdc in [0x30, 0x70, 0x38] {
        let scene = Scene {
            lcdc,
            bgp: 0xE7,
            ..SCROLLED
        };
        let mut system = enabled_with(scene);
        run_to_publication(&mut system);
        assert_frame(&system, &[Shade::Black; W * H], &format!("LCDC {lcdc:02X}"));
    }
}

#[test]
fn the_line_counter_pauses_while_the_window_is_hidden_by_lcdc_or_wx() {
    // "If the Window is hidden (by any means) on a given scanline, the row
    // of pixels rendered the next time it's shown will be the same as if it
    // had not been hidden." Writes in HBlank apply from the next line.
    let shown = Scene {
        lcdc: 0x31,
        wx: 7,
        wy: 20,
        ..SCROLLED
    };
    let mut system = enabled_with(shown);
    run_to_hblank(&mut system, 50);
    system.memory.write(LCDC, 0x80 | 0x11);
    run_to_hblank(&mut system, 80);
    system.memory.write(LCDC, 0x80 | 0x31);
    run_to_hblank(&mut system, 100);
    system.memory.write(WX, 200);
    run_to_hblank(&mut system, 110);
    system.memory.write(WX, 30);
    run_to_publication(&mut system);
    let expected = expected_frame_by_line(|ly| Scene {
        lcdc: if (51..=80).contains(&ly) { 0x11 } else { 0x31 },
        wx: match ly {
            0..=100 => 7,
            101..=110 => 200,
            _ => 30,
        },
        ..shown
    });
    // Rows 0-30 on lines 20-50, 31-50 on 81-100, then 51 from line 111.
    assert_frame(&system, &expected, "hidden twice");
}

#[test]
fn the_y_condition_latches_for_the_rest_of_the_frame() {
    // Set on a line with LY == WY, then kept until VBlank: a WY for a line
    // already passed never triggers, a later WY triggers on its line, and
    // moving WY after triggering does not hide the window.
    let scene = Scene {
        lcdc: 0x31,
        wx: 60,
        wy: 200,
        ..SCROLLED
    };
    let mut system = enabled_with(scene);
    run_to_hblank(&mut system, 60);
    system.memory.write(WY, 50);
    run_to_hblank(&mut system, 80);
    system.memory.write(WY, 90);
    run_to_hblank(&mut system, 110);
    system.memory.write(WY, 255);
    run_to_publication(&mut system);
    let expected = expected_frame_by_line(|ly| Scene {
        wy: match ly {
            0..=60 => 200,
            61..=80 => 50,
            81..=110 => 90,
            _ => 255,
        },
        ..scene
    });
    // The window starts at line 90 with row 0 and stays to line 143.
    assert_frame(&system, &expected, "latched");
}

#[test]
fn the_y_condition_is_set_while_the_window_is_disabled() {
    // DMG: the Y condition does not depend on LCDC bit 5 (Pan Docs notes
    // the reset on clearing bit 5 for GBC only). Enabled after line WY,
    // the window starts with row 0.
    let scene = Scene {
        lcdc: 0x11,
        wx: 20,
        wy: 30,
        ..SCROLLED
    };
    let mut system = enabled_with(scene);
    run_to_hblank(&mut system, 70);
    system.memory.write(LCDC, 0x80 | 0x31);
    run_to_publication(&mut system);
    let expected = expected_frame_by_line(|ly| Scene {
        lcdc: if ly > 70 { 0x31 } else { 0x11 },
        ..scene
    });
    assert_frame(&system, &expected, "enabled after WY");
}

#[test]
fn the_y_condition_and_line_counter_restart_every_frame() {
    // Cleared at VBlank: the next frame starts the window at its own WY
    // with row 0, and an unchanged frame repeats exactly.
    let first = Scene {
        lcdc: 0x31,
        wx: 7,
        wy: 10,
        ..SCROLLED
    };
    let second = Scene {
        wx: 50,
        wy: 60,
        ..first
    };
    let mut system = enabled_with(first);
    run_to_publication(&mut system);
    assert_frame(&system, &expected_frame(first), "first");
    write_registers(&mut system, second); // During VBlank.
    for frame in 0..2 {
        run_to_publication(&mut system);
        assert_frame(
            &system,
            &expected_frame(second),
            &format!("frame {frame} after"),
        );
    }
}

#[test]
fn lyc_and_stat_writes_mid_frame_leave_the_window_alone() {
    // dmg-acid2 changes LYC from LY=LYC interrupts every few lines; only
    // VBlank and an LCD restart reset the Y condition and line counter.
    let scene = Scene {
        lcdc: 0x31,
        wx: 30,
        wy: 20,
        ..SCROLLED
    };
    let mut system = enabled_with(scene);
    for (ly, lyc) in [(10u8, 11u8), (40, 41), (60, 0), (100, 101)] {
        run_to_hblank(&mut system, ly);
        system.memory.write(LYC, lyc);
        system.memory.write(STAT, 0x40);
    }
    run_to_publication(&mut system);
    assert_frame(&system, &expected_frame(scene), "LYC rewritten");
}

#[test]
fn an_lcd_restart_mid_window_starts_the_next_frame_fresh() {
    // Turning the LCD off mid-frame ends that frame (decision 3), so the
    // first frame after enable starts with a clear Y condition and row 0.
    let window = Scene {
        lcdc: 0x31,
        wx: 7,
        wy: 10,
        ..SCROLLED
    };
    let later = Scene { wy: 50, ..window };
    let mut system = enabled_with(window);
    run_to_hblank(&mut system, 90); // Y condition set, counter at 81.
    system.memory.write(LCDC, window.lcdc);
    write_registers(&mut system, later);
    system.memory.write(LCDC, 0x80 | window.lcdc);
    run_to_publication(&mut system);
    assert_frame(&system, &expected_frame(later), "first frame after enable");
}
