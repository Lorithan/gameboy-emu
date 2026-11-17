mod cartridge;
mod interrupts;

pub use cartridge::Cartridge;
pub(crate) use interrupts::Interrupt;
use interrupts::InterruptController;

#[cfg(test)]
mod tests;

pub struct Bus {
    cart: Cartridge,
    ram: [u8; 8 * 1024],
    hram: [u8; 0x7F],
    interrupt_controller: InterruptController,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ram: [0; 8 * 1024],
            hram: [0; 0x7F],
            interrupt_controller: InterruptController::post_boot(),
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
            0xFF0F => self.interrupt_controller.read_flag(),
            0xFFFF => self.interrupt_controller.read_enable(),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xC000..=0xDFFF => self.ram[(addr - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.ram[(addr - 0xE000) as usize] = value,
            0xFF0F => self.interrupt_controller.write_flag(value),
            0xFFFF => self.interrupt_controller.write_enable(value),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            _ => {
                // Ignore writes
            }
        }
    }

    // Temporary: production callers arrive with interrupt entry and peripherals.
    #[allow(dead_code)]
    pub(crate) fn request_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_controller.request_interrupt(interrupt);
    }

    pub(crate) fn acknowledge_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_controller.acknowledge_interrupt(interrupt);
    }

    pub(crate) fn pending_interrupts(&self) -> u8 {
        self.interrupt_controller.pending_interrupts()
    }
}
