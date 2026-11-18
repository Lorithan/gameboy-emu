use super::{CpuError, CycleAction, CycleResponse, InterruptSample};

// Preserve the existing instruction-data oracles while constructing explicit
// completion responses. None on a read deliberately exercises response rejection.
pub fn response(action: Result<CycleAction, CpuError>, data: Option<u8>) -> CycleResponse {
    match action {
        Ok(CycleAction::Write { .. }) => CycleResponse::WriteDone {
            interrupt_sample: InterruptSample {
                enable_after_bus: 0,
                flags_before_bus: 0,
            },
        },
        Ok(CycleAction::Idle) => CycleResponse::IdleDone,
        _ => match data {
            Some(value) => CycleResponse::Read {
                value,
                pending_interrupts: 0,
            },
            None => CycleResponse::IdleDone,
        },
    }
}
