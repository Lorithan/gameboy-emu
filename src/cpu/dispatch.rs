use crate::cpu::{
    ArithmeticKind, CPU, CpuError, CpuFault, CycleTransition, DecodedOpcode, Destination,
    ExecutionState, HaltBehavior, HighMemoryAddressSource, IllegalOpcode, Imm16Use, IndirectPair,
    InterruptState, OpcodeEncoding, Operand, PushOperand, Register, ReturnKind, StopBehavior,
    Target, UnaryOperation, registers::Flag,
};

impl CPU {
    pub(super) fn read_opcode(
        &mut self,
        response: u8,
        pending: u8,
        stop_wake_active: bool,
        skip_increment: bool,
    ) -> Result<CycleTransition, CpuError> {
        let op_addr = self.registers.pc;

        if !skip_increment {
            self.registers.increment_pc();
        }

        let opcode = match DecodedOpcode::try_from(response) {
            Ok(opcode) => opcode,
            Err(IllegalOpcode(byte)) => {
                let fault = CpuFault {
                    opcode: OpcodeEncoding::Base(byte),
                    addr: op_addr,
                };
                return Ok(CycleTransition::Faulted(fault));
            }
        };

        match opcode {
            DecodedOpcode::Nop => Ok(CycleTransition::InstructionFinished),
            DecodedOpcode::Load {
                destination,
                source,
            } => match source {
                Target::Register(register) => {
                    let value = self.registers.get(register);

                    match destination {
                        Target::Register(register) => {
                            self.registers.set(register, value);
                            Ok(CycleTransition::InstructionFinished)
                        }
                        Target::IndirectHL => {
                            let next_state = ExecutionState::WriteMemory {
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
                            let next_state = ExecutionState::ReadMemory {
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
                let next_state = ExecutionState::ReadImmediate8 { target };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadImmediate16 { destination } => {
                let next_state = ExecutionState::ReadImm16Low {
                    then: Imm16Use::LoadRegister(destination),
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Unary { operation, target } => match target {
                Target::Register(register) => {
                    let mut value = self.registers.get(register);
                    value = match operation {
                        UnaryOperation::Increment => self.increment(value),
                        UnaryOperation::Decrement => self.decrement(value),
                    };
                    self.registers.set(register, value);

                    Ok(CycleTransition::InstructionFinished)
                }
                Target::IndirectHL => {
                    let next_state = ExecutionState::ReadOperation {
                        addr: self.registers.hl(),
                        operation,
                    };
                    Ok(CycleTransition::Continue(next_state))
                }
            },
            DecodedOpcode::Unary16 { operation, target } => {
                let mut value = self.registers.get16(target);

                value = match operation {
                    UnaryOperation::Increment => value.wrapping_add(1),
                    UnaryOperation::Decrement => value.wrapping_sub(1),
                };

                let next_state = ExecutionState::Unary16 { target, value };
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
                        let left = self.registers.get(Register::A);
                        let right = self.registers.get(register);

                        self.execute_accumulator(operation, left, right);

                        Ok(CycleTransition::InstructionFinished)
                    }
                    Operand::Target(Target::IndirectHL) => {
                        let value = self.registers.get(Register::A);
                        let next_state = ExecutionState::ReadMemoryAccumulator {
                            addr: self.registers.hl(),
                            value,
                            operation,
                        };
                        Ok(CycleTransition::Continue(next_state))
                    }
                    Operand::Immediate => {
                        let value = self.registers.get(Register::A);
                        let next_state =
                            ExecutionState::ReadImmediateAccumulator { value, operation };
                        Ok(CycleTransition::Continue(next_state))
                    }
                }
            }
            DecodedOpcode::AddHL { target } => {
                let lhs = self.registers.hl();
                let rhs = self.registers.get16(target);
                let next_state = ExecutionState::Addition16 { lhs, rhs };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::ComplementAccumulator => {
                let value = !self.registers.get(Register::A);
                self.registers.set(Register::A, value);
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
                let value = self.registers.get(Register::A);
                let (result, carry_out) = self.rotate(value, direction);

                self.registers.set(Register::A, result);
                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, false);
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::RelativeJump { condition } => {
                let taken = condition.is_met(&self.registers);

                let next_state = ExecutionState::ReadDisplacement { taken };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::AbsoluteJump { condition } => {
                let taken = condition.is_met(&self.registers);

                let next_state = ExecutionState::ReadImm16Low {
                    then: Imm16Use::Jump { taken },
                };
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

                let next_state = ExecutionState::Push { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Pop { operand } => {
                let addr = self.registers.sp;
                let next_state = ExecutionState::PopLow {
                    addr,
                    target: operand,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Call { condition } => {
                let taken = condition.is_met(&self.registers);
                let stack_addr = self.registers.sp.wrapping_sub(1);

                let next_state = ExecutionState::ReadImm16Low {
                    then: Imm16Use::Call { taken, stack_addr },
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Return => {
                let next_state = ExecutionState::ReadReturnLow {
                    addr: self.registers.sp,
                    kind: ReturnKind::Normal,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::ConditionalReturn { condition } => {
                let taken = condition.is_met(&self.registers);

                let next_state = ExecutionState::ReturnConditional {
                    taken,
                    addr: self.registers.sp,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::Restart { target } => {
                let return_addr = self.registers.pc;
                let return_addr_high = (return_addr >> 8) as u8;
                let return_addr_low = (return_addr & 0xFF) as u8;
                let next_state = ExecutionState::Call {
                    jump_addr: target,
                    stack_addr: self.registers.sp.wrapping_sub(1),
                    return_addr_high,
                    return_addr_low,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::DecimalAdjust => {
                let mut a = self.registers.get(Register::A);
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

                self.registers.set(Register::A, a);
                self.registers.set_flag(Flag::Z, a == 0);
                self.registers.set_flag(Flag::H, false);
                Ok(CycleTransition::InstructionFinished)
            }
            DecodedOpcode::LoadHlFromSpOffset => {
                let next_state = ExecutionState::ReadSpOffset {
                    sp: self.registers.sp,
                    hl: true,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::AddSignedOffsetToSp => {
                let next_state = ExecutionState::ReadSpOffset {
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

                let next_state = ExecutionState::ReadMemory {
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

                let next_state = ExecutionState::WriteMemory { addr, value };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadAFromHL { operation } => {
                let addr = self.registers.hl();

                let next_state = ExecutionState::ReadMemoryAndUpdateHL { addr, operation };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::StoreAToHL { operation } => {
                let value = self.registers.a;
                let addr = self.registers.hl();

                let next_state = ExecutionState::WriteMemoryAndUpdateHL {
                    addr,
                    value,
                    operation,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadAFromAbsolute => {
                let next_state = ExecutionState::ReadImm16Low {
                    then: Imm16Use::LoadAFromAbsolute,
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::StoreAToAbsolute => {
                let value = self.registers.a;

                let next_state = ExecutionState::ReadImm16Low {
                    then: Imm16Use::StoreAToAbsolute { value },
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::LoadHighMemory {
                address_source: destination,
            } => match destination {
                HighMemoryAddressSource::C => {
                    let addr = (0xFF00) | self.registers.c as u16;
                    let next_state = ExecutionState::ReadMemory {
                        addr,
                        destination: Register::A,
                    };
                    Ok(CycleTransition::Continue(next_state))
                }
                HighMemoryAddressSource::Immediate => {
                    let next_state = ExecutionState::ReadHighMemoryImmediate { store: None };
                    Ok(CycleTransition::Continue(next_state))
                }
            },
            DecodedOpcode::StoreHighMemory {
                address_source: source,
            } => match source {
                HighMemoryAddressSource::C => {
                    let addr = (0xFF00) | self.registers.c as u16;
                    let value = self.registers.a;

                    let next_state = ExecutionState::WriteMemory { addr, value };
                    Ok(CycleTransition::Continue(next_state))
                }
                HighMemoryAddressSource::Immediate => {
                    let value = self.registers.a;

                    let next_state = ExecutionState::ReadHighMemoryImmediate { store: Some(value) };
                    Ok(CycleTransition::Continue(next_state))
                }
            },
            DecodedOpcode::TransferHLtoSP => {
                let hl = self.registers.hl();
                let next_state = ExecutionState::TransferHLtoSP { hl };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::StoreStackPointer => {
                let sp = self.registers.sp;
                let next_state = ExecutionState::ReadImm16Low {
                    then: Imm16Use::StoreSp { sp },
                };
                Ok(CycleTransition::Continue(next_state))
            }
            DecodedOpcode::DecodeCbOpcode => {
                Ok(CycleTransition::Continue(ExecutionState::DecodeCbOpcode))
            }
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
                let next_state = ExecutionState::ReadReturnLow {
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
        }
    }
}
