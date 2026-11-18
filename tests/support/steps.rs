use super::{CycleTrace, MachineStep};

// Instruction and interrupt regressions require actual elapsed cycles. Keep
// their existing trace oracles and fail if a step unexpectedly stops instead.
pub fn expect_cycle(step: MachineStep) -> CycleTrace {
    match step {
        MachineStep::Cycle(trace) => trace,
        MachineStep::Stopped => panic!("expected a real cycle trace, got a stopped clock"),
    }
}
