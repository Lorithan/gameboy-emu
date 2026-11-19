pub(super) struct Ppu {
    vram: [u8; 0x2000],
    oam: [u8; 0xA0],
}

impl Ppu {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
        }
    }

    pub(super) fn read_vram(&self, offset: usize) -> u8 {
        self.vram[offset]
    }

    pub(super) fn write_vram(&mut self, offset: usize, value: u8) {
        self.vram[offset] = value;
    }

    pub(super) fn read_oam(&self, offset: usize) -> u8 {
        self.oam[offset]
    }

    pub(super) fn write_oam(&mut self, offset: usize, value: u8) {
        self.oam[offset] = value;
    }
}
