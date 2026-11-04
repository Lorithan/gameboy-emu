#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Z,
    N,
    H,
    C,
}

impl Flag {
    const fn mask(self) -> u8 {
        match self {
            Flag::Z => 0x80,
            Flag::N => 0x40,
            Flag::H => 0x20,
            Flag::C => 0x10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flags(u8);

impl Flags {
    fn new(val: u8) -> Self {
        Flags(val & 0xF0)
    }

    fn get(self, flag: Flag) -> bool {
        self.0 & flag.mask() != 0
    }

    fn set(&mut self, flag: Flag, val: bool) {
        if val {
            self.0 |= flag.mask();
        } else {
            self.0 &= !flag.mask();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Register16 {
    BC,
    DE,
    HL,
    SP,
}

#[derive(Debug, Clone, Copy)]
pub struct Registers {
    pub(super) a: u8,
    pub(super) b: u8,
    pub(super) c: u8,
    pub(super) d: u8,
    pub(super) e: u8,
    f: Flags,
    pub(super) h: u8,
    pub(super) l: u8,
    /// Program counter
    pub(super) pc: u16,
    /// Stack Pointer
    pub(super) sp: u16,
}

impl Registers {
    pub fn dmg_post_boot(header_checksum: u8) -> Self {
        let flag = if header_checksum == 0 { 0x80 } else { 0xB0 };

        Registers {
            a: 0x01,
            b: 0,
            c: 0x13,
            d: 0,
            e: 0xD8,
            f: Flags::new(flag),
            h: 0x01,
            l: 0x4d,
            pc: 0x0100,
            sp: 0xFFFE,
        }
    }

    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | self.f.0 as u16
    }

    pub(super) fn set_af(&mut self, af: u16) {
        self.f = Flags::new(af as u8);
        self.a = ((af >> 8) & 0xFF) as u8;
    }

    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    pub(super) fn set_bc(&mut self, bc: u16) {
        self.c = (bc & 0xFF) as u8;
        self.b = ((bc >> 8) & 0xFF) as u8;
    }

    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    pub(super) fn set_de(&mut self, de: u16) {
        self.e = (de & 0xFF) as u8;
        self.d = ((de >> 8) & 0xFF) as u8;
    }

    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    pub(super) fn set_hl(&mut self, hl: u16) {
        self.l = (hl & 0xFF) as u8;
        self.h = ((hl >> 8) & 0xFF) as u8;
    }

    pub(super) fn set_pc(&mut self, pc: u16) {
        self.pc = pc;
    }

    pub(super) fn set_sp(&mut self, sp: u16) {
        self.sp = sp
    }

    pub fn get_flag(&self, flag: Flag) -> bool {
        self.f.get(flag)
    }

    pub fn set_flag(&mut self, flag: Flag, val: bool) {
        self.f.set(flag, val);
    }
}
