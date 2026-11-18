use crate::{
    cpu::{
        CPU, CpuError, CpuSnapShot, CycleAction, CycleOutcome, CycleRequest, CycleResponse,
        InterruptSample, Registers,
    },
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
pub enum MachineStep {
    Cycle(CycleTrace),
    Stopped,
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
    InterruptEntryCancelled {
        mcycles: u64,
        ticks: u64,
    },
    Stopped {
        mcycles: u64,
        ticks: u64,
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

    pub fn step_machine_cycle(&mut self) -> Result<MachineStep, SystemError> {
        // Handle any pending interrupts before executing the next CPU cycle.
        let pending_interrupts = self.memory.pending_interrupts();
        self.cpu.wake_if_pending(pending_interrupts);
        self.cpu.try_accept_interrupt(pending_interrupts);

        let action = match self.cpu.request_cycle() {
            Ok(CycleRequest::Execute(action)) => action,
            Ok(CycleRequest::Stopped) => return Ok(MachineStep::Stopped),
            Err(err) => return Err(SystemError::CpuRequestError(err)),
        };

        let tick_before = self.elapsed_ticks;
        let mut read_value = None;

        self.elapsed_ticks += 4;

        let outcome = match action {
            crate::cpu::CycleAction::Idle => self.cpu.complete_cycle(CycleResponse::IdleDone),
            crate::cpu::CycleAction::Read { addr } => {
                let value = self.memory.read(addr);
                read_value = Some(value);

                self.cpu.complete_cycle(CycleResponse::Read {
                    value,
                    pending_interrupts: self.memory.pending_interrupts(),
                })
            }
            crate::cpu::CycleAction::Write { addr, value } => {
                let flag_before_bus = self.memory.read(0xFF0F);
                self.memory.write(addr, value);
                let enable_after_bus = self.memory.read(0xFFFF);
                let interrupt_sample = InterruptSample {
                    enable_after_bus,
                    flags_before_bus: flag_before_bus,
                };

                self.cpu
                    .complete_cycle(CycleResponse::WriteDone { interrupt_sample })
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
                if let CycleOutcome::InterruptSelected { interrupt } = outcome {
                    self.memory.acknowledge_interrupt(interrupt);
                }
                Ok(MachineStep::Cycle(trace))
            }
            Err(err) => Err(SystemError::CpuCompletionError(err, record)),
        }
    }

    pub fn step_instruction(&mut self, max_mcycles: u64) -> Result<InstructionStep, SystemError> {
        let starting_ticks = self.elapsed_ticks;

        for cycle in 0..max_mcycles {
            match self.step_machine_cycle()? {
                MachineStep::Cycle(cycle_trace) => match cycle_trace.outcome {
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
                    CycleOutcome::InterruptEntryCancelled => {
                        return Ok(InstructionStep::InterruptEntryCancelled {
                            mcycles: cycle + 1,
                            ticks: self.elapsed_ticks - starting_ticks,
                        });
                    }
                    CycleOutcome::InterruptSelected { .. }
                    | CycleOutcome::InProgress
                    | CycleOutcome::InterruptSelectionCancelled => {}
                },
                MachineStep::Stopped => {
                    return Ok(InstructionStep::Stopped {
                        mcycles: cycle,
                        ticks: self.elapsed_ticks - starting_ticks,
                    });
                }
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
