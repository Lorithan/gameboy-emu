mod cartridge;
mod interrupts;
mod joypad;
mod ppu;
mod timer;

pub use cartridge::Cartridge;
pub use interrupts::Interrupt;
use interrupts::InterruptController;
pub use joypad::Button;

use crate::memory::{
    joypad::{Joypad, JoypadEvent},
    ppu::Ppu,
    timer::Timer,
};

#[cfg(test)]
mod tests;

pub struct Bus {
    cart: Cartridge,
    ram: [u8; 8 * 1024],
    hram: [u8; 0x7F],
    interrupt_controller: InterruptController,
    timer: Timer,
    joypad: Joypad,
    ppu: Ppu,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ram: [0; 8 * 1024],
            hram: [0; 0x7F],
            interrupt_controller: InterruptController::post_boot(),
            timer: Timer::dmg_post_boot(),
            joypad: Joypad::dmg_post_boot(),
            ppu: Ppu::dmg_post_boot(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                let rom = self.cart.rom();
                rom[addr as usize]
            }
            0x8000..=0x9FFF => self.ppu.read_vram((addr as usize) - 0x8000),
            0xC000..=0xDFFF => self.ram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.ram[(addr - 0xE000) as usize],
            0xFE00..=0xFE9F => self.ppu.read_oam((addr as usize) - 0xFE00),
            0xFF00 => self.joypad.read(),
            0xFF04 => self.timer.read_div(),
            0xFF05 => self.timer.read_tima(),
            0xFF06 => self.timer.read_tma(),
            0xFF07 => self.timer.read_tac(),
            0xFF0F => self.interrupt_controller.read_flag(),
            0xFF42 => self.ppu.read_scy(),
            0xFF43 => self.ppu.read_scx(),
            0xFF45 => self.ppu.read_lyc(),
            0xFF47 => self.ppu.read_bgp(),
            0xFF48 => self.ppu.read_obp0(),
            0xFF49 => self.ppu.read_obp1(),
            0xFF4A => self.ppu.read_wy(),
            0xFF4B => self.ppu.read_wx(),
            0xFFFF => self.interrupt_controller.read_enable(),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => self.ppu.write_vram((addr as usize) - 0x8000, value),
            0xC000..=0xDFFF => self.ram[(addr - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.ram[(addr - 0xE000) as usize] = value,
            0xFE00..=0xFE9F => self.ppu.write_oam((addr as usize) - 0xFE00, value),
            0xFF00 => {
                if let JoypadEvent::FallingEdge = self.joypad.write(value) {
                    self.request_interrupt(Interrupt::Joypad);
                }
            }
            0xFF04 => self.timer.write_div(value),
            0xFF05 => self.timer.write_tima(value),
            0xFF06 => self.timer.write_tma(value),
            0xFF07 => self.timer.write_tac(value),
            0xFF0F => self.interrupt_controller.write_flag(value),
            0xFF42 => self.ppu.write_scy(value),
            0xFF43 => self.ppu.write_scx(value),
            0xFF45 => self.ppu.write_lyc(value),
            0xFF47 => self.ppu.write_bgp(value),
            0xFF48 => self.ppu.write_obp0(value),
            0xFF49 => self.ppu.write_obp1(value),
            0xFF4A => self.ppu.write_wy(value),
            0xFF4B => self.ppu.write_wx(value),
            0xFFFF => self.interrupt_controller.write_enable(value),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            _ => {
                // Ignore writes
            }
        }
    }

    pub(crate) fn request_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_controller.request_interrupt(interrupt);
    }

    pub(crate) fn acknowledge_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_controller.acknowledge_interrupt(interrupt);
    }

    pub(crate) fn pending_interrupts(&self) -> u8 {
        self.interrupt_controller.pending_interrupts()
    }

    pub(crate) fn advance_mcycle(&mut self) {
        for _ in 0..4 {
            if let timer::TimerEvent::InterruptRequested = self.timer.tick() {
                self.request_interrupt(Interrupt::Timer);
            }
        }
    }

    pub(crate) fn on_stop_entered(&mut self) {
        self.timer.on_stop_entered();
    }

    pub(crate) fn set_button(&mut self, button: Button, pressed: bool) {
        if let JoypadEvent::FallingEdge = self.joypad.set_button(button, pressed) {
            self.request_interrupt(Interrupt::Joypad);
        }
    }

    pub(crate) fn stop_wake_active(&self) -> bool {
        self.joypad.stop_wake_active()
    }
}
