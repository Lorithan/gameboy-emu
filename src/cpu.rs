mod alu;
mod cycles;
mod dispatch;
pub mod error;
mod opcode;
mod registers;
mod state;

#[cfg(test)]
mod tests;

pub use cycles::*;
pub use error::{CpuError, CpuFault, OpcodeEncoding, UnexpectedCycleResponse};
pub use registers::Registers;

use crate::{
    cpu::{
        error::IllegalOpcode,
        opcode::*,
        registers::*,
        state::{
            CycleTransition, ExecutionState, HaltBehavior, Imm16Use, InterruptSelection,
            ReturnKind, StopBehavior,
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
}

impl CPU {
    pub fn from_registers(registers: Registers) -> Self {
        CPU {
            registers,
            state: FetchOpcode {
                skip_increment: false,
            },
            interrupt_state: InterruptState::Disabled,
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
            FetchOpcode { .. }
            | DecodeCbOpcode
            | ReadDisplacement { .. }
            | ReadImmediate8 { .. }
            | ReadImm16Low { .. }
            | ReadImm16High { .. }
            | ReadSpOffset { .. }
            | ReadHighMemoryImmediate { .. }
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
                self.state = FetchOpcode {
                    skip_increment: false,
                };
                Ok(CycleOutcome::InstructionComplete)
            }
            CycleTransition::InterruptEntryFinished { interrupt } => {
                self.state = FetchOpcode {
                    skip_increment: false,
                };
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
                self.state = FetchOpcode {
                    skip_increment: false,
                };
                Ok(CycleOutcome::InterruptEntryCancelled)
            }
            CycleTransition::HaltFinished(behavior) => {
                match behavior {
                    HaltBehavior::Sleep => {
                        self.state = Halted;
                    }
                    HaltBehavior::SuppressNextIncrement => {
                        self.state = FetchOpcode {
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
                    self.state = Stopped;
                    Ok(CycleOutcome::StopEntered)
                }
                StopBehavior::Halt => {
                    self.registers.increment_pc();
                    self.state = Halted;
                    Ok(CycleOutcome::InstructionComplete)
                }
                StopBehavior::Continue => {
                    self.state = FetchOpcode {
                        skip_increment: false,
                    };
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
        if let FetchOpcode { skip_increment } = self.state
            && self.interrupt_state == InterruptState::Enabled
        {
            let pending = pending & 0x1F; // Only the lower 5 bits are relevant

            if pending != 0 {
                if skip_increment {
                    self.registers.pc = self.registers.pc.wrapping_sub(1);
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
            self.state = FetchOpcode {
                skip_increment: false,
            };
        }
    }

    pub(crate) fn wake_from_stop_if(&mut self, wake: bool) {
        if matches!(self.state, Stopped) && wake {
            self.state = FetchOpcode {
                skip_increment: false,
            };
        }
    }

    fn complete_cycle_inner(
        &mut self,
        response: CycleResponse,
    ) -> Result<CycleTransition, CpuError> {
        match self.state {
            FetchOpcode { skip_increment } => self.read_opcode(
                response.read_value()?,
                response.pending_interrupts()?,
                response.stop_wake_active()?,
                skip_increment,
            ),
            DecodeCbOpcode => {
                let value = response.read_value()?;
                self.registers.increment_pc();

                let DecodedCbOpcode { target, cb_type } = DecodedCbOpcode::from(value);

                match target {
                    Target::Register(register) => {
                        let value = self.registers.get(register);

                        match cb_type {
                            CbType::Transform(operation) => {
                                let result = self.execute_cb_transform(value, operation);

                                self.registers.set(register, result);
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

                                self.registers.set(register, result);
                                Ok(CycleTransition::InstructionFinished)
                            }
                            CbType::Set(bit) => {
                                let result = value | (1 << bit);

                                self.registers.set(register, result);
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
                    Destination::Register(register) => Ok(self.store_immediate(register, value)),
                    Destination::Memory(addr) => {
                        self.registers.increment_pc();
                        let next_state = ExecutionState::WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }
                }
            }
            ReadImm16Low { then } => {
                let low = response.read_value()?;
                self.registers.increment_pc();

                let next_state = ReadImm16High { low, then };
                Ok(CycleTransition::Continue(next_state))
            }
            ReadImm16High { low, then } => {
                let high = response.read_value()?;
                self.registers.increment_pc();

                let value = u16::from_le_bytes([low, high]);
                Ok(self.finish_imm16(value, then))
            }
            ReadMemory { destination, .. } => {
                Ok(self.store_fetched(destination, response.read_value()?))
            }
            ReadMemoryAndUpdateHL { addr, operation } => {
                self.store_fetched(Register::A, response.read_value()?);
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
                self.registers.set16(target, value);
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

    fn store_immediate(&mut self, register: Register, value: u8) -> CycleTransition {
        self.registers.set(register, value);

        self.registers.increment_pc();
        CycleTransition::InstructionFinished
    }

    fn store_fetched(&mut self, destination: Register, value: u8) -> CycleTransition {
        self.registers.set(destination, value);

        CycleTransition::InstructionFinished
    }

    fn finish_imm16(&mut self, value: u16, then: Imm16Use) -> CycleTransition {
        match then {
            Imm16Use::LoadRegister(register16) => {
                self.registers.set16(register16, value);
                CycleTransition::InstructionFinished
            }
            Imm16Use::Jump { taken } => {
                if taken {
                    let next_state = ApplyJump { target: value };
                    CycleTransition::Continue(next_state)
                } else {
                    CycleTransition::InstructionFinished
                }
            }
            Imm16Use::Call { taken, stack_addr } => {
                let [return_addr_low, return_addr_high] = self.registers.pc.to_le_bytes();
                if taken {
                    let next_state = Call {
                        jump_addr: value,
                        stack_addr,
                        return_addr_high,
                        return_addr_low,
                    };
                    CycleTransition::Continue(next_state)
                } else {
                    CycleTransition::InstructionFinished
                }
            }
            Imm16Use::LoadAFromAbsolute => {
                let next_state = ReadMemory {
                    addr: value,
                    destination: Register::A,
                };
                CycleTransition::Continue(next_state)
            }
            Imm16Use::StoreAToAbsolute { value: a_value } => {
                let next_state = WriteMemory {
                    addr: value,
                    value: a_value,
                };
                CycleTransition::Continue(next_state)
            }
            Imm16Use::StoreSp { sp } => {
                let sp_low = (sp & 0xFF) as u8;
                let sp_high = (sp >> 8) as u8;
                let next_state = WriteSpLow {
                    sp_low,
                    sp_high,
                    addr: value,
                };
                CycleTransition::Continue(next_state)
            }
        }
    }
}
