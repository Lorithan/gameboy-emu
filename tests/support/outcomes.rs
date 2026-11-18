// Existing instruction oracles use a completion boolean. Reject interrupt entry
// explicitly so the migration cannot silently treat it as an unfinished cycle.
#[cfg(test)]
pub fn instruction_complete(outcome: super::InstructionCycleOutcome) -> bool {
    match outcome {
        super::InstructionCycleOutcome::InProgress => false,
        super::InstructionCycleOutcome::InstructionComplete => true,
        super::InstructionCycleOutcome::InterruptEntryComplete { vector } => {
            panic!("unexpected interrupt entry to {vector:04X} in instruction regression")
        }
        other => panic!("unexpected interrupt event {other:?} in instruction regression"),
    }
}
