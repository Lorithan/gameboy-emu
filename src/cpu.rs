mod opcode;
mod registers;

#[cfg(test)]
mod tests;

pub use registers::Registers;

use crate::cpu::{opcode::*, registers::*};

#[derive(Debug, Clone, Copy)]
enum Destination {
    Register(Register),
    Memory(u16),
}

#[derive(Debug, Clone, Copy)]
enum ExecutionState {
    FetchOpcode,
    ReadDisplacement {
        taken: bool,
    },
    ApplyJump {
        target: u16,
    },
    ReadImmediate8 {
        target: Destination,
    },
    ReadImmediate16Low {
        destination: Register16,
    },
    ReadImmediate16High {
        destination: Register16,
        low: u8,
    },
    ReadJumpLow {
        taken: bool,
    },
    ReadJumpHigh {
        taken: bool,
        low: u8,
    },
    ReadMemory {
        addr: u16,
        destination: Register,
    },
    ReadOperation {
        addr: u16,
        operation: UnaryOperation,
    },
    ReadImmediateAccumulator {
        value: u8,
        operation: ArithmeticKind,
    },
    ReadMemoryAccumulator {
        addr: u16,
        value: u8,
        operation: ArithmeticKind,
    },
    WriteMemory {
        addr: u16,
        value: u8,
    },
    Push {
        addr: u16,
        value: u16,
    },
    PushHigh {
        addr: u16,
        high: u8,
        low: u8,
    },
    PushLow {
        addr: u16,
        low: u8,
    },
    PopLow {
        addr: u16,
        target: PushOperand,
    },
    PopHigh {
        addr: u16,
        low: u8,
        target: PushOperand,
    },
    ReadCallLow {
        taken: bool,
        stack_addr: u16,
    },
    ReadCallHigh {
        taken: bool,
        low: u8,
        stack_addr: u16,
    },
    Call {
        jump_addr: u16,
        stack_addr: u16,
        return_addr_high: u8,
        return_addr_low: u8,
    },
    PushCallerHigh {
        jump_addr: u16,
        stack_addr: u16,
        return_addr_high: u8,
        return_addr_low: u8,
    },
    PushCallerLow {
        jump_addr: u16,
        stack_addr: u16,
        return_addr_low: u8,
    },
    ReadReturnLow {
        addr: u16,
    },
    ReadReturnHigh {
        addr: u16,
        low: u8,
    },
    ReturnConditional {
        taken: bool,
        addr: u16,
    },
    Faulted(CpuFault),
}

use ExecutionState::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapShot {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub pc: u16,
    pub sp: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum CycleAction {
    Idle,
    Read { addr: u16 },
    Write { addr: u16, value: u8 },
}

#[derive(Debug, Clone, Copy)]
pub struct CpuFault {
    pub opcode: u8,
    pub addr: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum CpuError {
    UnsupportedOpcode(CpuFault),
    MissingReadValue { addr: u16 },
}

#[derive(Debug, Clone)]
pub struct CPU {
    registers: Registers,
    state: ExecutionState,
}

impl CPU {
    pub fn from_registers(registers: Registers) -> Self {
        CPU {
            registers,
            state: ExecutionState::FetchOpcode,
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

    pub fn request_cycle(&self) -> Result<CycleAction, CpuError> {
        match self.state {
            FetchOpcode
            | ReadDisplacement { .. }
            | ReadImmediate8 { .. }
            | ReadImmediate16Low { .. }
            | ReadJumpLow { .. }
            | ReadJumpHigh { .. }
            | ReadCallLow { .. }
            | ReadCallHigh { .. }
            | ReadImmediate16High { .. }
            | ReadImmediateAccumulator { .. } => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ReadMemory { addr, .. }
            | PopLow { addr, .. }
            | PopHigh { addr, .. }
            | ReadReturnLow { addr }
            | ReadReturnHigh { addr, .. }
            | ReadOperation { addr, .. }
            | ReadMemoryAccumulator { addr, .. } => Ok(CycleAction::Read { addr }),
            WriteMemory { addr, value } => Ok(CycleAction::Write { addr, value }),
            PushHigh { addr, high, .. } => Ok(CycleAction::Write { addr, value: high }),
            PushLow { addr, low } => Ok(CycleAction::Write { addr, value: low }),
            PushCallerHigh {
                stack_addr,
                return_addr_high,
                ..
            } => Ok(CycleAction::Write {
                addr: stack_addr,
                value: return_addr_high,
            }),
            PushCallerLow {
                stack_addr,
                return_addr_low,
                ..
            } => Ok(CycleAction::Write {
                addr: stack_addr,
                value: return_addr_low,
            }),
            Push { .. } | ApplyJump { .. } | Call { .. } | ReturnConditional { .. } => {
                Ok(CycleAction::Idle)
            }
            Faulted(err) => Err(CpuError::UnsupportedOpcode(err)),
        }
    }

    pub fn complete_cycle(&mut self, read_value: Option<u8>) -> Result<bool, CpuError> {
        match self.state {
            FetchOpcode => self.read_opcode(read_value),
            ReadDisplacement { taken } => {
                let value = match read_value {
                    Some(byte) => byte as i8,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };
                self.registers.increment_pc();
                if taken {
                    self.state = ApplyJump {
                        target: self.registers.pc.wrapping_add_signed(value as i16),
                    };
                    Ok(false)
                } else {
                    self.state = FetchOpcode;
                    Ok(true)
                }
            }
            ApplyJump { target } => {
                self.registers.set_pc(target);

                self.state = FetchOpcode;
                Ok(true)
            }
            ReadImmediate8 { target } => match target {
                Destination::Register(register) => self.store_immediate(register, read_value),
                Destination::Memory(addr) => {
                    let value = match read_value {
                        Some(byte) => byte,
                        None => {
                            return Err(CpuError::MissingReadValue {
                                addr: self.registers.pc,
                            });
                        }
                    };

                    self.state = ExecutionState::WriteMemory { addr, value };
                    self.registers.increment_pc();
                    Ok(false)
                }
            },
            ReadImmediate16Low { destination } => {
                let value = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };

                self.registers.increment_pc();
                self.state = ReadImmediate16High {
                    destination,
                    low: value,
                };
                Ok(false)
            }
            ReadImmediate16High { destination, low } => {
                self.store_immediate_word(destination, low, read_value)
            }
            ReadJumpLow { taken } => {
                let value = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };

                self.registers.increment_pc();
                self.state = ReadJumpHigh { taken, low: value };
                Ok(false)
            }
            ReadJumpHigh { taken, low } => {
                let value = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };

                self.registers.increment_pc();

                if taken {
                    self.state = ApplyJump {
                        target: ((value as u16) << 8) | low as u16,
                    };
                    Ok(false)
                } else {
                    self.state = FetchOpcode;
                    Ok(true)
                }
            }
            ReadMemory { destination, addr } => self.store_fetched(destination, addr, read_value),
            ReadOperation { addr, operation } => {
                let mut value = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };
                value = match operation {
                    UnaryOperation::Increment => self.increment(value),
                    UnaryOperation::Decrement => self.decrement(value),
                };

                self.state = WriteMemory { addr, value };
                Ok(false)
            }
            ReadImmediateAccumulator {
                value: left,
                operation,
            } => {
                let right = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };

                self.execute_accumulator(operation, left, right);

                self.registers.increment_pc();
                self.state = FetchOpcode;
                Ok(true)
            }
            ReadMemoryAccumulator {
                addr,
                value: left,
                operation,
            } => {
                let right = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };

                self.execute_accumulator(operation, left, right);

                self.state = FetchOpcode;
                Ok(true)
            }
            WriteMemory { .. } => {
                self.state = ExecutionState::FetchOpcode;
                Ok(true)
            }
            Push { addr, value } => {
                let high = (value >> 8) as u8;
                let low = (value & 0xFF) as u8;
                self.state = PushHigh { addr, high, low };
                Ok(false)
            }
            PushHigh { addr, low, .. } => {
                self.registers.set_sp(addr);
                self.state = PushLow {
                    addr: self.registers.sp.wrapping_sub(1),
                    low,
                };
                Ok(false)
            }
            PushLow { addr, .. } => {
                self.registers.set_sp(addr);
                self.state = FetchOpcode;
                Ok(true)
            }
            PopLow { addr, target } => {
                let low = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };

                self.registers.sp = addr.wrapping_add(1);
                self.state = PopHigh {
                    addr: self.registers.sp,
                    low,
                    target,
                };
                Ok(false)
            }
            PopHigh { addr, low, target } => {
                let high = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };

                let value = (high as u16) << 8 | low as u16;

                match target {
                    PushOperand::BC => self.registers.set_bc(value),
                    PushOperand::DE => self.registers.set_de(value),
                    PushOperand::HL => self.registers.set_hl(value),
                    PushOperand::AF => self.registers.set_af(value),
                }

                self.registers.sp = addr.wrapping_add(1);
                self.state = FetchOpcode;
                Ok(true)
            }
            ReadCallLow { taken, stack_addr } => {
                let low = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };

                self.registers.increment_pc();
                self.state = ReadCallHigh {
                    taken,
                    low,
                    stack_addr,
                };
                Ok(false)
            }
            ReadCallHigh {
                taken,
                low,
                stack_addr,
            } => {
                let high = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };
                self.registers.increment_pc();

                let jump_addr = (high as u16) << 8 | low as u16;

                if taken {
                    let return_addr = self.registers.pc;

                    self.state = Call {
                        jump_addr,
                        stack_addr,
                        return_addr_high: (return_addr >> 8) as u8,
                        return_addr_low: (return_addr & 0xFF) as u8,
                    };

                    Ok(false)
                } else {
                    self.state = FetchOpcode;
                    Ok(true)
                }
            }
            Call {
                jump_addr,
                stack_addr,
                return_addr_high,
                return_addr_low,
            } => {
                self.state = PushCallerHigh {
                    jump_addr,
                    stack_addr,
                    return_addr_high,
                    return_addr_low,
                };
                Ok(false)
            }
            PushCallerHigh {
                jump_addr,
                stack_addr,
                return_addr_low,
                ..
            } => {
                self.registers.set_sp(stack_addr);
                self.state = PushCallerLow {
                    jump_addr,
                    stack_addr: self.registers.sp.wrapping_sub(1),
                    return_addr_low,
                };
                Ok(false)
            }
            PushCallerLow {
                jump_addr,
                stack_addr,
                ..
            } => {
                self.registers.set_sp(stack_addr);
                self.registers.set_pc(jump_addr);

                self.state = FetchOpcode;
                Ok(true)
            }
            ReadReturnLow { addr } => {
                let low = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };
                self.registers.set_sp(addr.wrapping_add(1));

                self.state = ReadReturnHigh {
                    addr: addr.wrapping_add(1),
                    low,
                };
                Ok(false)
            }
            ReadReturnHigh { addr, low } => {
                let high = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };
                self.registers.set_sp(addr.wrapping_add(1));

                let jump_addr = (high as u16) << 8 | low as u16;
                self.state = ApplyJump { target: jump_addr };
                Ok(false)
            }
            ReturnConditional { taken, addr } => {
                if taken {
                    self.state = ReadReturnLow { addr };
                    Ok(false)
                } else {
                    self.state = FetchOpcode;
                    Ok(true)
                }
            }
            Faulted(err) => Err(CpuError::UnsupportedOpcode(err)),
        }
    }

    fn read_opcode(&mut self, read_value: Option<u8>) -> Result<bool, CpuError> {
        let (opcode, byte) = match read_value {
            Some(byte) => (DecodedOpcode::from(byte), byte),
            None => {
                return Err(CpuError::MissingReadValue {
                    addr: self.registers.pc,
                });
            }
        };

        let op_addr = self.registers.pc;
        self.registers.increment_pc();

        match opcode {
            DecodedOpcode::Nop => {
                self.state = ExecutionState::FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::Load {
                destination,
                source,
            } => match source {
                Target::Register(register) => {
                    let value = self.get_register(register);

                    match destination {
                        Target::Register(register) => {
                            self.set_register(register, value);
                            Ok(true)
                        }
                        Target::IndirectHL => {
                            self.state = WriteMemory {
                                addr: self.registers.hl(),
                                value,
                            };
                            Ok(false)
                        }
                    }
                }
                Target::IndirectHL => {
                    match destination {
                        Target::Register(register) => {
                            self.state = ReadMemory {
                                addr: self.registers.hl(),
                                destination: register,
                            };
                            Ok(false)
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
                self.state = ReadImmediate8 { target };
                Ok(false)
            }
            DecodedOpcode::LoadImmediate16 { destination } => {
                self.state = ReadImmediate16Low { destination };
                Ok(false)
            }
            DecodedOpcode::Unary { operation, target } => match target {
                Target::Register(register) => {
                    let mut value = self.get_register(register);
                    value = match operation {
                        UnaryOperation::Increment => self.increment(value),
                        UnaryOperation::Decrement => self.decrement(value),
                    };
                    self.set_register(register, value);

                    self.state = FetchOpcode;
                    Ok(true)
                }
                Target::IndirectHL => {
                    self.state = ReadOperation {
                        addr: self.registers.hl(),
                        operation,
                    };
                    Ok(false)
                }
            },
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

                        self.state = FetchOpcode;
                        Ok(true)
                    }
                    Operand::Target(Target::IndirectHL) => {
                        let value = self.get_register(Register::A);
                        self.state = ReadMemoryAccumulator {
                            addr: self.registers.hl(),
                            value,
                            operation,
                        };
                        Ok(false)
                    }
                    Operand::Immediate => {
                        let value = self.get_register(Register::A);
                        self.state = ReadImmediateAccumulator { value, operation };
                        Ok(false)
                    }
                }
            }
            DecodedOpcode::Cpl => {
                let value = !self.get_register(Register::A);
                self.set_register(Register::A, value);
                self.registers.set_flag(Flag::N, true);
                self.registers.set_flag(Flag::H, true);
                Ok(true)
            }
            DecodedOpcode::Scf => {
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::C, true);
                Ok(true)
            }
            DecodedOpcode::Ccf => {
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers
                    .set_flag(Flag::C, !self.registers.get_flag(Flag::C));
                Ok(true)
            }
            DecodedOpcode::Rotate { direction } => {
                let a = self.get_register(Register::A);
                let carry_in = self.registers.get_flag(Flag::C);

                let (result, carry_out) = match direction {
                    RotationDirection::Right { through_carry } => {
                        let bit0 = a & 1;
                        let new_a = if through_carry {
                            if carry_in {
                                (a >> 1) | (1 << 7)
                            } else {
                                a >> 1
                            }
                        } else {
                            (a >> 1) | (bit0 << 7)
                        };

                        (new_a, bit0 == 1)
                    }
                    RotationDirection::Left { through_carry } => {
                        let bit7 = (a >> 7) & 1;
                        let new_a = if through_carry {
                            if carry_in { (a << 1) | 1 } else { a << 1 }
                        } else {
                            (a << 1) | bit7
                        };

                        (new_a, bit7 == 1)
                    }
                };

                self.set_register(Register::A, result);
                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, false);
                Ok(true)
            }
            DecodedOpcode::RelativeJump { condition } => {
                let taken = condition.jump(self.registers.f);

                self.state = ReadDisplacement { taken };
                Ok(false)
            }
            DecodedOpcode::AbsoluteJump { condition } => {
                let taken = condition.jump(self.registers.f);

                self.state = ReadJumpLow { taken };
                Ok(false)
            }
            DecodedOpcode::JumpHL => {
                self.registers.set_pc(self.registers.hl());
                Ok(true)
            }
            DecodedOpcode::Push { operand } => {
                let addr = self.registers.sp.wrapping_sub(1);
                let value = match operand {
                    PushOperand::BC => self.registers.bc(),
                    PushOperand::DE => self.registers.de(),
                    PushOperand::HL => self.registers.hl(),
                    PushOperand::AF => self.registers.af(),
                };

                self.state = Push { addr, value };
                Ok(false)
            }
            DecodedOpcode::Pop { operand } => {
                let addr = self.registers.sp;
                self.state = PopLow {
                    addr,
                    target: operand,
                };
                Ok(false)
            }
            DecodedOpcode::Call { condition } => {
                let taken = condition.jump(self.registers.f);
                let stack_addr = self.registers.sp.wrapping_sub(1);

                self.state = ReadCallLow { taken, stack_addr };
                Ok(false)
            }
            DecodedOpcode::Return => {
                self.state = ReadReturnLow {
                    addr: self.registers.sp,
                };
                Ok(false)
            }
            DecodedOpcode::ConditionalReturn { condition } => {
                let taken = condition.jump(self.registers.f);

                self.state = ReturnConditional {
                    taken,
                    addr: self.registers.sp,
                };
                Ok(false)
            }
            DecodedOpcode::Restart { target } => {
                let return_addr = self.registers.pc;
                let return_addr_high = (return_addr >> 8) as u8;
                let return_addr_low = (return_addr & 0xFF) as u8;
                self.state = Call {
                    jump_addr: target,
                    stack_addr: self.registers.sp.wrapping_sub(1),
                    return_addr_high,
                    return_addr_low,
                };
                Ok(false)
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
                Ok(true)
            }
            DecodedOpcode::Unknown => {
                let err = CpuFault {
                    opcode: byte,
                    addr: op_addr,
                };
                self.state = ExecutionState::Faulted(err);
                Err(CpuError::UnsupportedOpcode(err))
            }
        }
    }

    fn store_immediate(
        &mut self,
        register: Register,
        read_value: Option<u8>,
    ) -> Result<bool, CpuError> {
        let value = match read_value {
            Some(byte) => byte,
            None => {
                return Err(CpuError::MissingReadValue {
                    addr: self.registers.pc,
                });
            }
        };

        self.set_register(register, value);

        self.state = ExecutionState::FetchOpcode;
        self.registers.increment_pc();
        Ok(true)
    }

    fn store_fetched(
        &mut self,
        destination: Register,
        addr: u16,
        read_value: Option<u8>,
    ) -> Result<bool, CpuError> {
        let value = match read_value {
            Some(byte) => byte,
            None => {
                return Err(CpuError::MissingReadValue { addr });
            }
        };

        self.set_register(destination, value);

        self.state = ExecutionState::FetchOpcode;

        Ok(true)
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
        read_value: Option<u8>,
    ) -> Result<bool, CpuError> {
        let high_value = match read_value {
            Some(byte) => byte,
            None => {
                return Err(CpuError::MissingReadValue {
                    addr: self.registers.pc,
                });
            }
        };

        let value = (high_value as u16) << 8 | low as u16;

        match destination {
            Register16::BC => self.registers.set_bc(value),
            Register16::DE => self.registers.set_de(value),
            Register16::HL => self.registers.set_hl(value),
            Register16::SP => self.registers.set_sp(value),
        }

        self.state = ExecutionState::FetchOpcode;
        self.registers.increment_pc();
        Ok(true)
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
}
