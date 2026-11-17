use crate::{
    cpu::{CPU, CpuError, CpuSnapShot, CycleAction, CycleOutcome, Registers},
    memory::{Bus, Cartridge},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub enum SystemError {
    CpuRequestError(CpuError),
    CpuCompletionError(CpuError, CycleRecord),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleRecord {
    pub tick_before: u64,
    pub tick_after: u64,
    pub action: CycleAction,
    pub read_value: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct CycleTrace {
    pub record: CycleRecord,
    pub outcome: CycleOutcome,
}

#[derive(Debug, Clone, Copy)]
pub enum InstructionStep {
    Complete {
        mcycles: u64,
        ticks: u64,
    },
    BudgetExhausted {
        mcycles: u64,
        ticks: u64,
    },
    InterruptEntered {
        mcycles: u64,
        ticks: u64,
        vector: u16,
    },
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
        // Handle any pending interrupts before executing the next CPU cycle.
        let pending_interrupts = self.memory.pending_interrupts();
        if let Some(interrupt) = self.cpu.try_accept_interrupt(pending_interrupts) {
            self.memory.acknowledge_interrupt(interrupt);
        }

        let action = match self.cpu.request_cycle() {
            Ok(action) => action,
            Err(err) => return Err(SystemError::CpuRequestError(err)),
        };

        let tick_before = self.elapsed_ticks;
        let mut read_value = None;

        self.elapsed_ticks += 4;

        let outcome = match action {
            crate::cpu::CycleAction::Idle => self.cpu.complete_cycle(None),
            crate::cpu::CycleAction::Read { addr } => {
                let value = self.memory.read(addr);
                read_value = Some(value);

                self.cpu.complete_cycle(Some(value))
            }
            crate::cpu::CycleAction::Write { addr, value } => {
                self.memory.write(addr, value);
                self.cpu.complete_cycle(None)
            }
        };

        let record = CycleRecord {
            tick_before,
            tick_after: self.elapsed_ticks,
            action,
            read_value,
        };

        match outcome {
            Ok(outcome) => {
                let trace = CycleTrace { record, outcome };
                Ok(trace)
            }
            Err(err) => Err(SystemError::CpuCompletionError(err, record)),
        }
    }

    pub fn step_instruction(&mut self, max_mcycles: u64) -> Result<InstructionStep, SystemError> {
        let starting_ticks = self.elapsed_ticks;

        for cycle in 0..max_mcycles {
            let trace = self.step_machine_cycle()?;

            match trace.outcome {
                CycleOutcome::InstructionComplete => {
                    return Ok(InstructionStep::Complete {
                        mcycles: cycle + 1,
                        ticks: self.elapsed_ticks - starting_ticks,
                    });
                }
                CycleOutcome::InterruptEntryComplete { vector } => {
                    return Ok(InstructionStep::InterruptEntered {
                        mcycles: cycle + 1,
                        ticks: self.elapsed_ticks - starting_ticks,
                        vector,
                    });
                }
                CycleOutcome::InProgress => {}
            }
        }

        Ok(InstructionStep::BudgetExhausted {
            mcycles: max_mcycles,
            ticks: self.elapsed_ticks - starting_ticks,
        })
    }

    pub fn elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks
    }
}
