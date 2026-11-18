pub mod cpu;
pub mod memory;
pub mod system;

#[cfg(test)]
use cpu::CycleOutcome as InstructionCycleOutcome;
#[cfg(test)]
#[path = "../tests/support/outcomes.rs"]
mod outcome_support;

#[cfg(test)]
use cpu::{CpuError, CycleAction, CycleRequest, CycleResponse, InterruptSample};
#[cfg(test)]
#[path = "../tests/support/responses.rs"]
mod response_support;

#[cfg(test)]
use system::{CycleTrace, MachineStep};
#[cfg(test)]
#[path = "../tests/support/steps.rs"]
mod step_support;
