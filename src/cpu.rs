mod alu;
mod cycles;
mod dispatch;
pub mod error;
mod execution;
mod opcode;
mod registers;
mod state;

#[cfg(test)]
mod tests;

pub use cycles::*;
pub use error::{CpuError, CpuFault, OpcodeEncoding, UnexpectedCycleResponse};
pub use registers::Registers;

use crate::cpu::{error::IllegalOpcode, opcode::*, registers::*, state::*};

#[derive(Debug, Clone, Copy)]
enum Destination {
    Register(Register),
    Memory(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapshot {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub pc: u16,
    pub sp: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterruptState {
    Disabled,
    Delayed,
    Enabled,
}

#[derive(Debug, Clone)]
pub struct CPU {
    registers: Registers,
    state: ExecutionState,
    interrupt_state: InterruptState,
}

impl CPU {
    pub fn from_registers(registers: Registers) -> Self {
        CPU {
            registers,
            state: ExecutionState::FetchOpcode {
                skip_increment: false,
            },
            interrupt_state: InterruptState::Disabled,
        }
    }

    pub fn snapshot(&self) -> CpuSnapshot {
        CpuSnapshot {
            af: self.registers.af(),
            bc: self.registers.bc(),
            de: self.registers.de(),
            hl: self.registers.hl(),
            pc: self.registers.pc,
            sp: self.registers.sp,
        }
    }

    pub fn request_cycle(&self) -> Result<CycleRequest, CpuError> {
        match self.state {
            ExecutionState::FetchOpcode { .. }
            | ExecutionState::DecodeCbOpcode
            | ExecutionState::ReadDisplacement { .. }
            | ExecutionState::ReadImmediate8 { .. }
            | ExecutionState::ReadImm16Low { .. }
            | ExecutionState::ReadImm16High { .. }
            | ExecutionState::ReadSpOffset { .. }
            | ExecutionState::ReadHighMemoryImmediate { .. }
            | ExecutionState::ReadImmediateAccumulator { .. } => {
                Ok(CycleRequest::Execute(CycleAction::Read {
                    addr: self.registers.pc,
                }))
            }
            ExecutionState::ReadMemory { addr, .. }
            | ExecutionState::ReadMemoryAndUpdateHL { addr, .. }
            | ExecutionState::PopLow { addr, .. }
            | ExecutionState::PopHigh { addr, .. }
            | ExecutionState::ReadReturnLow { addr, .. }
            | ExecutionState::ReadReturnHigh { addr, .. }
            | ExecutionState::ReadOperation { addr, .. }
            | ExecutionState::ReadHLForCbOpcode { addr, .. }
            | ExecutionState::ReadMemoryAccumulator { addr, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Read { addr }))
            }
            ExecutionState::WriteMemory { addr, value }
            | ExecutionState::WriteMemoryAndUpdateHL { addr, value, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write { addr, value }))
            }
            ExecutionState::WriteSpLow { sp_low, addr, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write {
                    addr,
                    value: sp_low,
                }))
            }
            ExecutionState::PushHigh { addr, high, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write {
                    addr,
                    value: high,
                }))
            }
            ExecutionState::PushLow { addr, low, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write {
                    addr,
                    value: low,
                }))
            }
            ExecutionState::InterruptPushHigh { pc_high, sp, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write {
                    addr: sp,
                    value: pc_high,
                }))
            }
            ExecutionState::InterruptPushLow { pc_low, sp, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write {
                    addr: sp,
                    value: pc_low,
                }))
            }
            ExecutionState::Push { .. }
            | ExecutionState::ApplyJump { .. }
            | ExecutionState::ReturnConditional { .. }
            | ExecutionState::FinishReturn { .. }
            | ExecutionState::Addition16 { .. }
            | ExecutionState::DelaySpOffset { .. }
            | ExecutionState::ApplySpOffset { .. }
            | ExecutionState::TransferHLtoSP { .. }
            | ExecutionState::InterruptEntry { .. }
            | ExecutionState::InterruptWait { .. }
            | ExecutionState::InterruptJump { .. }
            | ExecutionState::Halted
            | ExecutionState::Unary16 { .. } => Ok(CycleRequest::Execute(CycleAction::Idle)),
            ExecutionState::Stopped => Ok(CycleRequest::Stopped),
            ExecutionState::Faulted(err) => Err(CpuError::UnsupportedOpcode(err)),
        }
    }

    pub fn complete_cycle(&mut self, response: CycleResponse) -> Result<CycleOutcome, CpuError> {
        let enable_was_pending = self.interrupt_state == InterruptState::Delayed;

        let expected_kind = match self.request_cycle()? {
            CycleRequest::Execute(CycleAction::Idle) => CycleKind::Idle,
            CycleRequest::Execute(CycleAction::Read { .. }) => CycleKind::Read,
            CycleRequest::Execute(CycleAction::Write { .. }) => CycleKind::Write,
            CycleRequest::Stopped => return Err(CpuError::NoCycleRequested),
        };
        let actual_kind = response.kind();

        if expected_kind != actual_kind {
            return Err(CpuError::UnexpectedCycleResponse(UnexpectedCycleResponse {
                expected: expected_kind,
                received: actual_kind,
            }));
        }

        let outcome = match self.complete_cycle_inner(response)? {
            CycleTransition::Continue(next_state) => {
                self.state = next_state;
                Ok(CycleOutcome::InProgress)
            }
            CycleTransition::InstructionFinished => {
                self.state = ExecutionState::FetchOpcode {
                    skip_increment: false,
                };
                Ok(CycleOutcome::InstructionComplete)
            }
            CycleTransition::InterruptEntryFinished { interrupt } => {
                self.state = ExecutionState::FetchOpcode {
                    skip_increment: false,
                };
                Ok(CycleOutcome::InterruptEntryComplete {
                    vector: interrupt.vector_address(),
                })
            }
            CycleTransition::InterruptSelectionResolved { interrupt } => {
                self.state = ExecutionState::InterruptJump { interrupt };
                match interrupt {
                    InterruptSelection::Selected(interrupt) => {
                        Ok(CycleOutcome::InterruptSelected { interrupt })
                    }
                    InterruptSelection::Cancelled => Ok(CycleOutcome::InterruptSelectionCancelled),
                }
            }
            CycleTransition::InterruptEntryCancelled => {
                self.state = ExecutionState::FetchOpcode {
                    skip_increment: false,
                };
                Ok(CycleOutcome::InterruptEntryCancelled)
            }
            CycleTransition::HaltFinished(behavior) => {
                match behavior {
                    HaltBehavior::Sleep => {
                        self.state = ExecutionState::Halted;
                    }
                    HaltBehavior::SuppressNextIncrement => {
                        self.state = ExecutionState::FetchOpcode {
                            skip_increment: true,
                        };
                    }
                }
                Ok(CycleOutcome::InstructionComplete)
            }
            CycleTransition::StopFinished(behavior) => match behavior {
                StopBehavior::Stop { skip_padding } => {
                    if skip_padding {
                        self.registers.increment_pc();
                    }
                    self.state = ExecutionState::Stopped;
                    Ok(CycleOutcome::StopEntered)
                }
                StopBehavior::Halt => {
                    self.registers.increment_pc();
                    self.state = ExecutionState::Halted;
                    Ok(CycleOutcome::InstructionComplete)
                }
                StopBehavior::Continue => {
                    self.state = ExecutionState::FetchOpcode {
                        skip_increment: false,
                    };
                    Ok(CycleOutcome::InstructionComplete)
                }
            },
            CycleTransition::Faulted(err) => {
                self.state = ExecutionState::Faulted(err);
                Err(CpuError::UnsupportedOpcode(err))
            }
        }?;

        if matches!(
            outcome,
            CycleOutcome::InstructionComplete | CycleOutcome::StopEntered
        ) && enable_was_pending
            && self.interrupt_state == InterruptState::Delayed
        {
            self.interrupt_state = InterruptState::Enabled;
        }

        Ok(outcome)
    }

    pub(crate) fn try_accept_interrupt(&mut self, pending: u8) -> bool {
        if let ExecutionState::FetchOpcode { skip_increment } = self.state
            && self.interrupt_state == InterruptState::Enabled
        {
            let pending = pending & 0x1F; // Only the lower 5 bits are relevant

            if pending != 0 {
                if skip_increment {
                    self.registers.pc = self.registers.pc.wrapping_sub(1);
                }

                self.state = ExecutionState::InterruptEntry {
                    pc: self.registers.pc,
                    sp: self.registers.sp,
                };
                self.interrupt_state = InterruptState::Disabled;
                return true;
            }
        }

        false
    }

    pub(crate) fn wake_if_pending(&mut self, pending: u8) -> bool {
        let pending = pending & 0x1F;

        if matches!(self.state, ExecutionState::Halted) && pending != 0 {
            self.state = ExecutionState::FetchOpcode {
                skip_increment: false,
            };
            true
        } else {
            false
        }
    }

    pub(crate) fn wake_from_stop_if(&mut self, wake: bool) {
        if matches!(self.state, ExecutionState::Stopped) && wake {
            self.state = ExecutionState::FetchOpcode {
                skip_increment: false,
            };
        }
    }
}
