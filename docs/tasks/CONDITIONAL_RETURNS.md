# Conditional returns

Issue: [#22](https://github.com/Lorithan/gameboy-emu/issues/22), M2 CPU execution. CALL/RET is committed locally as 49fbe24. Baseline: 205 tests pass (143 unit, 62 integration), formatting and Clippy pass without warnings.

## Scope and required behavior

| Opcode | Instruction | Condition |
| --- | --- | --- |
| C0 | RET NZ | Z clear |
| C8 | RET Z | Z set |
| D0 | RET NC | C clear |
| D8 | RET C | C set |

All are one-byte instructions. Taken returns cost five M-cycles (20 ticks); untaken returns cost two (8 ticks). No flags or general registers change. C9 unconditional RET retains its four-cycle behavior. Sources: [RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7) for timing/semantics and [Pan Docs encodings](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) for mappings.

Taken order: opcode read, condition-check Idle, stack-low read, stack-high read, target-apply Idle. Untaken order: opcode read, condition-check Idle, complete. Untaken must not read either stack byte, change SP or take a final target-apply Idle. PC advances once at opcode completion and remains at that sequential address unless a taken return commits its target in the final cycle. All PC/SP arithmetic wraps.

## Architecture proposal

Keep the existing decoded Return for C9 and add a conditional-return variant carrying JumpCondition. Resolve the decision with the existing helper during successful opcode completion, capturing both taken and current SP. Queue a pending state such as CheckReturn { taken, addr }.

CheckReturn requests Idle. Completion requires no read data and has no register/flag/memory side effects. If taken, queue existing ReadReturnLow { addr } and report incomplete. If untaken, return FetchOpcode and complete. Reuse ReadReturnLow, ReadReturnHigh and ApplyJump for the taken remainder. C9 enters ReadReturnLow directly, bypassing CheckReturn. This keeps the timing difference explicit and needs no generic scheduler, additional Bus behavior, or dependencies.

State/variant names are implementation choices. Capturing the decision at opcode completion follows the existing JR/JP/CALL project convention. The extra Idle is the observable condition-check phase in this initial M-cycle model; it does not claim physical edge-level register timing. Do not reevaluate live flags at Idle completion.

Repeated requests are immutable. Missing opcode data causes no capture or PC change. Taken stack-read failures report the captured address and preserve pending byte/address, PC/SP/registers for retry. Budget pauses retain the decision and initial stack address even before the first stack read. SP advances only after successful stack reads, as for C9. Memory always uses ordinary bus routing.

## Assistant-owned acceptance evidence

- Independent four-opcode mappings and updated all-byte classification; C9 still bypasses the extra Idle and RETI/RST remain unsupported.
- Every flag combination for every condition and all conditional return targets, with independent expected decisions/byte assembly.
- Captured decision/stack address across test-only live flag/SP changes, repeated requests and pauses; flags/general-register preservation.
- Exact taken Read/Idle/Read/Read/Idle and untaken Read/Idle traces, completion markers, no read values on Idle, SP/PC snapshots, no extra fetch or untaken stack access.
- Zero/partial/exact/excess budgets and resumes after every phase; per-call counts; missing opcode/low/high reads and successful retry.
- PC/SP wrapping, normal stack bus boundaries, and full memory preservation on both outcomes.
- Conditional returns in nested HRAM/WRAM subroutines: false conditions execute the next callee instruction, while true conditions restore the appropriate caller PC/SP without corrupting the remaining stack.
- Full CALL/RET/PUSH/POP/jump/arithmetic/bus/fault regressions, formatting and Clippy. Synthetic programs only; no ROM fixtures or dependencies.

## First slice and exclusions

Implement RET NZ (C0) first, verifying both outcomes and five/two-cycle timing before adding the other three mappings. User writes emulator implementation; assistant writes tests and reviews. RST, RETI and interrupts are separate tasks.

## Completion review — 2026-11-04

All four mappings and the ReturnConditional Idle state follow the contract. Captured decision/SP feed the existing return path only when taken; C9 retains its four-cycle path. No correctness findings remain. All 211 tests pass (147 unit, 64 integration); formatting and Clippy pass without warnings.

New independent mappings, every target with both decisions across all four opcodes (524,288 cases), all flag combinations with PC/SP boundaries, captured decisions/addresses across test-only flag/SP changes, repeated requests and missing-read retries pass. Public traces prove five/two-cycle behavior, no untaken stack access, correct snapshots and every budget pause/resume point. Nested HRAM/WRAM programs continue inside the callee after false returns and unwind correctly after true returns, completing in 112 ticks.

A full-address-space oracle verifies memory preservation across all four forms, both outcomes, stack bus boundaries and every budget pause. Ordinary bus data determines taken targets, including unmapped/ROM reads and wrapping; untaken SP stays unchanged. Existing CALL/RET and other regressions pass. One old comparison decoder test still treated C0 as unsupported; the assistant removed that obsolete expectation while retaining its unrelated rejection checks. User wrote implementation; assistant changed tests/docs only. Committed locally as 40e728a; not yet pushed. RST is the next ordinary control-flow task; RETI remains tied to interrupts.
