pub(super) struct LcdRegisters {
    pub(super) scy: u8,
    pub(super) scx: u8,
    pub(super) lyc: u8,
    pub(super) bgp: u8,
    pub(super) obp0: u8,
    pub(super) obp1: u8,
    pub(super) wy: u8,
    pub(super) wx: u8,
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
        }
    }
}
