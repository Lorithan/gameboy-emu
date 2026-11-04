mod cartridge;

pub use cartridge::Cartridge;

#[cfg(test)]
mod tests;

pub struct Bus {
    cart: Cartridge,
    ram: [u8; 8 * 1024],
    hram: [u8; 0x7F],
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ram: [0; 8 * 1024],
            hram: [0; 0x7F],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                let rom = self.cart.rom();
                rom[addr as usize]
            }
            0xC000..=0xDFFF => self.ram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.ram[(addr - 0xE000) as usize],
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xC000..=0xDFFF => self.ram[(addr - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.ram[(addr - 0xE000) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            _ => {
                // Ignore writes
            }
        }
    }
}
