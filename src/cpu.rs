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
            FetchOpcode => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ReadDisplacement { .. } => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ApplyJump { .. } => Ok(CycleAction::Idle),
            ReadImmediate8 { .. } => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ReadImmediate16Low { .. } => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ReadImmediate16High { .. } => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ReadMemory { addr, .. } => Ok(CycleAction::Read { addr }),
            ReadOperation { addr, .. } => Ok(CycleAction::Read { addr }),
            ReadImmediateAccumulator { .. } => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ReadMemoryAccumulator { addr, .. } => Ok(CycleAction::Read { addr }),
            WriteMemory { addr, value } => Ok(CycleAction::Write { addr, value }),
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
                self.registers.pc = self.registers.pc.wrapping_add(1);

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
                    self.registers.pc = self.registers.pc.wrapping_add(1);
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

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadImmediate16High {
                    destination,
                    low: value,
                };
                Ok(false)
            }
            ReadImmediate16High { destination, low } => {
                self.store_immediate_word(destination, low, read_value)
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

                self.registers.pc = self.registers.pc.wrapping_add(1);
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
        self.registers.pc = self.registers.pc.wrapping_add(1);

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
                    let value = self.register(register);

                    match destination {
                        Target::Register(register) => {
                            self.store_register(register, value);
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
                    let mut value = self.register(register);
                    value = match operation {
                        UnaryOperation::Increment => self.increment(value),
                        UnaryOperation::Decrement => self.decrement(value),
                    };
                    self.store_register(register, value);

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
                        let left = self.register(Register::A);
                        let right = self.register(register);

                        self.execute_accumulator(operation, left, right);

                        self.state = FetchOpcode;
                        Ok(true)
                    }
                    Operand::Target(Target::IndirectHL) => {
                        let value = self.register(Register::A);
                        self.state = ReadMemoryAccumulator {
                            addr: self.registers.hl(),
                            value,
                            operation,
                        };
                        Ok(false)
                    }
                    Operand::Immediate => {
                        let value = self.register(Register::A);
                        self.state = ReadImmediateAccumulator { value, operation };
                        Ok(false)
                    }
                }
            }
            DecodedOpcode::RelativeJump { condition } => {
                let taken = match condition {
                    JumpCondition::Always => true,
                    JumpCondition::Z => self.registers.get_flag(Flag::Z),
                    JumpCondition::NZ => !self.registers.get_flag(Flag::Z),
                    JumpCondition::C => self.registers.get_flag(Flag::C),
                    JumpCondition::NC => !self.registers.get_flag(Flag::C),
                };

                self.state = ReadDisplacement { taken };
                Ok(false)
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

        self.store_register(register, value);

        self.state = ExecutionState::FetchOpcode;
        self.registers.pc = self.registers.pc.wrapping_add(1);

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

        self.store_register(destination, value);

        self.state = ExecutionState::FetchOpcode;

        Ok(true)
    }

    fn store_register(&mut self, destination: Register, value: u8) {
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

    fn register(&self, register: Register) -> u8 {
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
        self.registers.pc = self.registers.pc.wrapping_add(1);

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
                self.store_register(Register::A, result);
            }
            ArithmeticKind::Sub { carry } => {
                let result = self.sub(left, right, carry);
                self.store_register(Register::A, result);
            }
            ArithmeticKind::Compare => {
                self.sub(left, right, false);
            }
        }
    }
}
