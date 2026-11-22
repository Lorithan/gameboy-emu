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
    ppu::{Ppu, PpuEffects},
    timer::Timer,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DmaRunningTransfer {
    page: u8,
    next_index: u8,
}

impl DmaRunningTransfer {
    fn source_addr(self) -> u16 {
        ((self.page as u16) << 8) | self.next_index as u16
    }

    fn m_cycle(&mut self) -> (u16, u8) {
        let addr = self.source_addr();
        let index = self.next_index;
        self.next_index += 1;

        (addr, index)
    }

    fn finished(&self) -> bool {
        self.next_index == 160
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DmaPendingStart {
    page: u8,
    cycles_left: u8,
}

impl DmaPendingStart {
    fn m_cycle(&mut self) -> bool {
        self.cycles_left -= 1;
        self.cycles_left == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OamDma {
    register: u8,
    running_transfer: Option<DmaRunningTransfer>,
    pending_start: Option<DmaPendingStart>,
}

impl OamDma {
    fn dmg_post_boot() -> Self {
        Self {
            register: 0xFF,
            running_transfer: None,
            pending_start: None,
        }
    }

    /// Returns addr for source read and the index to write to.
    fn m_cycle(&mut self) -> Option<(u16, u8)> {
        if let Some(pending) = self.pending_start.as_mut() {
            let finished = pending.m_cycle();

            if finished {
                let new_transfer = DmaRunningTransfer {
                    page: pending.page,
                    next_index: 0,
                };

                self.pending_start = None;
                self.running_transfer = Some(new_transfer);
            }
        }
        if let Some(transfer) = self.running_transfer.as_mut() {
            if transfer.finished() {
                self.running_transfer = None;
            } else {
                return Some(transfer.m_cycle());
            }
        }

        None
    }
}

pub struct Bus {
    cart: Cartridge,
    ram: [u8; 8 * 1024],
    hram: [u8; 0x7F],
    interrupt_controller: InterruptController,
    timer: Timer,
    joypad: Joypad,
    ppu: Ppu,
    dma: OamDma,
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
            dma: OamDma::dmg_post_boot(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        if self.dma.running_transfer.is_some() && matches!(addr, 0xFE00..=0xFE9F) {
            return 0xFF;
        }

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
            0xFF40 => self.ppu.read_lcdc(),
            0xFF41 => self.ppu.read_stat(),
            0xFF42 => self.ppu.read_scy(),
            0xFF43 => self.ppu.read_scx(),
            0xFF44 => self.ppu.read_ly(),
            0xFF45 => self.ppu.read_lyc(),
            0xFF46 => self.dma.register,
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
        if self.dma.running_transfer.is_some() && matches!(addr, 0xFE00..=0xFE9F) {
            return;
        }

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
            0xFF40 => self.ppu.write_lcdc(value),
            0xFF41 => {
                let effects = self.ppu.write_stat(value);
                self.handle_ppu_effects(effects);
            }
            0xFF42 => self.ppu.write_scy(value),
            0xFF43 => self.ppu.write_scx(value),
            0xFF45 => {
                let effects = self.ppu.write_lyc(value);
                self.handle_ppu_effects(effects);
            }
            0xFF46 => {
                self.dma.register = value;
                let pending = DmaPendingStart {
                    page: value,
                    cycles_left: 2,
                };
                self.dma.pending_start = Some(pending);
            }
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

            let effects = self.ppu.tick();
            self.handle_ppu_effects(effects);
        }

        if let Some((addr, index)) = self.dma.m_cycle() {
            let value = self.read_for_dma(addr);

            self.ppu.write_oam_dma(index as usize, value);
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

    fn handle_ppu_effects(&mut self, effects: PpuEffects) {
        if effects.vblank {
            self.request_interrupt(Interrupt::VBlank);
        }
        if effects.lcd_stat {
            self.request_interrupt(Interrupt::LCDStat);
        }
    }

    fn read_for_dma(&self, mut addr: u16) -> u8 {
        if addr >= 0xE000 {
            addr -= 0x2000;
        }

        match addr {
            0x0000..=0x7FFF => {
                let rom = self.cart.rom();
                rom[addr as usize]
            }
            0x8000..=0x9FFF => self.ppu.read_vram_dma((addr as usize) - 0x8000),
            0xC000..=0xDFFF => self.ram[(addr - 0xC000) as usize],
            _ => 0xFF,
        }
    }

    /// Test-only opaque view of private PPU timing, compared for equality by
    /// System-level tests that cannot name the private PpuTiming type.
    #[cfg(test)]
    pub(crate) fn ppu_timing_probe(&self) -> String {
        self.ppu.timing_probe()
    }
}
