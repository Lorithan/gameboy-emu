//! #89 F5: picker layout, hit test and text wrapping (docs/tasks/FRONTEND.md).
//! Sizes are in output pixels; the debug font is CELL x CELL per character.
//! The PickerLayout tests follow once its placement API settles.

use sdl3::pixels::Color;

use super::{
    Area, CELL, LINE, Label, OPEN_LABEL, PAD_X, PAD_Y, PickerLayout, text_scale, text_size,
    wrap_text,
};

#[test]
fn text_size_counts_characters_not_bytes() {
    assert_eq!(text_size("", 1.0), (0.0, CELL));
    assert_eq!(text_size("Open", 1.0), (4.0 * CELL, CELL));
    assert_eq!(text_size("Open", 2.0), (8.0 * CELL, 2.0 * CELL));
    // 'é' is two bytes and 'ア' three, but each is one character.
    assert_eq!(text_size("café", 1.0), (4.0 * CELL, CELL));
    assert_eq!(text_size("ア.gb", 3.0), (12.0 * CELL, 3.0 * CELL));
}

#[test]
fn contains_includes_the_top_left_edges_and_excludes_the_bottom_right() {
    let area = Area {
        x: 10.0,
        y: 20.0,
        w: 30.0,
        h: 40.0,
    };
    assert!(area.contains(10.0, 20.0), "top-left corner");
    assert!(area.contains(39.9, 59.9), "just inside bottom-right");
    assert!(!area.contains(40.0, 30.0), "right edge");
    assert!(!area.contains(20.0, 60.0), "bottom edge");
    assert!(!area.contains(40.0, 60.0), "bottom-right corner");
    assert!(!area.contains(9.9, 30.0), "left");
    assert!(!area.contains(20.0, 19.9), "above");
    assert!(!area.contains(40.1, 30.0), "right");
    assert!(!area.contains(20.0, 60.1), "below");
    // Two areas that touch never share a point.
    let next = Area {
        x: 40.0,
        y: 20.0,
        w: 30.0,
        h: 40.0,
    };
    for x in [39.9, 40.0, 40.1] {
        assert!(!(area.contains(x, 30.0) && next.contains(x, 30.0)), "x {x}");
    }
    let empty = Area {
        x: 5.0,
        y: 5.0,
        w: 0.0,
        h: 0.0,
    };
    assert!(!empty.contains(5.0, 5.0));
}

#[test]
fn the_text_scale_is_half_the_game_scale_and_at_least_one() {
    for ((width, height), scale) in [
        ((0, 0), 1.0),
        ((160, 144), 1.0),
        ((320, 288), 1.0),
        ((480, 432), 1.0),
        ((640, 576), 2.0),
        ((641, 577), 2.0),
        ((1920, 1080), 3.0),
        ((2560, 1440), 5.0),
        // The smaller axis decides, as for the game.
        ((3000, 288), 1.0),
        ((640, 5000), 2.0),
    ] {
        assert_eq!(text_scale(width, height), scale, "{width}x{height}");
    }
    // Never shrinks as the window grows.
    let mut previous = 0.0;
    for size in (0..4000).step_by(7) {
        let scale = text_scale(size, size * 9 / 10);
        assert!(scale >= previous && scale.fract() == 0.0, "{size}");
        previous = scale;
    }
}

/// The checks every wrap must pass, whatever the text.
fn check_wrap(text: &str, max_chars: usize) -> Vec<&str> {
    let lines = wrap_text(text, max_chars);
    let context = format!("{text:?} at {max_chars}: {lines:?}");
    for line in &lines {
        assert!(!line.is_empty(), "{context}: empty line");
        assert!(
            line.chars().count() <= max_chars,
            "{context}: {line:?} too long"
        );
        assert!(
            !line.starts_with(char::is_whitespace) && !line.ends_with(char::is_whitespace),
            "{context}: {line:?} has an edge space"
        );
        // Lines are slices of the text, in order.
        assert!(text.contains(line), "{context}: {line:?} not from the text");
    }
    // Nothing lost or repeated: the non-space characters survive in order.
    let wrapped: String = lines
        .concat()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let original: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(wrapped, original, "{context}: text changed");
    lines
}

#[test]
fn short_text_is_one_line_and_empty_text_none() {
    assert_eq!(check_wrap("Open ROM", 20), ["Open ROM"]);
    assert_eq!(check_wrap("Open ROM", 8), ["Open ROM"], "exactly fits");
    assert!(check_wrap("", 10).is_empty());
    assert!(check_wrap("   ", 10).is_empty(), "only spaces");
    assert_eq!(check_wrap("  padded  ", 10), ["padded"]);
}

#[test]
fn text_breaks_at_the_last_space_that_fits() {
    assert_eq!(
        check_wrap("not a valid ROM file", 10),
        ["not a", "valid ROM", "file"]
    );
    assert_eq!(check_wrap("Open ROM", 7), ["Open", "ROM"]);
    // A space exactly at the limit breaks there.
    assert_eq!(check_wrap("abcde fghij", 5), ["abcde", "fghij"]);
    // Runs of spaces at a break are dropped.
    assert_eq!(check_wrap("abc    def", 5), ["abc", "def"]);
}

#[test]
fn a_word_longer_than_a_line_is_cut_into_full_lines() {
    assert_eq!(check_wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
    let path = r"F:\Dev\gameboy-emu\target\test-roms\maxpirate.gb";
    let lines = check_wrap(path, 18);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[..2].iter().all(|line| line.chars().count() == 18));
    // A long path after a short word starts on its own line.
    let message = format!("bad {path}: not a ROM");
    let lines = check_wrap(&message, 18);
    assert_eq!(lines[0], "bad");
    assert!(lines[1].starts_with(r"F:\Dev"), "{lines:?}");
}

#[test]
fn multi_byte_characters_count_once_and_are_never_split() {
    assert_eq!(check_wrap("café olé", 4), ["café", "olé"]);
    assert_eq!(check_wrap("アイウエオ", 2), ["アイ", "ウエ", "オ"]);
    check_wrap(r"C:\Users\Zoë\ゲーム\テトリス.gb could not be read", 7);
}

#[test]
fn one_character_lines_still_work() {
    assert_eq!(check_wrap("ab c", 1), ["a", "b", "c"]);
}

#[test]
fn random_texts_always_wrap_cleanly() {
    let alphabet: Vec<char> = "ab é\\ア/.".chars().collect();
    let mut state = 0x2545_F491u32;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for _ in 0..2_000 {
        let len = next() as usize % 60;
        let text: String = (0..len)
            .map(|_| alphabet[next() as usize % alphabet.len()])
            .collect();
        let max_chars = 1 + next() as usize % 12;
        check_wrap(&text, max_chars);
    }
}

// --- PickerLayout --------------------------------------------------------

const SIZES: [(u32, u32); 8] = [
    (160, 144),
    (161, 145),
    (320, 288),
    (640, 576),
    (641, 577),
    (1920, 1080),
    (2560, 1440),
    (500, 2000),
];

const SHORT_ERROR: &str = "Not a Game Boy ROM";
const LONG_ERROR: &str = r"Could not read F:\Dev\gameboy-emu\target\test-roms\maxpirate\maxpirate.gb: The system cannot find the file specified. (os error 2)";

fn bottom(area: Area) -> f32 {
    area.y + area.h
}

fn centre_x(area: Area) -> f32 {
    area.x + area.w / 2.0
}

/// The size a label's text block occupies, at its own scale.
fn text_block(label: &Label) -> (f32, f32) {
    let s = label.scale;
    let widest = label
        .lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let lines = label.lines.len() as f32;
    (widest * CELL * s, (lines * LINE - PAD_Y).max(0.0) * s)
}

#[test]
fn a_one_line_label_is_its_text_plus_padding() {
    for scale in [1.0, 2.0, 3.0] {
        let label = Label::new(OPEN_LABEL, scale, 100, Color::BLACK);
        assert_eq!(label.lines, [OPEN_LABEL]);
        assert_eq!(
            (label.area.w, label.area.h),
            (
                (8.0 * CELL + 2.0 * PAD_X) * scale,
                (CELL + 2.0 * PAD_Y) * scale
            ),
            "scale {scale}"
        );
    }
    // The default window: a 160x32 button.
    let layout = PickerLayout::new(640, 576, None);
    assert_eq!((layout.button.area.w, layout.button.area.h), (160.0, 32.0));
}

#[test]
fn every_label_centres_its_text_at_its_own_scale() {
    for (width, height) in SIZES {
        for error in [None, Some(SHORT_ERROR), Some(LONG_ERROR)] {
            let layout = PickerLayout::new(width, height, error);
            for (name, label) in [
                ("button", Some(&layout.button)),
                ("hint", Some(&layout.hint)),
                ("error", layout.error.as_ref()),
            ] {
                let Some(label) = label else { continue };
                let (tw, th) = text_block(label);
                let left = label.text_at.0 - label.area.x;
                let right = bottom_right(label.area).0 - (label.text_at.0 + tw);
                let top = label.text_at.1 - label.area.y;
                let below = bottom(label.area) - (label.text_at.1 + th);
                let context = format!("{width}x{height} {name} {error:?}: {:?}", label.area);
                assert!(
                    (left - right).abs() <= 1.0,
                    "{context}: left {left} right {right}"
                );
                assert!(
                    (top - below).abs() <= 1.0,
                    "{context}: top {top} bottom {below}"
                );
                assert!(
                    label.area.x.fract() == 0.0 && label.area.y.fract() == 0.0,
                    "{context}: not on whole pixels"
                );
            }
        }
    }
}

fn bottom_right(area: Area) -> (f32, f32) {
    (area.x + area.w, area.y + area.h)
}

#[test]
fn the_rows_stack_error_button_hint_one_gap_apart() {
    for (width, height) in SIZES {
        for error in [SHORT_ERROR, LONG_ERROR] {
            let layout = PickerLayout::new(width, height, Some(error));
            let gap = CELL * layout.button.scale;
            let button = layout.button.area;
            let hint = layout.hint.area;
            let error_area = layout.error.as_ref().expect("an error label").area;
            let context = format!("{width}x{height} {error:?}");
            let below = hint.y - bottom(button);
            let above = button.y - bottom(error_area);
            assert!(
                (below - gap).abs() <= 1.0,
                "{context}: button to hint {below}, gap {gap}"
            );
            assert!(
                (above - gap).abs() <= 1.0,
                "{context}: error to button {above}, gap {gap}"
            );
        }
    }
}

#[test]
fn an_error_never_moves_the_button_or_the_hint() {
    for (width, height) in SIZES {
        let plain = PickerLayout::new(width, height, None);
        assert!(plain.error.is_none());
        for error in [SHORT_ERROR, LONG_ERROR] {
            let with = PickerLayout::new(width, height, Some(error));
            assert_eq!(with.button.area, plain.button.area, "{width}x{height}");
            assert_eq!(with.hint.area, plain.hint.area, "{width}x{height}");
        }
    }
}

#[test]
fn every_row_is_centred_and_within_the_window_width() {
    for (width, height) in SIZES {
        for error in [SHORT_ERROR, LONG_ERROR] {
            let layout = PickerLayout::new(width, height, Some(error));
            let error_area = layout.error.as_ref().unwrap().area;
            for (name, area) in [
                ("button", layout.button.area),
                ("hint", layout.hint.area),
                ("error", error_area),
            ] {
                let context = format!("{width}x{height} {name} {error:?}: {area:?}");
                assert!(
                    (centre_x(area) - width as f32 / 2.0).abs() <= 1.0,
                    "{context}: not centred"
                );
                assert!(
                    area.x >= 0.0 && area.x + area.w <= width as f32,
                    "{context}: off the side"
                );
            }
            // The button and the hint always fit vertically too; a short
            // error does as well. (Long errors in tiny windows: see below.)
            for area in [layout.button.area, layout.hint.area] {
                assert!(
                    area.y >= 0.0 && bottom(area) <= height as f32,
                    "{width}x{height}"
                );
            }
            if error == SHORT_ERROR {
                assert!(
                    error_area.y >= 0.0,
                    "{width}x{height}: short error off the top"
                );
            }
        }
    }
}

#[test]
fn the_button_is_centred_and_its_hit_test_matches() {
    for (width, height) in SIZES {
        let button = PickerLayout::new(width, height, None).button.area;
        let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
        assert!(button.contains(cx, cy), "{width}x{height}: window centre");
        assert!(
            (button.y + button.h / 2.0 - cy).abs() <= 1.0,
            "{width}x{height}"
        );
        assert!(!button.contains(button.x - 1.0, cy));
        assert!(!button.contains(button.x + button.w, cy));
        assert!(!button.contains(cx, bottom(button)));
    }
}

// --- Cutting long errors short --------------------------------------------

#[test]
fn a_long_error_fits_above_the_button_in_every_window() {
    for (width, height) in SIZES {
        let layout = PickerLayout::new(width, height, Some(LONG_ERROR));
        let error = layout.error.as_ref().unwrap();
        let gap = CELL * layout.button.scale;
        let context = format!("{width}x{height}: {:?} {:?}", error.area, error.lines);
        assert!(error.area.y >= 0.0, "{context}: off the top");
        assert!(
            (layout.button.area.y - bottom(error.area) - gap).abs() <= 1.0,
            "{context}: not one gap above the button"
        );
        // It uses the space: one more line would not have fitted.
        let line = LINE * error.scale;
        let cut = error.lines.last().unwrap().ends_with("...");
        if cut {
            assert!(error.area.y < line, "{context}: room for another line");
        }
    }
    // Every height from the minimum up, where rounding is easiest to get wrong.
    for height in 144..=320 {
        let layout = PickerLayout::new(160, height, Some(LONG_ERROR));
        let error = layout.error.unwrap();
        assert!(error.area.y >= 0.0, "160x{height}: {:?}", error.area);
    }
    // The smallest window shows four lines, the default ten or fewer.
    let small = PickerLayout::new(160, 144, Some(LONG_ERROR));
    assert_eq!(small.error.unwrap().lines.len(), 4);
}

#[test]
fn a_cut_error_keeps_its_first_lines_and_ends_with_dots() {
    let full = wrap_text(LONG_ERROR, 18);
    let cut = Label::new(LONG_ERROR, 1.0, 18, Color::BLACK).limited_to(3);
    assert!(full.len() > 3, "{full:?}");
    assert_eq!(cut.lines.len(), 3);
    assert_eq!(cut.lines[0], full[0]);
    assert_eq!(cut.lines[1], full[1]);
    let last = &cut.lines[2];
    assert!(last.ends_with("..."), "{last:?}");
    assert!(last.chars().count() <= 18, "{last:?} too long");
    assert!(
        full[2].starts_with(last.trim_end_matches("...")),
        "{last:?} is not the start of {:?}",
        full[2]
    );
    // The box is sized for the lines it now has.
    assert_eq!(cut.area.h, (3.0 * LINE + PAD_Y) * 1.0);
    for line in &cut.lines {
        assert!(line.chars().count() <= 18, "{line:?}");
    }
}

#[test]
fn an_error_that_fits_is_not_cut() {
    for max_lines in [3, 4, 10] {
        let label = Label::new("not a valid ROM file", 1.0, 10, Color::BLACK).limited_to(max_lines);
        assert_eq!(label.lines, ["not a", "valid ROM", "file"], "{max_lines}");
    }
    let one = Label::new("Not a ROM", 2.0, 20, Color::BLACK).limited_to(1);
    assert_eq!(one.lines, ["Not a ROM"]);
}

#[test]
fn a_cut_error_is_still_centred_on_its_text() {
    for (width, height) in [(160, 144), (161, 145), (200, 150)] {
        let layout = PickerLayout::new(width, height, Some(LONG_ERROR));
        let error = layout.error.as_ref().unwrap();
        assert!(
            error.lines.last().unwrap().ends_with("..."),
            "{width}x{height}"
        );
        let (tw, th) = text_block(error);
        let left = error.text_at.0 - error.area.x;
        let right = error.area.x + error.area.w - (error.text_at.0 + tw);
        let top = error.text_at.1 - error.area.y;
        let below = bottom(error.area) - (error.text_at.1 + th);
        assert!(
            (left - right).abs() <= 1.0,
            "{width}x{height}: {left} {right}"
        );
        assert!(
            (top - below).abs() <= 1.0,
            "{width}x{height}: {top} {below}"
        );
    }
}
