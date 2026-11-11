# Review of external CPU refactoring suggestions

Reviewed against checkpoint bee1ed0 on 2026-11-03. These are recommendations; the user owns implementation.

## Next focused refactor

Consolidate accumulator arithmetic operation and operand source as specified in [ACCUMULATOR_DECODE_REFACTOR.md](tasks/ACCUMULATOR_DECODE_REFACTOR.md). Explicit Add/Adc/Sub/Sbc/Compare kinds avoid invalid kind/carry combinations. Captured numeric carry belongs to pending execution input; resolve it at opcode completion and retain it across reads. The existing captured-carry tests are required evidence.

Bit-field decoding is appropriate for regular opcode families, but the 0x40–0xBF span contains unsupported logic groups as well as the HALT exception. Preserve the exact supported-byte set. Refactor the accumulator family first; byte-transfer/unary table compression can follow separately.

## Small independent cleanups that can be done now

- Merge identical request_cycle arms using or-patterns without changing actions or request immutability.
- Store CpuFault directly in Faulted, preserving original opcode/address and sticky-fault behavior.
- Expose read-only opcode/address accessors on CpuFault so hosts can inspect diagnostics. Public fields on this owned value would also be safe; accessors provide a stable interface.
- Narrow set_pc/set_sp to pub(super), matching the internal register mutation boundary. No current public integration test uses them; retain internal construction/wrapping coverage.
- Extract a required-read helper taking Option<u8> and the actual requested address. It must return MissingReadValue without mutation. Preserve the current protocol and captured-address error tests.
- Fix the program-counter documentation typo.

Keep these separate from the accumulator refactor when they alter public interfaces or error shape. Acceptance: existing tests/formatting pass, no new warnings; add public fault-accessor assertions and adapt private fault-state tests where appropriate. No need for new tests that merely mirror or-patterns or comment edits.

## Optional later changes

- TryFrom<u8> is a reasonable API for decoding only supported operations. From plus Unknown currently classifies every byte without panicking; it is not a correctness defect. A named decode function would also work. If changed, report unsupported/unimplemented separately from physically illegal instructions, preserve the raw byte/address in CPU diagnostics, and keep the same fault-cycle behavior. This does not remove the need for CPU error handling.
- A named CycleOutcome can clarify the public API when the next feature needs richer completion information. It is a readability improvement, not a transition guarantee.
- BusResponse with Read/WriteDone/IdleDone can improve protocol errors if needed. It still permits mismatched responses at the type level; the CPU must validate against its pending action at runtime and preserve state on rejection.
- Register-selector renaming and a private flag-bit setter helper are optional readability work. The current private flag representation and masks already enforce the low-nibble invariant. Avoid naming churn while another semantic refactor is underway.

## Transition ownership assessment

Removing Copy or consuming one ExecutionState enum does not constrain which variant a transition function can return. Both Copy and non-Copy versions can still construct any accessible enum variant. A consuming transition function may centralize code, but it does not itself prove legal state adjacency. Stronger compile-time transition restrictions would require per-state types/restricted transition APIs or another explicitly checked model.

Retain the current enum for now. Its payloads enforce the data available in each state; private CPU ownership, explicit transitions, and observable cycle tests check progression. Preserve retry atomicity and sticky faults if ownership style changes later.

## References

[Rust From documentation](https://doc.rust-lang.org/std/convert/trait.From.html) and [TryFrom documentation](https://doc.rust-lang.org/std/convert/trait.TryFrom.html), consulted 2026-11-03. From is intended for infallible conversions; choosing whether unsupported classification is represented as a variant or a decode error is an API decision here.

## Proposed follow-up: return cycle transitions - 2026-11-08

The user identified a real omission hazard after Unary16 reported instruction completion but stayed in its pending Idle state. Current handlers separately mutate ExecutionState and return a boolean; these can disagree.

Proposal for a separate user-owned refactor: internal handlers return an explicit transition outcome, such as Continue(next_state) or InstructionComplete, and a central coordinator assigns execution state and derives the existing completion boolean. InstructionComplete supplies FetchOpcode centrally and cannot carry an arbitrary next state. Merely returning (next_state, complete) would retain the contradictory combinations. Keep the existing externally observable cycle API while refactoring internals. This is a proposal, not an implementation decision or an instruction to refactor now; the user suggested revisiting after opcode work.

Handlers can still mutate registers after validated reads; returning a transition does not require cloning the entire CPU or staging every register write. Validate read responses before any mutation. MissingReadValue must leave registers and pending state unchanged for retry. UnsupportedOpcode must still latch the original opcode/address in Faulted and remain sticky; generic error propagation must not accidentally remove that state transition. The central outcome/error contract must express both behaviors explicitly before implementation.

This reduces forgotten-transition bugs and makes all successful paths choose an outcome. It does not prove valid adjacency: Continue can still contain the wrong state. Retain exact cycle, next-opcode, retry, fault, timing and budget tests. No typestate hierarchy, general micro-operation queue, new dependencies or Bus/System rewrite is justified for this fix. Keep the refactor separate from adding instructions; choose a checkpoint when the user is ready.

Acceptance evidence: behavior-equivalent snapshots, traces/timing, budgets/resume, missing-read atomicity, sticky original faults, and an explicit regression for completion followed by a different instruction. Existing instruction tests plus meaningful transition-invariant checks should cover the change; no need to mirror every enum arm in new tests.
## Deferred extraction: required reads and sequential operand fetches - 2026-11-10

User identified recurring execution-state code while implementing absolute A transfers. The pattern appears in address/word loads, jumps, calls and immediate operands: validate Some(byte), report MissingReadValue with the requested address on None, advance PC with wrapping only after success, then choose the next state. Little-endian word assembly also repeats.

Keep this as a separate user-owned cleanup. First candidate is a small read-validation helper returning Result<u8, CpuError> and taking the actual requested address; it must not mutate registers, execution state or time. A sequential-immediate-read helper may additionally advance PC exactly once after successful validation, if its name/contract clearly states that effect. Do not use that PC-advancing helper for ordinary memory/stack reads. Register writes, pending-state selection, sticky faults and completion remain explicit responsibilities until the separately planned transition refactor.

Use an existing endian conversion where it clarifies byte order; a custom helper for two shifts/ORs alone is not necessary. Shared low/high address stages with an explicit continuation could later consolidate the four absolute-transfer states, but a generic instruction framework is not justified. The four current states are accepted: explicit payloads avoid carrying store-only data through load states.

Acceptance: preserve every read-stage retry without mutation, captured low/store bytes, PC wrapping/advancement, exact request addresses and traces, timing/budget behavior and fault semantics. Do not combine a naming/helper cleanup with adding another opcode family. This records a deferred proposal; no emulator refactor was performed by the assistant.