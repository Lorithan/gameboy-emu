mod cycles;
pub mod error;
mod opcode;
mod registers;
mod state;

#[cfg(test)]
mod tests;

use std::ops::{Shl, Shr};

pub use cycles::*;
pub use error::{CpuError, CpuFault, OpcodeEncoding, UnexpectedCycleResponse};
pub use registers::Registers;

use crate::{
    cpu::{
        opcode::*,
        registers::*,
        state::{
            CycleTransition, ExecutionState, HaltBehavior, InterruptSelection, ReturnKind,
            StopBehavior,
        },
    },
    memory::Interrupt,
};

#[derive(Debug, Clone, Copy)]
enum Destination {
    Register(Register),
    Memory(u16),
}

use state::ExecutionState::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapShot {
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
    suppress_next_opcode_increment: bool,
}

impl CPU {
    pub fn from_registers(registers: Registers) -> Self {
        CPU {
            registers,
            state: FetchOpcode,
            interrupt_state: InterruptState::Disabled,
            suppress_next_opcode_increment: false,
        }
    }

    pub fn snapshot(&self) -> CpuSnapShot {
        CpuSnapShot {
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
            FetchOpcode
            | DecodeCbOpcode
            | ReadAbsoluteLow
            | ReadAbsoluteHigh { .. }
            | StoreAToAbsoluteLow { .. }
            | StoreAToAbsoluteHigh { .. }
            | ReadDisplacement { .. }
            | ReadImmediate8 { .. }
            | ReadImmediate16Low { .. }
            | ReadJumpLow { .. }
            | ReadJumpHigh { .. }
            | ReadCallLow { .. }
            | ReadCallHigh { .. }
            | ReadImmediate16High { .. }
            | ReadSpOffset { .. }
            | ReadHighMemoryImmediate { .. }
            | ReadLowToStoreSp { .. }
            | ReadHighToStoreSp { .. }
            | ReadImmediateAccumulator { .. } => Ok(CycleRequest::Execute(CycleAction::Read {
                addr: self.registers.pc,
            })),
            ReadMemory { addr, .. }
            | ReadMemoryAndUpdateHL { addr, .. }
            | PopLow { addr, .. }
            | PopHigh { addr, .. }
            | ReadReturnLow { addr, .. }
            | ReadReturnHigh { addr, .. }
            | ReadOperation { addr, .. }
            | ReadHLForCbOpcode { addr, .. }
            | ReadMemoryAccumulator { addr, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Read { addr }))
            }
            WriteMemory { addr, value } | WriteMemoryAndUpdateHL { addr, value, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write { addr, value }))
            }
            WriteSpLow { sp_low, addr, .. } => Ok(CycleRequest::Execute(CycleAction::Write {
                addr,
                value: sp_low,
            })),
            PushHigh { addr, high, .. } => Ok(CycleRequest::Execute(CycleAction::Write {
                addr,
                value: high,
            })),
            PushLow { addr, low } => Ok(CycleRequest::Execute(CycleAction::Write {
                addr,
                value: low,
            })),
            PushCallerHigh {
                stack_addr,
                return_addr_high,
                ..
            } => Ok(CycleRequest::Execute(CycleAction::Write {
                addr: stack_addr,
                value: return_addr_high,
            })),
            PushCallerLow {
                stack_addr,
                return_addr_low,
                ..
            } => Ok(CycleRequest::Execute(CycleAction::Write {
                addr: stack_addr,
                value: return_addr_low,
            })),
            InterruptPushHigh { pc_high, sp, .. } => {
                Ok(CycleRequest::Execute(CycleAction::Write {
                    addr: sp,
                    value: pc_high,
                }))
            }
            InterruptPushLow { pc_low, sp, .. } => Ok(CycleRequest::Execute(CycleAction::Write {
                addr: sp,
                value: pc_low,
            })),
            Push { .. }
            | ApplyJump { .. }
            | Call { .. }
            | ReturnConditional { .. }
            | FinishReturn { .. }
            | Addition16 { .. }
            | DelaySpOffset { .. }
            | ApplySpOffset { .. }
            | TransferHLtoSP { .. }
            | InterruptEntry { .. }
            | InterruptWait { .. }
            | InterruptJump { .. }
            | Halted
            | Unary16 { .. } => Ok(CycleRequest::Execute(CycleAction::Idle)),
            Stopped => Ok(CycleRequest::Stopped),
            Faulted(err) => Err(CpuError::UnsupportedOpcode(err)),
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
                self.state = FetchOpcode;
                Ok(CycleOutcome::InstructionComplete)
            }
            CycleTransition::InterruptEntryFinished { interrupt } => {
                self.state = FetchOpcode;
                Ok(CycleOutcome::InterruptEntryComplete {
                    vector: interrupt.vector_address(),
                })
            }
            CycleTransition::InterruptSelectionResolved { interrupt } => {
                self.state = InterruptJump { interrupt };
                match interrupt {
                    InterruptSelection::Selected(interrupt) => {
                        Ok(CycleOutcome::InterruptSelected { interrupt })
                    }
                    InterruptSelection::Cancelled => Ok(CycleOutcome::InterruptSelectionCancelled),
                }
            }
            CycleTransition::InterruptEntryCancelled => {
                self.state = FetchOpcode;
                Ok(CycleOutcome::InterruptEntryCancelled)
            }
            CycleTransition::HaltFinished(behavior) => {
                match behavior {
                    HaltBehavior::Sleep => {
                        self.state = Halted;
                    }
                    HaltBehavior::SuppressNextIncrement => {
                        self.state = FetchOpcode;
                        self.suppress_next_opcode_increment = true;
                    }
                }
                Ok(CycleOutcome::InstructionComplete)
            }
            CycleTransition::StopFinished(behavior) => match behavior {
                StopBehavior::Stop { skip_padding } => {
                    if skip_padding {
                        self.registers.increment_pc();
                    }
                    self.state = Stopped;
                    Ok(CycleOutcome::StopEntered)
                }
                StopBehavior::Halt => {
                    self.registers.increment_pc();
                    self.state = Halted;
                    Ok(CycleOutcome::InstructionComplete)
                }
                StopBehavior::Continue => {
                    self.state = FetchOpcode;
                    Ok(CycleOutcome::InstructionComplete)
                }
            },
            CycleTransition::Faulted(err) => {
                self.state = Faulted(err);
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
        if matches!(self.state, FetchOpcode) && self.interrupt_state == InterruptState::Enabled {
            let pending = pending & 0x1F; // Only the lower 5 bits are relevant

            if pending != 0 {
                if self.suppress_next_opcode_increment {
                    self.registers.pc = self.registers.pc.wrapping_sub(1);
                    self.suppress_next_opcode_increment = false;
                }

                self.state = InterruptEntry {
                    pc: self.registers.pc,
                    sp: self.registers.sp,
                };
                self.interrupt_state = InterruptState::Disabled;
                return true;
            }
        }

        false
    }

    pub(crate) fn wake_if_pending(&mut self, pending: u8) {
        let pending = pending & 0x1F;

        if matches!(self.state, Halted) && pending != 0 {
            self.state = FetchOpcode;
        }
    }

    pub(crate) fn wake_from_stop_if(&mut self, wake: bool) {
        if matches!(self.state, Stopped) && wake {
            self.state = FetchOpcode;
        }
    }

    fn complete_cycle_inner(
        &mut self,
        response: CycleResponse,
    ) -> Result<CycleTransition, CpuError> {
        match self.state {
            FetchOpcode => self.read_opcode(
                response.read_value()?,
                response.pending_interrupts()?,
                response.stop_wake_active()?,
            ),
            DecodeCbOpcode => {
                let value = response.read_value()?;
                self.registers.increment_pc();

                let (target, cb_type) = match DecodedCbOpcode::try_from(value) {
                    Ok(DecodedCbOpcode { target, cb_type }) => (target, cb_type),
                    Err(_) => {
                        let fault = CpuFault {
                            opcode: OpcodeEncoding::Cb(value),
                            addr: self.registers.pc.wrapping_sub(2),
                        };
                        return Ok(CycleTransition::Faulted(fault));
                    }
                };

                match target {
                    Target::Register(register) => {
                        let value = self.get_register(register);

                        match cb_type {
                            CbType::Transform(operation) => {
                                let result = self.execute_cb_transform(value, operation);

                                self.set_register(register, result);
                                Ok(CycleTransition::InstructionFinished)
                            }
                            CbType::Bit(bit) => {
                                let zero = value & (1 << bit) == 0;
                                self.registers.set_flag(Flag::Z, zero);
                                self.registers.set_flag(Flag::H, true);
                                self.registers.set_flag(Flag::N, false);
                                Ok(CycleTransition::InstructionFinished)
                            }
                            CbType::Clear(bit) => {
                                let result = value & (!(1 << bit));

                                self.set_register(register, result);
                                Ok(CycleTransition::InstructionFinished)
                            }
                            CbType::Set(bit) => {
                                let result = value | (1 << bit);

                                self.set_register(register, result);
                                Ok(CycleTransition::InstructionFinished)
                            }
                        }
                    }
                    Target::IndirectHL => {
                        let addr = self.registers.hl();

                        Ok(CycleTransition::Continue(ReadHLForCbOpcode {
                            addr,
                            cb_type,
                        }))
                    }
                }
            }
            ReadHLForCbOpcode { addr, cb_type } => {
                let value = response.read_value()?;

                match cb_type {
                    CbType::Transform(operation) => {
                        let value = self.execute_cb_transform(value, operation);

                        let next_state = WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }
                    CbType::Bit(bit) => {
                        let zero = value & (1 << bit) == 0;
                        self.registers.set_flag(Flag::Z, zero);
                        self.registers.set_flag(Flag::H, true);
                        self.registers.set_flag(Flag::N, false);
                        Ok(CycleTransition::InstructionFinished)
                    }
                    CbType::Clear(bit) => {
                        let value = value & (!(1 << bit));

                        let next_state = WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }

                    CbType::Set(bit) => {
                        let value = value | (1 << bit);

                        let next_state = WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }
                }
            }
            ReadDisplacement { taken } => {
                let value = response.read_value()? as i8;
                self.registers.increment_pc();
                if taken {
                    let next_state = ExecutionState::ApplyJump {
                        target: self.registers.pc.wrapping_add_signed(value as i16),
                    };
                    Ok(CycleTransition::Continue(next_state))
                } else {
                    Ok(CycleTransition::InstructionFinished)
                }
            }
            ApplyJump { target } => {
                self.registers.set_pc(target);

                Ok(CycleTransition::InstructionFinished)
            }
            ReadImmediate8 { target } => {
                let value = response.read_value()?;
                match target {
                    Destination::Register(register) => self.store_immediate(register, value),
                    Destination::Memory(addr) => {
                        self.registers.increment_pc();
                        let next_state = ExecutionState::WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }
                }
            }
            ReadImmediate16Low { destination } => {
                let value = response.read_value()?;

                self.registers.increment_pc();
                let next_state = ReadImmediate16High {
                    destination,
                    low: value,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadImmediate16High { destination, low } => {
                self.store_immediate_word(destination, low, response.read_value()?)
            }
            ReadJumpLow { taken } => {
                let value = response.read_value()?;

                self.registers.increment_pc();
                let next_state = ReadJumpHigh { taken, low: value };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadJumpHigh { taken, low } => {
                let value = response.read_value()?;

                self.registers.increment_pc();

                if taken {
                    let next_state = ApplyJump {
                        target: ((value as u16) << 8) | low as u16,
                    };
                    Ok(CycleTransition::Continue(next_state))
                } else {
                    Ok(CycleTransition::InstructionFinished)
                }
            }
            ReadMemory { destination, .. } => {
                self.store_fetched(destination, response.read_value()?)
            }
            ReadMemoryAndUpdateHL { addr, operation } => {
                self.store_fetched(Register::A, response.read_value()?)?;
                let hl = match operation {
                    UnaryOperation::Increment => addr.wrapping_add(1),
                    UnaryOperation::Decrement => addr.wrapping_sub(1),
                };

                self.registers.set_hl(hl);
                Ok(CycleTransition::InstructionFinished)
            }
            ReadOperation { addr, operation } => {
                let mut value = response.read_value()?;
                value = match operation {
                    UnaryOperation::Increment => self.increment(value),
                    UnaryOperation::Decrement => self.decrement(value),
                };

                let next_state = WriteMemory { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadImmediateAccumulator {
                value: left,
                operation,
            } => {
                let right = response.read_value()?;

                self.execute_accumulator(operation, left, right);

                self.registers.increment_pc();
                Ok(CycleTransition::InstructionFinished)
            }
            ReadMemoryAccumulator {
                value: left,
                operation,
                ..
            } => {
                let right = response.read_value()?;

                self.execute_accumulator(operation, left, right);

                Ok(CycleTransition::InstructionFinished)
            }
            WriteMemory { .. } => Ok(CycleTransition::InstructionFinished),
            WriteMemoryAndUpdateHL {
                addr, operation, ..
            } => {
                let hl = match operation {
                    UnaryOperation::Increment => addr.wrapping_add(1),
                    UnaryOperation::Decrement => addr.wrapping_sub(1),
                };

                self.registers.set_hl(hl);
                Ok(CycleTransition::InstructionFinished)
            }
            Push { addr, value } => {
                let high = (value >> 8) as u8;
                let low = (value & 0xFF) as u8;
                let next_state = PushHigh { addr, high, low };
                Ok(CycleTransition::Continue(next_state))
            }
            PushHigh { addr, low, .. } => {
                self.registers.set_sp(addr);
                let next_state = PushLow {
                    addr: self.registers.sp.wrapping_sub(1),
                    low,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            PushLow { addr, .. } => {
                self.registers.set_sp(addr);
                Ok(CycleTransition::InstructionFinished)
            }
            PopLow { addr, target } => {
                let low = response.read_value()?;

                self.registers.sp = addr.wrapping_add(1);
                let next_state = PopHigh {
                    addr: self.registers.sp,
                    low,
                    target,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            PopHigh { addr, low, target } => {
                let high = response.read_value()?;

                let value = (high as u16) << 8 | low as u16;

                match target {
                    PushOperand::BC => self.registers.set_bc(value),
                    PushOperand::DE => self.registers.set_de(value),
                    PushOperand::HL => self.registers.set_hl(value),
                    PushOperand::AF => self.registers.set_af(value),
                }

                self.registers.sp = addr.wrapping_add(1);
                Ok(CycleTransition::InstructionFinished)
            }
            ReadCallLow { taken, stack_addr } => {
                let low = response.read_value()?;

                self.registers.increment_pc();
                let next_state = ReadCallHigh {
                    taken,
                    low,
                    stack_addr,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadCallHigh {
                taken,
                low,
                stack_addr,
            } => {
                let high = response.read_value()?;
                self.registers.increment_pc();

                let jump_addr = (high as u16) << 8 | low as u16;

                if taken {
                    let return_addr = self.registers.pc;

                    let next_state = Call {
                        jump_addr,
                        stack_addr,
                        return_addr_high: (return_addr >> 8) as u8,
                        return_addr_low: (return_addr & 0xFF) as u8,
                    };

                    Ok(CycleTransition::Continue(next_state))
                } else {
                    Ok(CycleTransition::InstructionFinished)
                }
            }
            Call {
                jump_addr,
                stack_addr,
                return_addr_high,
                return_addr_low,
            } => {
                let next_state = PushCallerHigh {
                    jump_addr,
                    stack_addr,
                    return_addr_high,
                    return_addr_low,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            PushCallerHigh {
                jump_addr,
                stack_addr,
                return_addr_low,
                ..
            } => {
                self.registers.set_sp(stack_addr);
                let next_state = PushCallerLow {
                    jump_addr,
                    stack_addr: self.registers.sp.wrapping_sub(1),
                    return_addr_low,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            PushCallerLow {
                jump_addr,
                stack_addr,
                ..
            } => {
                self.registers.set_sp(stack_addr);
                self.registers.set_pc(jump_addr);

                Ok(CycleTransition::InstructionFinished)
            }
            ReadReturnLow { addr, kind } => {
                let low = response.read_value()?;
                self.registers.set_sp(addr.wrapping_add(1));

                let next_state = ReadReturnHigh {
                    addr: addr.wrapping_add(1),
                    low,
                    kind,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadReturnHigh { addr, low, kind } => {
                let high = response.read_value()?;
                self.registers.set_sp(addr.wrapping_add(1));

                let jump_addr = (high as u16) << 8 | low as u16;
                let next_state = FinishReturn {
                    target: jump_addr,
                    kind,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            FinishReturn { target: addr, kind } => {
                self.registers.set_pc(addr);
                if matches!(kind, ReturnKind::Interrupt) {
                    self.interrupt_state = InterruptState::Enabled;
                }
                Ok(CycleTransition::InstructionFinished)
            }
            ReturnConditional { taken, addr } => {
                if taken {
                    let next_state = ReadReturnLow {
                        addr,
                        kind: ReturnKind::Normal,
                    };
                    Ok(CycleTransition::Continue(next_state))
                } else {
                    Ok(CycleTransition::InstructionFinished)
                }
            }
            Unary16 { target, value } => {
                match target {
                    Register16::BC => self.registers.set_bc(value),
                    Register16::DE => self.registers.set_de(value),
                    Register16::HL => self.registers.set_hl(value),
                    Register16::SP => self.registers.set_sp(value),
                }

                Ok(CycleTransition::InstructionFinished)
            }
            Addition16 { lhs, rhs } => {
                let (result, c) = lhs.overflowing_add(rhs);

                let h = ((lhs ^ rhs ^ result) & 0x1000) != 0;

                self.registers.set_flag(registers::Flag::C, c);
                self.registers.set_flag(registers::Flag::N, false);
                self.registers.set_flag(registers::Flag::H, h);

                self.registers.set_hl(result);

                Ok(CycleTransition::InstructionFinished)
            }
            ReadSpOffset { sp, hl } => {
                let offset = response.read_value()? as i8;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let next_state = if hl {
                    ApplySpOffset { sp, offset, hl }
                } else {
                    DelaySpOffset { sp, offset, hl }
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DelaySpOffset { sp, offset, hl } => {
                let next_state = ApplySpOffset { sp, offset, hl };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadAbsoluteLow => {
                let low = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let next_state = ReadAbsoluteHigh { low };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadAbsoluteHigh { low } => {
                let high = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let addr = ((high as u16) << 8) | low as u16;

                let next_state = ReadMemory {
                    addr,
                    destination: Register::A,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            StoreAToAbsoluteLow { value } => {
                let low = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let next_state = StoreAToAbsoluteHigh { value, low };
                Ok(CycleTransition::Continue(next_state))
            }
            StoreAToAbsoluteHigh { value, low } => {
                let high = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let addr = ((high as u16) << 8) | low as u16;

                let next_state = WriteMemory { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            ApplySpOffset { sp, offset, hl } => {
                let result = sp.wrapping_add_signed(offset as i16);

                // flags
                self.registers.set_flag(Flag::Z, false);
                self.registers.set_flag(Flag::N, false);

                let raw_offset = u16::from(offset as u8);
                let h = (sp & 0x0F) + (raw_offset & 0x0F) > 0x0F;
                let c = (sp & 0xFF) + raw_offset > 0xFF;
                self.registers.set_flag(Flag::H, h);
                self.registers.set_flag(Flag::C, c);

                if hl {
                    self.registers.set_hl(result);
                } else {
                    self.registers.set_sp(result);
                }
                Ok(CycleTransition::InstructionFinished)
            }
            ReadHighMemoryImmediate { store } => {
                let low = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let addr = (0xFF00) | low as u16;

                let next_state = match store {
                    Some(value) => WriteMemory { addr, value },
                    None => ReadMemory {
                        addr,
                        destination: Register::A,
                    },
                };
                Ok(CycleTransition::Continue(next_state))
            }
            TransferHLtoSP { hl } => {
                self.registers.set_sp(hl);

                Ok(CycleTransition::InstructionFinished)
            }
            ReadLowToStoreSp { sp } => {
                let low = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let next_state = ReadHighToStoreSp { sp, low };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadHighToStoreSp { sp, low } => {
                let high = response.read_value()?;

                self.registers.pc = self.registers.pc.wrapping_add(1);

                let addr = (high as u16) << 8 | low as u16;
                let sp_low = (sp & 0xFF) as u8;
                let sp_high = (sp >> 8) as u8;

                let next_state = WriteSpLow {
                    sp_low,
                    sp_high,
                    addr,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            WriteSpLow { sp_high, addr, .. } => {
                let addr = addr.wrapping_add(1);
                let next_state = WriteMemory {
                    addr,
                    value: sp_high,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            InterruptEntry { pc, sp } => {
                let next_state = InterruptWait { pc, sp };
                Ok(CycleTransition::Continue(next_state))
            }
            InterruptWait { pc, sp } => {
                let next_state = InterruptPushHigh {
                    pc_high: (pc >> 8) as u8,
                    pc_low: (pc & 0xFF) as u8,
                    sp: sp.wrapping_sub(1),
                };
                Ok(CycleTransition::Continue(next_state))
            }
            InterruptPushHigh { pc_low, sp, .. } => {
                let interrupt_selection = response.interrupt_sample()?;

                self.registers.set_sp(sp);
                let next_state = InterruptPushLow {
                    pc_low,
                    sp: sp.wrapping_sub(1),
                    captured_enable: interrupt_selection.enable_after_bus,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            InterruptPushLow {
                sp,
                captured_enable,
                ..
            } => {
                let sample = response.interrupt_sample()?;
                let selection = captured_enable & sample.flags_before_bus & 0x1F;

                self.registers.set_sp(sp);
                let interrupt = match Interrupt::from_selection(selection) {
                    Some(interrupt) => InterruptSelection::Selected(interrupt),
                    None => InterruptSelection::Cancelled,
                };

                Ok(CycleTransition::InterruptSelectionResolved { interrupt })
            }
            InterruptJump { interrupt } => match interrupt {
                InterruptSelection::Selected(interrupt) => {
                    self.registers.set_pc(interrupt.vector_address());
                    Ok(CycleTransition::InterruptEntryFinished { interrupt })
                }
                InterruptSelection::Cancelled => {
                    self.registers.set_pc(0);
                    Ok(CycleTransition::InterruptEntryCancelled)
                }
            },
            Halted => Ok(CycleTransition::Continue(Halted)),
            Stopped => Err(CpuError::NoCycleRequested),
            Faulted(err) => Err(CpuError::UnsupportedOpcode(err)),
        }
    }

    fn read_opcode(
        &mut self,
        response: u8,
        pending: u8,
        stop_wake_active: bool,
    ) -> Result<CycleTransition, CpuError> {
        let opcode = DecodedOpcode::from(response);

        let op_addr = self.registers.pc;
        if self.suppress_next_opcode_increment {
            self.suppress_next_opcode_increment = false;
        } else {
            self.registers.increment_pc();
        }

        match opcode {
            DecodedOpcode::Nop => Ok(CycleTransition::InstructionFinished),
            DecodedOpcode::Load {
                destination,
                source,
            } => match source {
                Target::Register(register) => {
                    let value = self.get_register(register);

                    match destination {
                        Target::Register(register) => {
                            self.set_register(register, value);
                            Ok(CycleTransition::InstructionFinished)
                        }
                        Target::IndirectHL => {
                            let next_state = WriteMemory {
                                addr: self.registers.hl(),
                                value,
                            };
                            Ok(CycleTransition::Continue(next_state))
                        }
                    }
                }
                Target::IndirectHL => {
                    match destination {
                        Target::Register(register) => {
                            let next_state = ReadMemory {
                                addr: self.registers.hl(),
                                destination: register,
                            };
                            Ok(CycleTransition::Continue(next_state))
                        }

                        Target::IndirectHL => unreachable!(), // Instead of LD (HL),(HL) 0x76 resolves to HALT
                    }
                }
            },
            DecodedOpcode::LoadImmediate { destination } => {
                let target = match destination {
                    Target::Register(register) => Destination::Register(register),
                    Target::IndirectHL => Destination::Memory(self.registers.hl()),
                };
                let next_state = ReadImmediate8 { target };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadImmediate16 { destination } => {
                let next_state = ReadImmediate16Low { destination };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Unary { operation, target } => match target {
                Target::Register(register) => {
                    let mut value = self.get_register(register);
                    value = match operation {
                        UnaryOperation::Increment => self.increment(value),
                        UnaryOperation::Decrement => self.decrement(value),
                    };
                    self.set_register(register, value);

                    Ok(CycleTransition::InstructionFinished)
                }
                Target::IndirectHL => {
                    let next_state = ReadOperation {
                        addr: self.registers.hl(),
                        operation,
                    };
                    Ok(CycleTransition::Continue(next_state))
                }
            },
            DecodedOpcode::Unary16 { operation, target } => {
                let mut value = match target {
                    Register16::BC => self.registers.bc(),
                    Register16::DE => self.registers.de(),
                    Register16::HL => self.registers.hl(),
                    Register16::SP => self.registers.sp,
                };

                value = match operation {
                    UnaryOperation::Increment => value.wrapping_add(1),
                    UnaryOperation::Decrement => value.wrapping_sub(1),
                };

                let next_state = Unary16 { target, value };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Arithmetic { kind, operand } => {
                let operation = match kind {
                    ArithmeticKind::Add { carry } => ArithmeticKind::Add {
                        carry: carry && self.registers.get_flag(Flag::C),
                    },
                    ArithmeticKind::Sub { carry } => ArithmeticKind::Sub {
                        carry: carry && self.registers.get_flag(Flag::C),
                    },
                    _ => kind,
                };

                match operand {
                    Operand::Target(Target::Register(register)) => {
                        let left = self.get_register(Register::A);
                        let right = self.get_register(register);

                        self.execute_accumulator(operation, left, right);

                        Ok(CycleTransition::InstructionFinished)
                    }
                    Operand::Target(Target::IndirectHL) => {
                        let value = self.get_register(Register::A);
                        let next_state = ReadMemoryAccumulator {
                            addr: self.registers.hl(),
                            value,
                            operation,
                        };
                        Ok(CycleTransition::Continue(next_state))
                    }
                    Operand::Immediate => {
                        let value = self.get_register(Register::A);
                        let next_state = ReadImmediateAccumulator { value, operation };
                        Ok(CycleTransition::Continue(next_state))
                    }
                }
            }
            DecodedOpcode::AddHL { target } => {
                let lhs = self.registers.hl();
                let rhs = match target {
                    Register16::BC => self.registers.bc(),
                    Register16::DE => self.registers.de(),
                    Register16::HL => self.registers.hl(),
                    Register16::SP => self.registers.sp,
                };

                let next_state = Addition16 { lhs, rhs };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::ComplementAccumulator => {
                let value = !self.get_register(Register::A);
                self.set_register(Register::A, value);
                self.registers.set_flag(Flag::N, true);
                self.registers.set_flag(Flag::H, true);
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::SetCarry => {
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::C, true);
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::ComplementCarry => {
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers
                    .set_flag(Flag::C, !self.registers.get_flag(Flag::C));
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::Rotate { direction } => {
                let value = self.get_register(Register::A);
                let (result, carry_out) = self.rotate(value, direction);

                self.set_register(Register::A, result);
                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, false);
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::RelativeJump { condition } => {
                let taken = condition.jump(self.registers.f);

                let next_state = ReadDisplacement { taken };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::AbsoluteJump { condition } => {
                let taken = condition.jump(self.registers.f);

                let next_state = ReadJumpLow { taken };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::JumpHL => {
                self.registers.set_pc(self.registers.hl());
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::Push { operand } => {
                let addr = self.registers.sp.wrapping_sub(1);
                let value = match operand {
                    PushOperand::BC => self.registers.bc(),
                    PushOperand::DE => self.registers.de(),
                    PushOperand::HL => self.registers.hl(),
                    PushOperand::AF => self.registers.af(),
                };

                let next_state = Push { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Pop { operand } => {
                let addr = self.registers.sp;
                let next_state = PopLow {
                    addr,
                    target: operand,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Call { condition } => {
                let taken = condition.jump(self.registers.f);
                let stack_addr = self.registers.sp.wrapping_sub(1);

                let next_state = ReadCallLow { taken, stack_addr };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Return => {
                let next_state = ReadReturnLow {
                    addr: self.registers.sp,
                    kind: ReturnKind::Normal,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::ConditionalReturn { condition } => {
                let taken = condition.jump(self.registers.f);

                let next_state = ReturnConditional {
                    taken,
                    addr: self.registers.sp,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Restart { target } => {
                let return_addr = self.registers.pc;
                let return_addr_high = (return_addr >> 8) as u8;
                let return_addr_low = (return_addr & 0xFF) as u8;
                let next_state = Call {
                    jump_addr: target,
                    stack_addr: self.registers.sp.wrapping_sub(1),
                    return_addr_high,
                    return_addr_low,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::DecimalAdjust => {
                let mut a = self.get_register(Register::A);
                if self.registers.get_flag(Flag::N) {
                    // after subtraction
                    if self.registers.get_flag(Flag::C) {
                        a = a.wrapping_sub(0x60);
                    }
                    if self.registers.get_flag(Flag::H) {
                        a = a.wrapping_sub(0x6);
                    }
                } else {
                    // after addition
                    if self.registers.get_flag(Flag::C) || a > 0x99 {
                        a = a.wrapping_add(0x60);
                        self.registers.set_flag(Flag::C, true);
                    }
                    if self.registers.get_flag(Flag::H) || (a & 0x0F > 0x09) {
                        a = a.wrapping_add(0x6);
                    }
                }

                self.set_register(Register::A, a);
                self.registers.set_flag(Flag::Z, a == 0);
                self.registers.set_flag(Flag::H, false);
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::LoadHlFromSpOffset => {
                let next_state = ReadSpOffset {
                    sp: self.registers.sp,
                    hl: true,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::AddSignedOffsetToSp => {
                let next_state = ReadSpOffset {
                    sp: self.registers.sp,
                    hl: false,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadAFromIndirect { source } => {
                let addr = match source {
                    IndirectPair::BC => self.registers.bc(),
                    IndirectPair::DE => self.registers.de(),
                };

                let next_state = ReadMemory {
                    addr,
                    destination: Register::A,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::StoreAToIndirect { destination } => {
                let value = self.registers.a;

                let addr = match destination {
                    IndirectPair::BC => self.registers.bc(),
                    IndirectPair::DE => self.registers.de(),
                };

                let next_state = WriteMemory { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadAFromHL { operation } => {
                let addr = self.registers.hl();

                let next_state = ReadMemoryAndUpdateHL { addr, operation };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::StoreAToHL { operation } => {
                let value = self.registers.a;
                let addr = self.registers.hl();

                let next_state = WriteMemoryAndUpdateHL {
                    addr,
                    value,
                    operation,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadAFromAbsolute => Ok(CycleTransition::Continue(ReadAbsoluteLow)),
            DecodedOpcode::StoreAToAbsolute => {
                let value = self.registers.a;

                let next_state = StoreAToAbsoluteLow { value };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadHighMemory {
                address_source: destination,
            } => match destination {
                HighMemoryAddressSource::C => {
                    let addr = (0xFF00) | self.registers.c as u16;
                    let next_state = ReadMemory {
                        addr,
                        destination: Register::A,
                    };
                    Ok(CycleTransition::Continue(next_state))
                }
                HighMemoryAddressSource::Immediate => {
                    let next_state = ReadHighMemoryImmediate { store: None };
                    Ok(CycleTransition::Continue(next_state))
                }
            },
            DecodedOpcode::StoreHighMemory {
                address_source: source,
            } => match source {
                HighMemoryAddressSource::C => {
                    let addr = (0xFF00) | self.registers.c as u16;
                    let value = self.registers.a;

                    let next_state = WriteMemory { addr, value };
                    Ok(CycleTransition::Continue(next_state))
                }
                HighMemoryAddressSource::Immediate => {
                    let value = self.registers.a;

                    let next_state = ReadHighMemoryImmediate { store: Some(value) };
                    Ok(CycleTransition::Continue(next_state))
                }
            },
            DecodedOpcode::TransferHLtoSP => {
                let hl = self.registers.hl();
                let next_state = TransferHLtoSP { hl };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::StoreStackPointer => {
                let sp = self.registers.sp;
                let next_state = ReadLowToStoreSp { sp };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::DecodeCbOpcode => Ok(CycleTransition::Continue(DecodeCbOpcode)),
            DecodedOpcode::DisableInterrupts => {
                self.interrupt_state = InterruptState::Disabled;
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::EnabledInterrupts => {
                if self.interrupt_state == InterruptState::Disabled {
                    self.interrupt_state = InterruptState::Delayed;
                }

                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::ReturnFromInterrupt => {
                let next_state = ReadReturnLow {
                    addr: self.registers.sp,
                    kind: ReturnKind::Interrupt,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Halt => {
                let behavior = if pending & 0x1F == 0
                    || matches!(self.interrupt_state, InterruptState::Enabled)
                {
                    HaltBehavior::Sleep
                } else {
                    HaltBehavior::SuppressNextIncrement
                };

                Ok(CycleTransition::HaltFinished(behavior))
            }
            DecodedOpcode::Stop => {
                let interrupt_pending = pending & 0x1F != 0;
                let behavior = match (stop_wake_active, interrupt_pending) {
                    (false, pending) => StopBehavior::Stop {
                        skip_padding: !pending,
                    },
                    (true, false) => StopBehavior::Halt,
                    (true, true) => StopBehavior::Continue,
                };
                Ok(CycleTransition::StopFinished(behavior))
            }
            DecodedOpcode::Unknown => {
                let fault = CpuFault {
                    opcode: OpcodeEncoding::Base(response),
                    addr: op_addr,
                };
                Ok(CycleTransition::Faulted(fault))
            }
        }
    }

    fn store_immediate(
        &mut self,
        register: Register,
        value: u8,
    ) -> Result<CycleTransition, CpuError> {
        self.set_register(register, value);

        self.registers.increment_pc();
        Ok(CycleTransition::InstructionFinished)
    }

    fn store_fetched(
        &mut self,
        destination: Register,
        value: u8,
    ) -> Result<CycleTransition, CpuError> {
        self.set_register(destination, value);

        Ok(CycleTransition::InstructionFinished)
    }

    fn set_register(&mut self, destination: Register, value: u8) {
        match destination {
            Register::A => self.registers.a = value,
            Register::B => self.registers.b = value,
            Register::C => self.registers.c = value,
            Register::D => self.registers.d = value,
            Register::E => self.registers.e = value,
            Register::H => self.registers.h = value,
            Register::L => self.registers.l = value,
        }
    }

    fn get_register(&self, register: Register) -> u8 {
        match register {
            Register::A => self.registers.a,
            Register::B => self.registers.b,
            Register::C => self.registers.c,
            Register::D => self.registers.d,
            Register::E => self.registers.e,
            Register::H => self.registers.h,
            Register::L => self.registers.l,
        }
    }

    fn store_immediate_word(
        &mut self,
        destination: Register16,
        low: u8,
        value: u8,
    ) -> Result<CycleTransition, CpuError> {
        let value = (value as u16) << 8 | low as u16;

        match destination {
            Register16::BC => self.registers.set_bc(value),
            Register16::DE => self.registers.set_de(value),
            Register16::HL => self.registers.set_hl(value),
            Register16::SP => self.registers.set_sp(value),
        }

        self.registers.increment_pc();
        Ok(CycleTransition::InstructionFinished)
    }

    fn increment(&mut self, mut value: u8) -> u8 {
        let h = (value & 0x0F) == 0x0F;

        value = value.wrapping_add(1);

        let z = value == 0;
        self.registers.set_flag(registers::Flag::Z, z);
        self.registers.set_flag(registers::Flag::N, false);
        self.registers.set_flag(registers::Flag::H, h);

        value
    }

    fn decrement(&mut self, mut value: u8) -> u8 {
        let h = (value & 0x0F) == 0x00;

        value = value.wrapping_sub(1);

        let z = value == 0;
        self.registers.set_flag(registers::Flag::Z, z);
        self.registers.set_flag(registers::Flag::N, true);
        self.registers.set_flag(registers::Flag::H, h);

        value
    }

    fn add(&mut self, left: u8, right: u8, carry: bool) -> u8 {
        let (mut result, mut c) = left.overflowing_add(right);
        if carry {
            let (result2, c2) = result.overflowing_add(1);
            result = result2;
            c = c || c2;
        }
        let z = result == 0;
        let h = ((left ^ right ^ result) & 0x10) != 0;

        self.registers.set_flag(registers::Flag::C, c);
        self.registers.set_flag(registers::Flag::Z, z);
        self.registers.set_flag(registers::Flag::N, false);
        self.registers.set_flag(registers::Flag::H, h);

        result
    }

    fn sub(&mut self, left: u8, right: u8, carry: bool) -> u8 {
        let (mut result, mut c) = left.overflowing_sub(right);
        if carry {
            let (result2, c2) = result.overflowing_sub(1);
            result = result2;
            c = c || c2;
        }
        let z = result == 0;
        let h = ((left ^ right ^ result) & 0x10) != 0;

        self.registers.set_flag(registers::Flag::C, c);
        self.registers.set_flag(registers::Flag::Z, z);
        self.registers.set_flag(registers::Flag::N, true);
        self.registers.set_flag(registers::Flag::H, h);

        result
    }

    fn execute_accumulator(&mut self, operation: ArithmeticKind, left: u8, right: u8) {
        match operation {
            ArithmeticKind::Add { carry } => {
                let result = self.add(left, right, carry);
                self.set_register(Register::A, result);
            }
            ArithmeticKind::Sub { carry } => {
                let result = self.sub(left, right, carry);
                self.set_register(Register::A, result);
            }
            ArithmeticKind::And => {
                let result = left & right;
                self.set_register(Register::A, result);
                self.registers.set_flag(registers::Flag::Z, result == 0);
                self.registers.set_flag(registers::Flag::N, false);
                self.registers.set_flag(registers::Flag::H, true);
                self.registers.set_flag(registers::Flag::C, false);
            }
            ArithmeticKind::Xor => {
                let result = left ^ right;
                self.set_register(Register::A, result);
                self.registers.set_flag(registers::Flag::Z, result == 0);
                self.registers.set_flag(registers::Flag::N, false);
                self.registers.set_flag(registers::Flag::H, false);
                self.registers.set_flag(registers::Flag::C, false);
            }
            ArithmeticKind::Or => {
                let result = left | right;
                self.set_register(Register::A, result);
                self.registers.set_flag(registers::Flag::Z, result == 0);
                self.registers.set_flag(registers::Flag::N, false);
                self.registers.set_flag(registers::Flag::H, false);
                self.registers.set_flag(registers::Flag::C, false);
            }
            ArithmeticKind::Compare => {
                self.sub(left, right, false);
            }
        }
    }

    fn rotate(&self, value: u8, direction: RotationDirection) -> (u8, bool) {
        let carry_in = self.registers.get_flag(Flag::C);

        let (result, carry_out) = match direction {
            RotationDirection::Right { through_carry } => {
                let bit0 = value & 1;
                let new_value = if through_carry {
                    if carry_in {
                        (value >> 1) | (1 << 7)
                    } else {
                        value >> 1
                    }
                } else {
                    (value >> 1) | (bit0 << 7)
                };

                (new_value, bit0 == 1)
            }
            RotationDirection::Left { through_carry } => {
                let bit7 = (value >> 7) & 1;
                let new_value = if through_carry {
                    if carry_in {
                        (value << 1) | 1
                    } else {
                        value << 1
                    }
                } else {
                    (value << 1) | bit7
                };

                (new_value, bit7 == 1)
            }
        };

        (result, carry_out)
    }

    fn shift(&self, value: u8, kind: ShiftKind) -> (u8, bool) {
        match kind {
            ShiftKind::LeftArithmetic => {
                let bit7 = (value >> 7) & 1;
                let new_value = value.shl(1);

                (new_value, bit7 == 1)
            }
            ShiftKind::RightArithmetic => {
                let bit7 = value & (1 << 7);
                let bit0 = value & 1;
                let new_value = value.shr(1) | (bit7);

                (new_value, bit0 == 1)
            }
            ShiftKind::RightLogical => {
                let bit0 = value & 1;
                let new_value = value.shr(1);

                (new_value, bit0 == 1)
            }
        }
    }

    fn execute_cb_transform(&mut self, value: u8, operation: CbTransform) -> u8 {
        match operation {
            CbTransform::Rotate { direction } => {
                let (result, carry_out) = self.rotate(value, direction);

                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, result == 0);

                result
            }
            CbTransform::Shift { kind } => {
                let (result, carry_out) = self.shift(value, kind);

                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, result == 0);

                result
            }
            CbTransform::Swap => {
                let low_nibble = value & 0xF;
                let high_nibble = (value & 0xF0) >> 4;
                let result = low_nibble << 4 | high_nibble;

                self.registers.set_flag(Flag::C, false);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, result == 0);

                result
            }
        }
    }
}
