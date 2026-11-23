//! #86 F2: frame-to-RGBA conversion and integer scaling
//! (docs/tasks/FRONTEND.md). Written ahead of implementation.

use super::{RGBA_LEN, integer_viewport, shade_rgba, write_rgba32};
use gameboy_emu::{
    memory::Cartridge,
    system::{MachineStep, System},
    video::{Frame, Shade},
};

const W: usize = Frame::WIDTH;
const H: usize = Frame::HEIGHT;

/// A ROM that turns the LCD off, fills 8000-9FFF with a pattern, sets BGP
/// E4, turns the LCD on (LCDC 91) and spins. The frame it renders has every
/// shade and no simple symmetry.
fn patterned_rom() -> Vec<u8> {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7; // Header checksum of an all-zero header.
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xC3, 0x50, 0x01]); // nop; jp 0150
    #[rustfmt::skip]
    let main = [
        0xF3,             // 0150 di
        0xAF,             // 0151 xor a
        0xE0, 0x40,       // 0152 ldh [LCDC], a     ; LCD off
        0x21, 0x00, 0x80, // 0154 ld hl, 8000
        0x7D,             // 0157 ld a, l           ; loop
        0xAC,             // 0158 xor h
        0x07,             // 0159 rlca
        0xAD,             // 015A xor l
        0x22,             // 015B ld [hl+], a
        0x7C,             // 015C ld a, h
        0xFE, 0xA0,       // 015D cp A0
        0x20, 0xF6,       // 015F jr nz, 0157
        0x3E, 0xE4,       // 0161 ld a, E4
        0xE0, 0x47,       // 0163 ldh [BGP], a
        0x3E, 0x91,       // 0165 ld a, 91
        0xE0, 0x40,       // 0167 ldh [LCDC], a     ; LCD on
        0x18, 0xFE,       // 0169 jr 0169
    ];
    rom[0x150..0x150 + main.len()].copy_from_slice(&main);
    rom
}

/// Run the patterned ROM until two full frames after the disable's blank
/// one have been published.
fn rendered_system() -> System {
    let mut system = System::new(Cartridge::from_bytes(&patterned_rom()).unwrap());
    for _ in 0..200_000 {
        if system.frames_completed() >= 3 {
            return system;
        }
        assert!(matches!(
            system.step_machine_cycle().unwrap(),
            MachineStep::Cycle(_)
        ));
    }
    panic!("patterned ROM published no frames");
}

#[test]
fn shades_map_to_the_dmg_acid2_greys_with_opaque_alpha() {
    assert_eq!(shade_rgba(Shade::White), [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(shade_rgba(Shade::LightGray), [0xAA, 0xAA, 0xAA, 0xFF]);
    assert_eq!(shade_rgba(Shade::DarkGray), [0x55, 0x55, 0x55, 0xFF]);
    assert_eq!(shade_rgba(Shade::Black), [0x00, 0x00, 0x00, 0xFF]);
}

#[test]
fn the_buffer_holds_four_bytes_per_pixel() {
    assert_eq!(RGBA_LEN, W * H * 4);
}

#[test]
fn a_blank_frame_overwrites_every_byte_with_white() {
    let mut out = [0x12; RGBA_LEN];
    write_rgba32(&Frame::new(), &mut out);
    assert!(out.iter().all(|&byte| byte == 0xFF));
}

#[test]
fn every_pixel_is_written_in_row_major_order() {
    let system = rendered_system();
    let frame = system.frame();
    // The fixture must exercise the layout: all four shades, and rows and
    // columns that differ.
    for shade in [
        Shade::White,
        Shade::LightGray,
        Shade::DarkGray,
        Shade::Black,
    ] {
        assert!(frame.pixels().contains(&shade), "fixture lacks {shade:?}");
    }
    assert_ne!(
        frame.pixels()[..W],
        frame.pixels()[W..2 * W],
        "rows 0 and 1"
    );
    assert!(
        (0..H).any(|y| frame.pixel(0, y) != frame.pixel(y % W, 0)),
        "fixture is symmetric"
    );

    // Two writes into the same buffer: the second must fully replace the first.
    let mut out = [0x12; RGBA_LEN];
    write_rgba32(&Frame::new(), &mut out);
    write_rgba32(frame, &mut out);
    for y in 0..H {
        for x in 0..W {
            let i = (y * W + x) * 4;
            assert_eq!(out[i..i + 4], shade_rgba(frame.pixel(x, y)), "x {x}, y {y}");
        }
    }
}

#[test]
fn the_default_window_shows_the_picture_at_scale_4_exactly() {
    assert_eq!(integer_viewport(640, 576), (0, 0, 640, 576));
}

#[test]
fn the_viewport_uses_the_largest_whole_scale_and_centres_it() {
    for (window, expected) in [
        ((160, 144), (0, 0, 160, 144)),
        ((641, 577), (0, 0, 640, 576)),
        ((800, 600), (80, 12, 640, 576)),
        ((801, 601), (80, 12, 640, 576)),
        ((1920, 1080), (400, 36, 1120, 1008)),
        ((3840, 2160), (720, 0, 2400, 2160)),
        ((500, 2000), (10, 784, 480, 432)),
    ] {
        assert_eq!(integer_viewport(window.0, window.1), expected, "{window:?}");
    }
}

#[test]
fn a_window_smaller_than_scale_1_keeps_scale_1_centred() {
    // Offsets truncate toward zero, as Rust integer division does.
    assert_eq!(integer_viewport(100, 100), (-30, -22, 160, 144));
    assert_eq!(integer_viewport(0, 0), (-80, -72, 160, 144));
    assert_eq!(integer_viewport(159, 1000), (0, 428, 160, 144));
}

#[test]
fn every_window_size_gets_a_maximal_centred_integer_scale() {
    for w in (0..=2000).step_by(7) {
        for h in (0..=2000).step_by(11) {
            let (x, y, vw, vh) = integer_viewport(w, h);
            let scale = vw / 160;
            assert!(scale >= 1, "{w}x{h}");
            assert_eq!((vw, vh), (160 * scale, 144 * scale), "{w}x{h}");
            if scale > 1 {
                assert!(vw <= w && vh <= h, "{w}x{h}: scale {scale} does not fit");
            }
            assert!(
                160 * (scale + 1) > w || 144 * (scale + 1) > h,
                "{w}x{h}: scale {scale} is not the largest"
            );
            assert_eq!(x, (w as i32 - vw as i32) / 2, "{w}x{h}");
            assert_eq!(y, (h as i32 - vh as i32) / 2, "{w}x{h}");
        }
    }
}
