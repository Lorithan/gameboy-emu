#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flag(u8);

impl Flag {
    fn new(val: u8) -> Self {
        Flag(val & 0xF0)
    }

    fn z(&self) -> bool {
        self.0 & (1 << 7) > 0
    }

    fn set_z(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 7;
        } else {
            self.0 &= !(1 << 7);
        }
    }

    fn n(&self) -> bool {
        self.0 & (1 << 6) > 0
    }

    fn set_n(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 6;
        } else {
            self.0 &= !(1 << 6);
        }
    }

    fn h(&self) -> bool {
        self.0 & (1 << 5) > 0
    }

    fn set_h(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 5;
        } else {
            self.0 &= !(1 << 5);
        }
    }

    fn c(&self) -> bool {
        self.0 & (1 << 4) > 0
    }

    fn set_c(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 4;
        } else {
            self.0 &= !(1 << 4);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagValue {
    Z,
    N,
    H,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegisterValue {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WordValue {
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
    f: Flag,
    pub(super) h: u8,
    pub(super) l: u8,
    /// Programm counter
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
            f: Flag::new(flag),
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
        self.f = Flag::new(af as u8);
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

    pub fn set_pc(&mut self, pc: u16) {
        self.pc = pc;
    }

    pub fn set_sp(&mut self, sp: u16) {
        self.sp = sp
    }

    pub fn get_flag(&self, flag: FlagValue) -> bool {
        match flag {
            FlagValue::Z => self.f.z(),
            FlagValue::N => self.f.n(),
            FlagValue::H => self.f.h(),
            FlagValue::C => self.f.c(),
        }
    }

    pub fn set_flag(&mut self, flag: FlagValue, val: bool) {
        match flag {
            FlagValue::Z => self.f.set_z(val),
            FlagValue::N => self.f.set_n(val),
            FlagValue::H => self.f.set_h(val),
            FlagValue::C => self.f.set_c(val),
        }
    }
}
