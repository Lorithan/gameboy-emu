use crate::{
    cpu::{
        CPU, CbType, CpuError, CycleResponse, CycleTransition, DecodedCbOpcode, Destination,
        Imm16Use, InterruptSelection, InterruptState, PushThen, Register, ReturnKind,
        StackRegister, Target, UnaryOperation, registers::Flag, state::ExecutionState,
    },
    memory::Interrupt,
};

impl CPU {
    pub(super) fn complete_cycle_inner(
        &mut self,
        response: CycleResponse,
    ) -> Result<CycleTransition, CpuError> {
        match self.state {
            ExecutionState::FetchOpcode { skip_increment } => self.read_opcode(
                response.read_value()?,
                response.pending_interrupts()?,
                response.stop_wake_active()?,
                skip_increment,
            ),
            ExecutionState::DecodeCbOpcode => {
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

                        Ok(CycleTransition::Continue(
                            ExecutionState::ReadHLForCbOpcode { addr, cb_type },
                        ))
                    }
                }
            }
            ExecutionState::ReadHLForCbOpcode { addr, cb_type } => {
                let value = response.read_value()?;

                match cb_type {
                    CbType::Transform(operation) => {
                        let value = self.execute_cb_transform(value, operation);

                        let next_state = ExecutionState::WriteMemory { addr, value };
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

                        let next_state = ExecutionState::WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }

                    CbType::Set(bit) => {
                        let value = value | (1 << bit);

                        let next_state = ExecutionState::WriteMemory { addr, value };
                        Ok(CycleTransition::Continue(next_state))
                    }
                }
            }
            ExecutionState::ReadDisplacement { taken } => {
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
            ExecutionState::ApplyJump { target } => {
                self.registers.set_pc(target);

                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::ReadImmediate8 { target } => {
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
            ExecutionState::ReadImm16Low { then } => {
                let low = response.read_value()?;
                self.registers.increment_pc();

                let next_state = ExecutionState::ReadImm16High { low, then };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::ReadImm16High { low, then } => {
                let high = response.read_value()?;
                self.registers.increment_pc();

                let value = u16::from_le_bytes([low, high]);
                Ok(self.finish_imm16(value, then))
            }
            ExecutionState::ReadMemory { destination, .. } => {
                Ok(self.store_fetched(destination, response.read_value()?))
            }
            ExecutionState::ReadMemoryAndUpdateHL { addr, operation } => {
                self.store_fetched(Register::A, response.read_value()?);
                let hl = match operation {
                    UnaryOperation::Increment => addr.wrapping_add(1),
                    UnaryOperation::Decrement => addr.wrapping_sub(1),
                };

                self.registers.set_hl(hl);
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::ReadOperation { addr, operation } => {
                let mut value = response.read_value()?;
                value = match operation {
                    UnaryOperation::Increment => self.increment(value),
                    UnaryOperation::Decrement => self.decrement(value),
                };

                let next_state = ExecutionState::WriteMemory { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::ReadImmediateAccumulator {
                value: left,
                operation,
            } => {
                let right = response.read_value()?;

                self.execute_accumulator(operation, left, right);

                self.registers.increment_pc();
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::ReadMemoryAccumulator {
                value: left,
                operation,
                ..
            } => {
                let right = response.read_value()?;

                self.execute_accumulator(operation, left, right);

                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::WriteMemory { .. } => Ok(CycleTransition::InstructionFinished),
            ExecutionState::WriteMemoryAndUpdateHL {
                addr, operation, ..
            } => {
                let hl = match operation {
                    UnaryOperation::Increment => addr.wrapping_add(1),
                    UnaryOperation::Decrement => addr.wrapping_sub(1),
                };

                self.registers.set_hl(hl);
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::Push { addr, value, then } => {
                let [low, high] = value.to_le_bytes();
                let next_state = ExecutionState::PushHigh {
                    addr,
                    high,
                    low,
                    then,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::PushHigh {
                addr, low, then, ..
            } => {
                self.registers.set_sp(addr);
                let next_state = ExecutionState::PushLow {
                    addr: self.registers.sp.wrapping_sub(1),
                    low,
                    then,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::PushLow { addr, then, .. } => {
                self.registers.set_sp(addr);
                if let PushThen::Jump(target) = then {
                    self.registers.set_pc(target);
                }
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::PopLow { addr, target } => {
                let low = response.read_value()?;

                self.registers.sp = addr.wrapping_add(1);
                let next_state = ExecutionState::PopHigh {
                    addr: self.registers.sp,
                    low,
                    target,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::PopHigh { addr, low, target } => {
                let high = response.read_value()?;

                let value = (high as u16) << 8 | low as u16;

                match target {
                    StackRegister::BC => self.registers.set_bc(value),
                    StackRegister::DE => self.registers.set_de(value),
                    StackRegister::HL => self.registers.set_hl(value),
                    StackRegister::AF => self.registers.set_af(value),
                }

                self.registers.sp = addr.wrapping_add(1);
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::ReadReturnLow { addr, kind } => {
                let low = response.read_value()?;
                self.registers.set_sp(addr.wrapping_add(1));

                let next_state = ExecutionState::ReadReturnHigh {
                    addr: addr.wrapping_add(1),
                    low,
                    kind,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::ReadReturnHigh { addr, low, kind } => {
                let high = response.read_value()?;
                self.registers.set_sp(addr.wrapping_add(1));

                let jump_addr = (high as u16) << 8 | low as u16;
                let next_state = ExecutionState::FinishReturn {
                    target: jump_addr,
                    kind,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::FinishReturn { target: addr, kind } => {
                self.registers.set_pc(addr);
                if matches!(kind, ReturnKind::Interrupt) {
                    self.interrupt_state = InterruptState::Enabled;
                }
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::ReturnConditional { taken, addr } => {
                if taken {
                    let next_state = ExecutionState::ReadReturnLow {
                        addr,
                        kind: ReturnKind::Normal,
                    };
                    Ok(CycleTransition::Continue(next_state))
                } else {
                    Ok(CycleTransition::InstructionFinished)
                }
            }
            ExecutionState::Unary16 { target, value } => {
                self.registers.set16(target, value);
                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::Addition16 { lhs, rhs } => {
                let (result, c) = lhs.overflowing_add(rhs);

                let h = ((lhs ^ rhs ^ result) & 0x1000) != 0;

                self.registers.set_flag(Flag::C, c);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, h);

                self.registers.set_hl(result);

                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::ReadSpOffset { sp, hl } => {
                let offset = response.read_value()? as i8;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let next_state = if hl {
                    ExecutionState::ApplySpOffset { sp, offset, hl }
                } else {
                    ExecutionState::DelaySpOffset { sp, offset, hl }
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::DelaySpOffset { sp, offset, hl } => {
                let next_state = ExecutionState::ApplySpOffset { sp, offset, hl };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::ApplySpOffset { sp, offset, hl } => {
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
            ExecutionState::ReadHighMemoryImmediate { store } => {
                let low = response.read_value()?;
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let addr = (0xFF00) | low as u16;

                let next_state = match store {
                    Some(value) => ExecutionState::WriteMemory { addr, value },
                    None => ExecutionState::ReadMemory {
                        addr,
                        destination: Register::A,
                    },
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::TransferHLtoSP { hl } => {
                self.registers.set_sp(hl);

                Ok(CycleTransition::InstructionFinished)
            }
            ExecutionState::WriteSpLow { sp_high, addr, .. } => {
                let addr = addr.wrapping_add(1);
                let next_state = ExecutionState::WriteMemory {
                    addr,
                    value: sp_high,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::InterruptEntry { pc, sp } => {
                let next_state = ExecutionState::InterruptWait { pc, sp };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::InterruptWait { pc, sp } => {
                let next_state = ExecutionState::InterruptPushHigh {
                    pc_high: (pc >> 8) as u8,
                    pc_low: (pc & 0xFF) as u8,
                    sp: sp.wrapping_sub(1),
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::InterruptPushHigh { pc_low, sp, .. } => {
                let interrupt_selection = response.interrupt_sample()?;

                self.registers.set_sp(sp);
                let next_state = ExecutionState::InterruptPushLow {
                    pc_low,
                    sp: sp.wrapping_sub(1),
                    captured_enable: interrupt_selection.enable_after_bus,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            ExecutionState::InterruptPushLow {
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
            ExecutionState::InterruptJump { interrupt } => match interrupt {
                InterruptSelection::Selected(interrupt) => {
                    self.registers.set_pc(interrupt.vector_address());
                    Ok(CycleTransition::InterruptEntryFinished { interrupt })
                }
                InterruptSelection::Cancelled => {
                    self.registers.set_pc(0);
                    Ok(CycleTransition::InterruptEntryCancelled)
                }
            },
            ExecutionState::Halted => Ok(CycleTransition::Continue(ExecutionState::Halted)),
            ExecutionState::Stopped => Err(CpuError::NoCycleRequested),
            ExecutionState::Faulted(err) => Err(CpuError::UnsupportedOpcode(err)),
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
                    let next_state = ExecutionState::ApplyJump { target: value };
                    CycleTransition::Continue(next_state)
                } else {
                    CycleTransition::InstructionFinished
                }
            }
            Imm16Use::Call { taken, stack_addr } => {
                if taken {
                    let next_state = ExecutionState::Push {
                        addr: stack_addr,
                        value: self.registers.pc,
                        then: PushThen::Jump(value),
                    };
                    CycleTransition::Continue(next_state)
                } else {
                    CycleTransition::InstructionFinished
                }
            }
            Imm16Use::LoadAFromAbsolute => {
                let next_state = ExecutionState::ReadMemory {
                    addr: value,
                    destination: Register::A,
                };
                CycleTransition::Continue(next_state)
            }
            Imm16Use::StoreAToAbsolute { value: a_value } => {
                let next_state = ExecutionState::WriteMemory {
                    addr: value,
                    value: a_value,
                };
                CycleTransition::Continue(next_state)
            }
            Imm16Use::StoreSp { sp } => {
                let sp_low = (sp & 0xFF) as u8;
                let sp_high = (sp >> 8) as u8;
                let next_state = ExecutionState::WriteSpLow {
                    sp_low,
                    sp_high,
                    addr: value,
                };
                CycleTransition::Continue(next_state)
            }
        }
    }
}
