const DOT_MAX: u16 = 455u16;
const LINE_MAX: u8 = 153u8;
const DRAWING_START: u16 = 80u16;
const HBLANK_START: u16 = 252u16;
const VBLANK_START: u8 = 144u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PpuMode {
    HBlank,
    VBlank,
    OamScan,
    Drawing,
}

impl PpuMode {
    pub(super) fn stat_bits(&self) -> u8 {
        match self {
            PpuMode::HBlank => 0,
            PpuMode::VBlank => 1,
            PpuMode::OamScan => 2,
            PpuMode::Drawing => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PpuTiming {
    line: u8,
    dot: u16,
    mode: PpuMode,
    first_enabled_line: bool,
}

impl PpuTiming {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            line: 153,
            dot: 4,
            mode: PpuMode::VBlank,
            first_enabled_line: false,
        }
    }

    pub(super) fn lcd_on() -> Self {
        Self {
            line: 0,
            dot: 0,
            mode: PpuMode::HBlank,
            first_enabled_line: true,
        }
    }

    pub(super) fn lcd_off() -> Self {
        Self {
            line: 0,
            dot: 0,
            mode: PpuMode::HBlank,
            first_enabled_line: false,
        }
    }

    pub(super) fn mode(&self) -> PpuMode {
        self.mode
    }

    pub(super) fn visible_ly(&self) -> u8 {
        if self.line == LINE_MAX {
            if self.dot > 3 { 0 } else { self.line }
        } else {
            if self.line_end() {
                self.line + 1
            } else {
                self.line
            }
        }
    }

    pub(super) fn lyc_compare_line(&self) -> Option<u8> {
        if self.line_end() {
            return None;
        }

        Some(self.visible_ly())
    }

    pub(super) fn just_entered_line_144(&self) -> bool {
        self.line == VBLANK_START && self.dot == 0
    }

    pub(super) fn just_entered_drawing(&self) -> bool {
        self.line < VBLANK_START && self.dot == 80
    }

    fn line_end(&self) -> bool {
        self.line < 143 && (self.dot > 451)
    }

    fn normal_line(&self) -> bool {
        !self.first_enabled_line && self.line < VBLANK_START
    }

    fn in_drawing(&self) -> bool {
        self.line < VBLANK_START && matches!(self.dot, 80..=251)
    }

    pub(super) fn oam_read_blocked(&self) -> bool {
        if self.normal_line() {
            self.dot < 252 || self.line_end()
        } else {
            self.in_drawing() || self.line_end()
        }
    }

    pub(super) fn oam_write_blocked(&self) -> bool {
        if self.normal_line() {
            matches!(self.dot, 0..=75 | 80..=251)
        } else {
            self.in_drawing()
        }
    }

    pub(super) fn vram_read_blocked(&self) -> bool {
        if self.normal_line() {
            matches!(self.dot, 76..=251)
        } else {
            self.in_drawing()
        }
    }

    pub(super) fn vram_write_blocked(&self) -> bool {
        if self.normal_line() {
            matches!(self.dot, 80..=251)
        } else {
            self.in_drawing()
        }
    }

    pub(super) fn tick(&mut self) {
        self.dot += 1; // This doesn't need to worry about wrapping, as we'll reset it before u16 max

        if self.dot > DOT_MAX {
            self.dot = 0;
            self.first_enabled_line = false;
            self.line += 1;

            if self.line > LINE_MAX {
                self.line = 0;
            }
        }

        self.set_mode();
    }

    fn set_mode(&mut self) {
        if self.line >= VBLANK_START {
            self.mode = PpuMode::VBlank;
            return;
        }

        if self.dot == 0 {
            self.mode = if self.first_enabled_line {
                PpuMode::HBlank
            } else {
                PpuMode::OamScan
            };
        } else if self.dot == DRAWING_START {
            self.mode = PpuMode::Drawing;
        } else if self.dot == HBLANK_START {
            self.mode = PpuMode::HBlank;
        }
    }
}

#[cfg(test)]
mod tests;
