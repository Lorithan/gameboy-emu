//! Independent PPU position model shared by the #76 and #77 tests. It shares
//! no code with production timing or status logic.

pub(super) const DOTS_PER_LINE: u32 = 456;
pub(super) const DOTS_PER_FRAME: u32 = DOTS_PER_LINE * 154;
pub(super) const MCYCLES_PER_FRAME: u32 = DOTS_PER_FRAME / 4;
/// Post-boot seed: physical line 153, dot 4, as dots since line 0 / dot 0.
pub(super) const SEED_DOT: u32 = 153 * DOTS_PER_LINE + 4;

/// Independent position model: physical line, dot, and whether this is the
/// first line after enable (which reports HBlank instead of OAM scan).
#[derive(Clone, Copy, Debug)]
pub(super) struct Pos {
    pub(super) line: u32,
    pub(super) dot: u32,
    pub(super) first: bool,
}

impl Pos {
    pub(super) const SEED: Pos = Pos {
        line: 153,
        dot: 4,
        first: false,
    };
    pub(super) const ENABLED: Pos = Pos {
        line: 0,
        dot: 0,
        first: true,
    };

    pub(super) fn next(self) -> Pos {
        let (mut line, mut dot, mut first) = (self.line, self.dot + 1, self.first);
        if dot == DOTS_PER_LINE {
            dot = 0;
            first = false;
            line = (line + 1) % 154;
        }
        Pos { line, dot, first }
    }

    pub(super) fn mode(self) -> u8 {
        match (self.line, self.dot) {
            (144.., _) => 1,
            (_, 80..=251) => 3,
            (_, 0..=79) if !self.first => 2,
            _ => 0,
        }
    }

    pub(super) fn ly(self) -> u8 {
        if self.line == 153 && self.dot >= 4 {
            0
        } else {
            self.line as u8
        }
    }

    /// One-dot mode-2 source at physical line 144 / dot 0 (provisional).
    pub(super) fn pulse(self) -> bool {
        self.line == 144 && self.dot == 0
    }

    pub(super) fn line_level(self, selects: u8, lyc: u8) -> bool {
        let mode = self.mode();
        (selects & 0x08 != 0 && mode == 0)
            || (selects & 0x10 != 0 && mode == 1)
            || (selects & 0x20 != 0 && (mode == 2 || self.pulse()))
            || (selects & 0x40 != 0 && self.ly() == lyc)
    }
}
