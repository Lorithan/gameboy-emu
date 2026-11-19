use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcodeEncoding {
    Base(u8),
    Cb(u8),
}

impl Display for OpcodeEncoding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OpcodeEncoding::Base(value) => write!(f, "opcode: 0x{value:02X}"),
            OpcodeEncoding::Cb(value) => write!(f, "cb opcode: 0x{value:02X}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CpuFault {
    pub opcode: OpcodeEncoding,
    pub addr: u16,
}

impl Display for CpuFault {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, address: 0x{:04X}", self.opcode, self.addr)
    }
}

impl Error for CpuFault {}

#[derive(Debug, Clone, Copy)]
pub enum CpuError {
    UnsupportedOpcode(CpuFault),
    UnexpectedCycleResponse(UnexpectedCycleResponse),
    NoCycleRequested,
}

impl Display for CpuError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuError::UnsupportedOpcode(_) => {
                write!(f, "cannot execute CPU instruction")
            }
            CpuError::UnexpectedCycleResponse(_) => write!(f, "cannot complete CPU cycle"),
            CpuError::NoCycleRequested => {
                write!(f, "cannot complete a cycle while the CPU is stopped")
            }
        }
    }
}

impl Error for CpuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CpuError::UnsupportedOpcode(cpu_fault) => Some(cpu_fault),
            CpuError::UnexpectedCycleResponse(unexpected_cycle_response) => {
                Some(unexpected_cycle_response)
            }
            CpuError::NoCycleRequested => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnexpectedCycleResponse {
    pub expected: super::CycleKind,
    pub received: super::CycleKind,
}

impl From<UnexpectedCycleResponse> for CpuError {
    fn from(err: UnexpectedCycleResponse) -> Self {
        CpuError::UnexpectedCycleResponse(err)
    }
}

impl Display for UnexpectedCycleResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "expected {} response, received {} response",
            self.expected, self.received
        )
    }
}

impl Error for UnexpectedCycleResponse {}
