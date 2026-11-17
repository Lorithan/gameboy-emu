#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InterruptController {
    interrupt_enable: u8,
    interrupt_flag: u8,
}

impl InterruptController {
    pub fn post_boot() -> Self {
        Self {
            interrupt_enable: 0x00,
            interrupt_flag: 0x01,
        }
    }

    pub fn read_enable(&self) -> u8 {
        self.interrupt_enable
    }

    pub fn write_enable(&mut self, value: u8) {
        self.interrupt_enable = value;
    }

    pub fn read_flag(&self) -> u8 {
        self.interrupt_flag | 0xE0
    }

    pub fn write_flag(&mut self, value: u8) {
        self.interrupt_flag = value & 0x1F; // Only the lower 5 bits are writable
    }

    // Temporary: production callers arrive with interrupt entry and peripherals.
    #[allow(dead_code)]
    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        let bit = interrupt.bit_index();
        self.interrupt_flag |= 1 << bit;
    }

    // Temporary: production callers arrive with interrupt entry and peripherals.
    #[allow(dead_code)]
    pub fn acknowledge_interrupt(&mut self, interrupt: Interrupt) {
        let bit = interrupt.bit_index();
        self.interrupt_flag &= !(1 << bit);
    }

    // Temporary: production callers arrive with interrupt entry and peripherals.
    #[allow(dead_code)]
    pub fn pending_interrupts(&self) -> u8 {
        self.interrupt_enable & self.interrupt_flag & 0x1F
    }
}

// Temporary: production sources arrive with interrupt entry and peripherals.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interrupt {
    VBlank,
    LCDStat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    // Temporary: used by the staged interrupt request/acknowledgment hooks.
    #[allow(dead_code)]
    fn bit_index(self) -> u8 {
        match self {
            Interrupt::VBlank => 0,
            Interrupt::LCDStat => 1,
            Interrupt::Timer => 2,
            Interrupt::Serial => 3,
            Interrupt::Joypad => 4,
        }
    }

    pub(crate) fn vector_address(self) -> u16 {
        match self {
            Interrupt::VBlank => 0x0040,
            Interrupt::LCDStat => 0x0048,
            Interrupt::Timer => 0x0050,
            Interrupt::Serial => 0x0058,
            Interrupt::Joypad => 0x0060,
        }
    }
}
