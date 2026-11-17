pub mod cpu;
pub mod memory;
pub mod system;

#[cfg(test)]
use cpu::CycleOutcome as InstructionCycleOutcome;
#[cfg(test)]
#[path = "../tests/support/outcomes.rs"]
mod outcome_support;
