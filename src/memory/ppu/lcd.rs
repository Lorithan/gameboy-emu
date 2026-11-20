use crate::memory::ppu::timing::PpuMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LcdRegisters {
    pub(super) scy: u8,
    pub(super) scx: u8,
    pub(super) lyc: u8,
    pub(super) bgp: u8,
    pub(super) obp0: u8,
    pub(super) obp1: u8,
    pub(super) wy: u8,
    pub(super) wx: u8,
    pub(super) lcd_control: LcdControl,
    pub(super) lcd_stat: LcdStatus,
}

impl LcdRegisters {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            scy: 0,
            scx: 0,
            lyc: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            lcd_control: LcdControl::from(0x91),
            lcd_stat: LcdStatus::dmg_post_boot(),
        }
    }

    pub(super) fn read_stat(&self, mode: PpuMode) -> u8 {
        self.lcd_stat.read(mode)
    }

    pub(super) fn write_stat(&mut self, value: u8) {
        self.lcd_stat.write(value);
    }

    pub(super) fn set_lyc_equal(&mut self, lyc_equal: bool) {
        self.lcd_stat.set_lyc_equal(lyc_equal);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LcdControl(u8);

impl From<u8> for LcdControl {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl LcdControl {
    pub(super) fn bits(&self) -> u8 {
        self.0
    }

    pub(super) fn write(&mut self, value: u8) {
        self.0 = value;
    }

    pub(super) fn is_enabled(&self) -> bool {
        self.0 & 0x80 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LcdStatus {
    interrupt_enable_bits: u8,
    lyc_equal: bool,
}

impl LcdStatus {
    pub(super) fn dmg_post_boot() -> Self {
        Self {
            interrupt_enable_bits: 0x00,
            lyc_equal: true,
        }
    }

    pub(super) fn write(&mut self, value: u8) {
        self.interrupt_enable_bits = value & 0x78;
    }

    pub(super) fn read(&self, mode: PpuMode) -> u8 {
        let interrupt_enabled_bits = self.interrupt_enable_bits;
        let bit2 = if self.lyc_equal { 0x04 } else { 0x00 };
        let stat_bits = mode.stat_bits();

        0x80 | interrupt_enabled_bits | bit2 | stat_bits
    }

    pub(super) fn set_lyc_equal(&mut self, lyc_equal: bool) {
        self.lyc_equal = lyc_equal;
    }
}
