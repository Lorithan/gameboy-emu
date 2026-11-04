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
