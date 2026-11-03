use crate::{
    cpu::{CPU, CpuError, CpuSnapShot, CycleAction, Registers},
    memory::{Bus, Cartridge},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub enum SystemError {
    CpuRequestError(CpuError),
    CpuCompletionError(CpuError, CycleTrace),
}

#[derive(Debug, Clone, Copy)]
pub struct CycleTrace {
    pub tick_before: u64,
    pub tick_after: u64,
    pub action: CycleAction,
    pub read_value: Option<u8>,
    pub complete: bool,
}

pub struct System {
    memory: Bus,
    cpu: CPU,
    elapsed_ticks: u64,
}

impl System {
    pub fn new(cartridge: Cartridge) -> Self {
        let header_checksum = cartridge.rom()[0x014D];
        let registers = Registers::dmg_post_boot(header_checksum);
        let memory = Bus::new(cartridge);
        let cpu = CPU::from_registers(registers);

        System {
            memory,
            cpu,
            elapsed_ticks: 0,
        }
    }

    pub fn cpu_snapshot(&self) -> CpuSnapShot {
        self.cpu.snapshot()
    }

    pub fn step_machine_cycle(&mut self) -> Result<CycleTrace, SystemError> {
        let action = match self.cpu.request_cycle() {
            Ok(action) => action,
            Err(err) => return Err(SystemError::CpuRequestError(err)),
        };

        let mut trace = CycleTrace {
            tick_before: self.elapsed_ticks,
            tick_after: self.elapsed_ticks,
            action,
            read_value: None,
            complete: false,
        };

        self.elapsed_ticks += 4;

        match action {
            crate::cpu::CycleAction::Idle => {
                let complete = match self.cpu.complete_cycle(None) {
                    Ok(complete) => complete,
                    Err(err) => {
                        trace.tick_after = self.elapsed_ticks;
                        return Err(SystemError::CpuCompletionError(err, trace));
                    }
                };
                trace.complete = complete;
            }
            crate::cpu::CycleAction::Read { addr } => {
                let value = self.memory.read(addr);
                trace.read_value = Some(value);

                let complete = match self.cpu.complete_cycle(Some(value)) {
                    Ok(complete) => complete,
                    Err(err) => {
                        trace.tick_after = self.elapsed_ticks;
                        return Err(SystemError::CpuCompletionError(err, trace));
                    }
                };
                trace.complete = complete;
            }
            crate::cpu::CycleAction::Write { addr, value } => {
                self.memory.write(addr, value);
                let complete = match self.cpu.complete_cycle(None) {
                    Ok(complete) => complete,
                    Err(err) => {
                        trace.tick_after = self.elapsed_ticks;
                        return Err(SystemError::CpuCompletionError(err, trace));
                    }
                };
                trace.complete = complete;
            }
        }

        trace.tick_after = self.elapsed_ticks;

        Ok(trace)
    }

    pub fn elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks
    }
}
