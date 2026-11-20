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
        }
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
