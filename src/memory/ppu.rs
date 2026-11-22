use crate::{
    memory::ppu::{lcd::LcdRegisters, timing::PpuTiming},
    video::{Frame, Shade},
};

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
    front_frame: Frame,
    back_frame: Frame,
    frames_completed: u64,
    window_line_counter: usize,
    window_y_triggered: bool,
}

impl Ppu {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcd_registers: LcdRegisters::dmg_post_boot(),
            timing: PpuTiming::dmg_post_boot(),
            front_frame: Frame::new(),
            back_frame: Frame::new(),
            frames_completed: 0,
            window_line_counter: 0,
            window_y_triggered: false,
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

            // Clear frame
            self.front_frame = Frame::new();
            self.frames_completed += 1;
            self.window_y_triggered = false;
            self.window_line_counter = 0;
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

    pub(super) fn frame(&self) -> &Frame {
        &self.front_frame
    }

    fn publish_frame(&mut self) {
        self.window_y_triggered = false;
        self.window_line_counter = 0;

        std::mem::swap(&mut self.back_frame, &mut self.front_frame);
        self.frames_completed += 1;
    }

    pub(super) fn frames_completed(&self) -> u64 {
        self.frames_completed
    }

    pub(super) fn tick(&mut self) -> PpuEffects {
        if self.lcd_on() {
            self.timing.tick();

            if self.timing.just_entered_drawing() {
                // Render a line
                let ly = self.timing.visible_ly();
                self.render_line(ly);
            }

            self.lcd_registers
                .set_lyc_equal(self.timing.lyc_compare_line() == Some(self.lcd_registers.lyc));

            let vblank = self.timing.just_entered_line_144(); // Also serves as the mode 2 pulse for lcd_stat

            if vblank {
                self.publish_frame();
            }

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

    fn render_line(&mut self, ly: u8) {
        // These are fixed for the whole line
        let lcdc = self.lcd_registers.lcd_control.bits();
        let scx = self.lcd_registers.scx;
        let scy = self.lcd_registers.scy;
        let bgp = self.lcd_registers.bgp;
        let wy = self.lcd_registers.wy;
        let wx = self.lcd_registers.wx;
        let by = scy.wrapping_add(ly) as usize;

        if ly == wy {
            self.window_y_triggered = true;
        }

        let bg_enabled = lcdc & (1 << 0) != 0;
        let map_start: usize = if lcdc & (1 << 3) != 0 { 0x1C00 } else { 0x1800 };
        let window_map: usize = if lcdc & (1 << 6) != 0 { 0x1C00 } else { 0x1800 };
        let unsigned_tiles = lcdc & (1 << 4) != 0;
        let window_flag = lcdc & (1 << 5) != 0;
        let window_drawn = self.window_y_triggered && window_flag && wx <= 166;

        for x in 0..Frame::WIDTH {
            let index = if bg_enabled {
                let (px, py, map) = if window_drawn && x + 7 >= wx as usize {
                    // Calculate the window column
                    (x + 7 - wx as usize, self.window_line_counter, window_map)
                } else {
                    // Calculate background column
                    let bx = scx.wrapping_add(x as u8) as usize;
                    (bx, by, map_start)
                };

                //  Read the tile number stored in the map.
                let tile_number = self.vram[map + (py / 8) * 32 + (px / 8)];

                // Find that tile's 16 bytes. 0x9000 is vram offset 0x1000.
                let tile_start = if unsigned_tiles {
                    tile_number as usize * 16
                } else {
                    (0x1000 + (tile_number as i8 as isize) * 16) as usize
                };

                // Two bytes per row: little endian
                let row = tile_start + (py % 8) * 2;
                let low = self.vram[row];
                let high = self.vram[row + 1];

                // Bit 7 is the leftmost pixel.
                let bit = 7 - (px % 8);
                (((high >> bit) & 1) << 1) | ((low >> bit) & 1)
            } else {
                0
            };

            // Map the color index through BGP
            let shade = Shade::from_bits(bgp >> (index * 2));

            self.back_frame.write_pixel(x, ly as usize, shade);
        }

        if window_drawn {
            self.window_line_counter += 1;
        }
    }
}
