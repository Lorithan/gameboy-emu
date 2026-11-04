mod opcode;
mod registers;

#[cfg(test)]
mod tests;

pub use registers::Registers;

use crate::cpu::{
    opcode::DecodedOpcode,
    registers::{RegisterValue, WordValue},
};

#[derive(Debug, Clone, Copy)]
enum Destination {
    Register(RegisterValue),
    Memory(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOperation {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccumulatorOperation {
    Add,
    Sub,
}

#[derive(Debug, Clone, Copy)]
enum ExecutionState {
    FetchOpcode,
    ReadImmediate8 {
        destination: Destination,
    },
    ReadImmediate16Low {
        destination: WordValue,
    },
    ReadImmediate16High {
        destination: WordValue,
        low: u8,
    },
    ReadMemory {
        addr: u16,
        destination: RegisterValue,
    },
    ReadOperation {
        addr: u16,
        operation: UnaryOperation,
    },
    ReadImmediateAccumulator {
        value: u8,
        operation: AccumulatorOperation,
    },
    ReadMemoryAccumulator {
        addr: u16,
        value: u8,
        operation: AccumulatorOperation,
    },
    WriteMemory {
        addr: u16,
        value: u8,
    },
    Faulted {
        opcode: u8,
        addr: u16,
    },
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
    opcode: u8,
    addr: u16,
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
            Faulted { opcode, addr } => Err(CpuError::UnsupportedOpcode(CpuFault { opcode, addr })),
        }
    }

    pub fn complete_cycle(&mut self, read_value: Option<u8>) -> Result<bool, CpuError> {
        match self.state {
            FetchOpcode => self.read_opcode(read_value),
            ReadImmediate8 {
                destination: Destination::Register(register),
            } => self.store_immediate(register, read_value),
            ReadImmediate8 {
                destination: Destination::Memory(addr),
            } => {
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
                value: a,
                operation,
            } => {
                let b = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue {
                            addr: self.registers.pc,
                        });
                    }
                };

                match operation {
                    AccumulatorOperation::Add => {
                        let result = self.add(a, b);
                        self.store_register(RegisterValue::A, result);
                    }
                    AccumulatorOperation::Sub => {
                        let result = self.sub(a, b);
                        self.store_register(RegisterValue::A, result);
                    }
                }

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = FetchOpcode;
                Ok(true)
            }
            ReadMemoryAccumulator {
                addr,
                value: a,
                operation,
            } => {
                let b = match read_value {
                    Some(byte) => byte,
                    None => {
                        return Err(CpuError::MissingReadValue { addr });
                    }
                };

                match operation {
                    AccumulatorOperation::Add => {
                        let result = self.add(a, b);
                        self.store_register(RegisterValue::A, result);
                    }
                    AccumulatorOperation::Sub => {
                        let result = self.sub(a, b);
                        self.store_register(RegisterValue::A, result);
                    }
                }

                self.state = FetchOpcode;
                Ok(true)
            }
            WriteMemory { .. } => {
                self.state = ExecutionState::FetchOpcode;
                Ok(true)
            }
            Faulted { opcode, addr } => Err(CpuError::UnsupportedOpcode(CpuFault { opcode, addr })),
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

        match opcode {
            DecodedOpcode::Nop => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ExecutionState::FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::LoadRegister {
                source,
                destination,
            } => {
                let value = self.register(source);

                self.store_register(destination, value);

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ExecutionState::FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::LoadImmediate { destination } => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadImmediate8 {
                    destination: Destination::Register(destination),
                };
                Ok(false)
            }
            DecodedOpcode::LoadImmediateWord { destination } => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadImmediate16Low { destination };
                Ok(false)
            }
            DecodedOpcode::LoadFromHL { destination } => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadMemory {
                    addr: self.registers.hl(),
                    destination,
                };
                Ok(false)
            }
            DecodedOpcode::StoreToHL { source } => {
                self.registers.pc = self.registers.pc.wrapping_add(1);

                let value = self.register(source);

                self.state = ExecutionState::WriteMemory {
                    addr: self.registers.hl(),
                    value,
                };
                Ok(false)
            }
            DecodedOpcode::StoreImmediateHL => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadImmediate8 {
                    destination: Destination::Memory(self.registers.hl()),
                };

                Ok(false)
            }
            DecodedOpcode::Increment(register) => {
                let mut value = self.register(register);
                value = self.increment(value);
                self.store_register(register, value);

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::Decrement(register) => {
                let mut value = self.register(register);
                value = self.decrement(value);
                self.store_register(register, value);

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::IncrementHL => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadOperation {
                    addr: self.registers.hl(),
                    operation: UnaryOperation::Increment,
                };
                Ok(false)
            }
            DecodedOpcode::DecrementHL => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadOperation {
                    addr: self.registers.hl(),
                    operation: UnaryOperation::Decrement,
                };
                Ok(false)
            }
            DecodedOpcode::AddRegister(register) => {
                let a = self.register(RegisterValue::A);
                let b = self.register(register);

                let result = self.add(a, b);
                self.store_register(RegisterValue::A, result);

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::SubRegister(register) => {
                let a = self.register(RegisterValue::A);
                let b = self.register(register);

                let result = self.sub(a, b);
                self.store_register(RegisterValue::A, result);

                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = FetchOpcode;
                Ok(true)
            }
            DecodedOpcode::AddHL => {
                let value = self.register(RegisterValue::A);
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadMemoryAccumulator {
                    addr: self.registers.hl(),
                    value,
                    operation: AccumulatorOperation::Add,
                };
                Ok(false)
            }
            DecodedOpcode::SubHL => {
                let value = self.register(RegisterValue::A);
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadMemoryAccumulator {
                    addr: self.registers.hl(),
                    value,
                    operation: AccumulatorOperation::Sub,
                };
                Ok(false)
            }
            DecodedOpcode::AddImmediate => {
                let value = self.register(RegisterValue::A);
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadImmediateAccumulator {
                    value,
                    operation: AccumulatorOperation::Add,
                };
                Ok(false)
            }
            DecodedOpcode::SubImmediate => {
                let value = self.register(RegisterValue::A);
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ReadImmediateAccumulator {
                    value,
                    operation: AccumulatorOperation::Sub,
                };
                Ok(false)
            }
            DecodedOpcode::Unknown => {
                let addr = self.registers.pc;
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ExecutionState::Faulted { opcode: byte, addr };
                Err(CpuError::UnsupportedOpcode(CpuFault { opcode: byte, addr }))
            }
        }
    }

    fn store_immediate(
        &mut self,
        register: RegisterValue,
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
        destination: RegisterValue,
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

    fn store_register(&mut self, destination: RegisterValue, value: u8) {
        match destination {
            RegisterValue::A => self.registers.a = value,
            RegisterValue::B => self.registers.b = value,
            RegisterValue::C => self.registers.c = value,
            RegisterValue::D => self.registers.d = value,
            RegisterValue::E => self.registers.e = value,
            RegisterValue::H => self.registers.h = value,
            RegisterValue::L => self.registers.l = value,
        }
    }

    fn register(&self, register: RegisterValue) -> u8 {
        match register {
            RegisterValue::A => self.registers.a,
            RegisterValue::B => self.registers.b,
            RegisterValue::C => self.registers.c,
            RegisterValue::D => self.registers.d,
            RegisterValue::E => self.registers.e,
            RegisterValue::H => self.registers.h,
            RegisterValue::L => self.registers.l,
        }
    }

    fn store_immediate_word(
        &mut self,
        destination: WordValue,
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
            WordValue::BC => self.registers.set_bc(value),
            WordValue::DE => self.registers.set_de(value),
            WordValue::HL => self.registers.set_hl(value),
            WordValue::SP => self.registers.set_sp(value),
        }

        self.state = ExecutionState::FetchOpcode;
        self.registers.pc = self.registers.pc.wrapping_add(1);

        Ok(true)
    }

    fn increment(&mut self, mut value: u8) -> u8 {
        let h = (value & 0x0F) == 0x0F;

        value = value.wrapping_add(1);

        let z = value == 0;
        self.registers.set_flag(registers::FlagValue::Z, z);
        self.registers.set_flag(registers::FlagValue::N, false);
        self.registers.set_flag(registers::FlagValue::H, h);

        value
    }

    fn decrement(&mut self, mut value: u8) -> u8 {
        let h = (value & 0x0F) == 0x00;

        value = value.wrapping_sub(1);

        let z = value == 0;
        self.registers.set_flag(registers::FlagValue::Z, z);
        self.registers.set_flag(registers::FlagValue::N, true);
        self.registers.set_flag(registers::FlagValue::H, h);

        value
    }

    fn add(&mut self, a: u8, b: u8) -> u8 {
        let (result, c) = a.overflowing_add(b);
        let z = result == 0;
        let h = ((a ^ b ^ result) & 0x10) != 0;

        self.registers.set_flag(registers::FlagValue::C, c);
        self.registers.set_flag(registers::FlagValue::Z, z);
        self.registers.set_flag(registers::FlagValue::N, false);
        self.registers.set_flag(registers::FlagValue::H, h);

        result
    }

    fn sub(&mut self, a: u8, b: u8) -> u8 {
        let (result, c) = a.overflowing_sub(b);
        let z = result == 0;
        let h = ((a ^ b ^ result) & 0x10) != 0;

        self.registers.set_flag(registers::FlagValue::C, c);
        self.registers.set_flag(registers::FlagValue::Z, z);
        self.registers.set_flag(registers::FlagValue::N, true);
        self.registers.set_flag(registers::FlagValue::H, h);

        result
    }
}
