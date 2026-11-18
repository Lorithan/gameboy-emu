use super::{CpuError, CycleAction, CycleRequest, CycleResponse, InterruptSample};

// Existing instruction oracles require a real clocked action. Fail explicitly
// if a request unexpectedly suspends instead of erasing that distinction.
pub fn expect_execute(request: CycleRequest) -> CycleAction {
    match request {
        CycleRequest::Execute(action) => action,
        CycleRequest::Stopped => panic!("expected a real cycle, got a stopped clock"),
    }
}

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
