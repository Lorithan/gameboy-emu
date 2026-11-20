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
