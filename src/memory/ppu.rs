use crate::memory::ppu::{lcd::LcdRegisters, timing::PpuTiming};

mod lcd;
mod timing;

#[cfg(test)]
mod tests;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct PpuEffects {
    pub(super) vblank: bool,
    pub(super) lcd_stat: bool,
}

#[derive(Debug)]
pub(super) struct Ppu {
    vram: [u8; 0x2000],
    oam: [u8; 0xA0],
    lcd_registers: LcdRegisters,
    timing: PpuTiming,
}

impl Ppu {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcd_registers: LcdRegisters::dmg_post_boot(),
            timing: PpuTiming::dmg_post_boot(),
        }
    }

    pub(super) fn read_vram(&self, offset: usize) -> u8 {
        if self.vram_read_blocked() {
            0xFF
        } else {
            self.vram[offset]
        }
    }

    pub(super) fn read_vram_dma(&self, offset: usize) -> u8 {
        self.vram[offset]
    }

    pub(super) fn write_vram(&mut self, offset: usize, value: u8) {
        if !self.vram_write_blocked() {
            self.vram[offset] = value;
        }
    }

    pub(super) fn read_oam(&self, offset: usize) -> u8 {
        if self.oam_read_blocked() {
            0xFF
        } else {
            self.oam[offset]
        }
    }

    pub(super) fn write_oam(&mut self, offset: usize, value: u8) {
        if !self.oam_write_blocked() {
            self.oam[offset] = value;
        }
    }

    pub(super) fn write_oam_dma(&mut self, offset: usize, value: u8) {
        self.oam[offset] = value;
    }

    pub(super) fn read_scy(&self) -> u8 {
        self.lcd_registers.scy
    }

    pub(super) fn write_scy(&mut self, value: u8) {
        self.lcd_registers.scy = value;
    }

    pub(super) fn read_scx(&self) -> u8 {
        self.lcd_registers.scx
    }

    pub(super) fn write_scx(&mut self, value: u8) {
        self.lcd_registers.scx = value;
    }

    pub(super) fn read_lyc(&self) -> u8 {
        self.lcd_registers.lyc
    }

    pub(super) fn write_lyc(&mut self, value: u8) -> PpuEffects {
        self.lcd_registers.lyc = value;

        if self.lcd_on() {
            let lyc_equal = Some(value) == self.timing.lyc_compare_line();
            self.lcd_registers.set_lyc_equal(lyc_equal);

            let level = self
                .lcd_registers
                .lcd_stat
                .line_level(self.timing.mode(), self.timing.just_entered_line_144());
            let lcd_stat = self.lcd_registers.lcd_stat.update_line(level);

            PpuEffects {
                vblank: false,
                lcd_stat,
            }
        } else {
            PpuEffects::default()
        }
    }

    pub(super) fn read_bgp(&self) -> u8 {
        self.lcd_registers.bgp
    }

    pub(super) fn write_bgp(&mut self, value: u8) {
        self.lcd_registers.bgp = value;
    }

    pub(super) fn read_obp0(&self) -> u8 {
        self.lcd_registers.obp0
    }

    pub(super) fn write_obp0(&mut self, value: u8) {
        self.lcd_registers.obp0 = value;
    }

    pub(super) fn read_obp1(&self) -> u8 {
        self.lcd_registers.obp1
    }

    pub(super) fn write_obp1(&mut self, value: u8) {
        self.lcd_registers.obp1 = value;
    }

    pub(super) fn read_wy(&self) -> u8 {
        self.lcd_registers.wy
    }

    pub(super) fn write_wy(&mut self, value: u8) {
        self.lcd_registers.wy = value;
    }

    pub(super) fn read_wx(&self) -> u8 {
        self.lcd_registers.wx
    }

    pub(super) fn write_wx(&mut self, value: u8) {
        self.lcd_registers.wx = value;
    }

    pub(super) fn read_lcdc(&self) -> u8 {
        self.lcd_registers.lcd_control.bits()
    }

    pub(super) fn write_lcdc(&mut self, value: u8) {
        let old_enable = self.lcd_registers.lcd_control.is_enabled();
        self.lcd_registers.lcd_control.write(value);
        let new_enabled = self.lcd_registers.lcd_control.is_enabled();

        if old_enable && !new_enabled {
            self.timing = PpuTiming::lcd_off();
        } else if !old_enable && new_enabled {
            self.timing = PpuTiming::lcd_on();
        }
    }

    pub(super) fn read_ly(&self) -> u8 {
        self.timing.visible_ly()
    }

    pub(super) fn read_stat(&self) -> u8 {
        self.lcd_registers.read_stat(self.timing.mode())
    }

    pub(super) fn write_stat(&mut self, value: u8) -> PpuEffects {
        if self.lcd_on() {
            // Seems to be a weird bug or hardware quirk, where it looks like the interrupt is computed with all selection bits
            // despite what is actually written.
            let level = self.lcd_registers.lcd_stat.line_level_with(
                0xFF,
                self.timing.mode(),
                self.timing.just_entered_line_144(),
            );
            let lcd_stat = self.lcd_registers.lcd_stat.update_line(level);
            let effect = PpuEffects {
                vblank: false,
                lcd_stat,
            };

            self.lcd_registers.write_stat(value);

            let level = self
                .lcd_registers
                .lcd_stat
                .line_level(self.timing.mode(), self.timing.just_entered_line_144());
            self.lcd_registers.lcd_stat.update_line(level);

            effect
        } else {
            self.lcd_registers.write_stat(value);

            PpuEffects::default()
        }
    }

    pub(super) fn tick(&mut self) -> PpuEffects {
        if self.lcd_on() {
            self.timing.tick();

            self.lcd_registers
                .set_lyc_equal(self.timing.lyc_compare_line() == Some(self.lcd_registers.lyc));

            let vblank = self.timing.just_entered_line_144(); // Also serves as the mode 2 pulse for lcd_stat

            let level = self
                .lcd_registers
                .lcd_stat
                .line_level(self.timing.mode(), vblank);
            let lcd_stat = self.lcd_registers.lcd_stat.update_line(level);

            return PpuEffects { vblank, lcd_stat };
        }

        PpuEffects::default()
    }

    fn lcd_on(&self) -> bool {
        self.lcd_registers.lcd_control.is_enabled()
    }

    fn vram_read_blocked(&self) -> bool {
        self.lcd_on() && self.timing.vram_read_blocked()
    }

    fn vram_write_blocked(&self) -> bool {
        self.lcd_on() && self.timing.vram_write_blocked()
    }

    fn oam_read_blocked(&self) -> bool {
        self.lcd_on() && self.timing.oam_read_blocked()
    }

    fn oam_write_blocked(&self) -> bool {
        self.lcd_on() && self.timing.oam_write_blocked()
    }

    #[cfg(test)]
    pub(super) fn timing_probe(&self) -> String {
        format!("{:?}", self.timing)
    }
}
