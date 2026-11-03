mod registers;

#[cfg(test)]
mod tests;

pub use registers::Registers;

use crate::cpu::{ExecutionState::FetchOpcode, registers::RegisterValue};

#[derive(Debug, Clone, Copy)]
enum ExecutionState {
    FetchOpcode,
    FetchImmediateA,
    Faulted { opcode: u8, addr: u16 },
}

#[derive(Debug, Clone)]
pub struct CPU {
    registers: Registers,
    state: ExecutionState,
}

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
    MissingReadValue { address: u16 },
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
            pc: self.registers.pc(),
            sp: self.registers.sp(),
        }
    }

    pub fn request_cycle(&self) -> Result<CycleAction, CpuError> {
        match self.state {
            ExecutionState::FetchOpcode => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ExecutionState::FetchImmediateA => Ok(CycleAction::Read {
                addr: self.registers.pc,
            }),
            ExecutionState::Faulted { opcode, addr } => {
                Err(CpuError::UnsupportedOpcode(CpuFault { opcode, addr }))
            }
        }
    }

    pub fn complete_cycle(&mut self, read_value: Option<u8>) -> Result<bool, CpuError> {
        match self.state {
            ExecutionState::FetchOpcode => self.read_opcode(read_value),
            ExecutionState::FetchImmediateA => self.store_immediate(RegisterValue::A, read_value),
            ExecutionState::Faulted { opcode, addr } => {
                Err(CpuError::UnsupportedOpcode(CpuFault { opcode, addr }))
            }
        }
    }

    fn read_opcode(&mut self, read_value: Option<u8>) -> Result<bool, CpuError> {
        let opcode = match read_value {
            Some(byte) => byte,
            None => {
                return Err(CpuError::MissingReadValue {
                    address: self.registers.pc,
                });
            }
        };

        match opcode {
            0x00 => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ExecutionState::FetchOpcode;
                Ok(true)
            }
            0x3E => {
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ExecutionState::FetchImmediateA;
                Ok(false)
            }
            _ => {
                let addr = self.registers.pc;
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.state = ExecutionState::Faulted { opcode, addr };
                Err(CpuError::UnsupportedOpcode(CpuFault { opcode, addr }))
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
                    address: self.registers.pc,
                });
            }
        };

        match register {
            RegisterValue::A => self.registers.a = value,
            RegisterValue::B => self.registers.b = value,
            RegisterValue::C => self.registers.c = value,
            RegisterValue::D => self.registers.d = value,
            RegisterValue::E => self.registers.e = value,
            RegisterValue::H => self.registers.h = value,
            RegisterValue::L => self.registers.l = value,
        }

        self.state = FetchOpcode;
        self.registers.pc = self.registers.pc.wrapping_add(1);

        Ok(true)
    }
}
