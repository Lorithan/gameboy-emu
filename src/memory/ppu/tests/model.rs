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

    /// The last four dots of lines 0-142 (#73 D3).
    fn line_end_window(self) -> bool {
        self.line <= 142 && self.dot >= 452
    }

    /// A visible line other than the first after enable.
    fn normal_line(self) -> bool {
        self.line < 144 && !self.first
    }

    pub(super) fn ly(self) -> u8 {
        if self.line == 153 && self.dot >= 4 {
            0
        } else if self.line_end_window() {
            self.line as u8 + 1
        } else {
            self.line as u8
        }
    }

    /// The LY value the LY=LYC comparison sees; none in the line-end window,
    /// where the flag reads 0.
    pub(super) fn compare(self) -> Option<u8> {
        (!self.line_end_window()).then(|| self.ly())
    }

    /// CPU access locks while the LCD is on (#77, refined by #73 D3).
    pub(super) fn oam_read_blocked(self) -> bool {
        let drawing = self.line < 144 && (80..252).contains(&self.dot);
        (self.normal_line() && self.dot < 80) || drawing || self.line_end_window()
    }

    pub(super) fn oam_write_blocked(self) -> bool {
        let drawing = self.line < 144 && (80..252).contains(&self.dot);
        (self.normal_line() && self.dot < 76) || drawing
    }

    pub(super) fn vram_read_blocked(self) -> bool {
        let start = if self.normal_line() { 76 } else { 80 };
        self.line < 144 && (start..252).contains(&self.dot)
    }

    pub(super) fn vram_write_blocked(self) -> bool {
        self.line < 144 && (80..252).contains(&self.dot)
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
            || (selects & 0x40 != 0 && self.compare() == Some(lyc))
    }
}
