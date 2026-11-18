use crate::{cpu::CpuError, memory::Interrupt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleAction {
    Idle,
    Read { addr: u16 },
    Write { addr: u16, value: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    InProgress,
    InstructionComplete,
    InterruptSelected { interrupt: Interrupt },
    InterruptSelectionCancelled,
    InterruptEntryComplete { vector: u16 },
    InterruptEntryCancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptSample {
    pub enable_after_bus: u8,
    pub flags_before_bus: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleResponse {
    Read { value: u8, pending_interrupts: u8 },
    WriteDone { interrupt_sample: InterruptSample },
    IdleDone,
}

impl CycleResponse {
    pub fn kind(&self) -> CycleKind {
        match self {
            CycleResponse::Read { .. } => CycleKind::Read,
            CycleResponse::WriteDone { .. } => CycleKind::Write,
            CycleResponse::IdleDone => CycleKind::Idle,
        }
    }

    pub fn read_value(&self) -> Result<u8, UnexpectedCycleResponse> {
        match self {
            CycleResponse::Read { value, .. } => Ok(*value),
            _ => Err(UnexpectedCycleResponse {
                expected: CycleKind::Read,
                received: self.kind(),
            }),
        }
    }

    pub fn pending_interrupts(&self) -> Result<u8, UnexpectedCycleResponse> {
        match self {
            CycleResponse::Read {
                pending_interrupts, ..
            } => Ok(*pending_interrupts),
            _ => Err(UnexpectedCycleResponse {
                expected: CycleKind::Read,
                received: self.kind(),
            }),
        }
    }

    pub fn interrupt_sample(&self) -> Result<InterruptSample, UnexpectedCycleResponse> {
        match self {
            CycleResponse::WriteDone { interrupt_sample } => Ok(*interrupt_sample),
            _ => Err(UnexpectedCycleResponse {
                expected: CycleKind::Write,
                received: self.kind(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleKind {
    Read,
    Write,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnexpectedCycleResponse {
    pub expected: CycleKind,
    pub received: CycleKind,
}

impl From<UnexpectedCycleResponse> for CpuError {
    fn from(err: UnexpectedCycleResponse) -> Self {
        CpuError::UnexpectedCycleResponse(err)
    }
}
