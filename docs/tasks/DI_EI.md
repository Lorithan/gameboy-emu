# DI/EI and IME lifecycle

GitHub task: [#37](https://github.com/Lorithan/gameboy-emu/issues/37)

## Goal
Implement DI (F3), EI (FB), and CPU-owned interrupt master enable (IME) lifecycle as a small M2 instruction task. This does not implement interrupt dispatch or hardware IF/IE registers.

## Proposed architecture
- CPU owns private IME and any delayed-enable bookkeeping. IME is separate from the F register. Startup at cartridge entry has IME disabled and no pending enable.
- Keep current CPU/System/Bus ownership and public stepping semantics. No new execution states should be needed for these two one-cycle instructions.
- Advance delayed enable once per successfully completed instruction, centrally in CPU completion handling. Do not tie it to reads, bytes, individual M-cycles, or System::step_instruction (direct CPU stepping must behave identically).
- Exact private representation is the user's choice. Public snapshot expansion is not required for this task; assistant unit tests may inspect private CPU state.
- IF/IE storage, arbitration/acknowledgment, interrupt service cycles, RETI, HALT and STOP require subsequent scoped contracts.

## Required behavior
- DI: one byte, one M-cycle/four ticks. Disable IME at successful completion and cancel any pending enable.
- EI: one byte, one M-cycle/four ticks. From disabled, leave IME disabled until the following whole instruction completes. Do not disable already-enabled IME.
- EI; DI never enables IME.
- EI; EI enables IME at completion of the second EI: a new EI must not postpone the earlier pending enable.
- A CB prefix and its second byte belong to one instruction. Multi-cycle instructions mature the delay only on final completion.
- Preserve general registers, F and SP; advance PC by one for DI/EI, with wrapping.
- Missing read data, repeated request queries, zero-budget steps, incomplete cycles, and sticky faults do not mature the pending enable.

## Acceptance criteria (assistant owns tests)
- Explicit decoder mapping, startup state, all initial F patterns and boundary PCs.
- DI from enabled/disabled/pending states; EI from enabled/disabled states.
- EI; NOP, EI; DI, EI; EI, repeated DI/EI sequences.
- Pause after each cycle of a following immediate instruction and CB register/BIT-memory/modifying-memory instructions; check enable only at instruction completion.
- Missing-read retry, queries, zero-budget calls and unsupported base-opcode fault preserve pending bookkeeping appropriately.
- Public System traces verify one-cycle timing, PC, register/flag preservation and resumable sequences. Private CPU tests observe IME until interrupt dispatch is introduced.
- Existing suite, formatting and strict Clippy pass. No dependency or ROM fixture additions.

## References
- [Pan Docs interrupts](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupts.md)
- [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7)
- [Mooneye EI sequence source](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/ei_sequence.s): reference only; suite not run or imported.

The user owns emulator implementation; the assistant owns architecture guidance, tests, review and project records. Baseline: commit 6494b7a, 326 passing tests (228 unit, 98 integration), formatting and strict Clippy pass.

## Completion evidence - 2026-11-14

DI/EI and CPU-owned Disabled/Delayed/Enabled IME lifecycle are verified locally. complete_cycle_inner is private, so public cycle completion always runs instruction-boundary bookkeeping. The user implemented application changes; the assistant added eight tests, minimal wiring, a test-only IME observer, and updated opcode expectations. Production IME state remains private.

The last full run passed all 334 tests (236 unit, 98 integration), formatting and strict Clippy. After the visibility-only fix, all eight focused DI/EI tests, formatting and strict Clippy pass. Coverage includes explicit decoding/startup, all initial flag patterns and wrapping PCs, all seven-instruction NOP/DI/EI sequences from each IME state, every supported base instruction and every CB encoding as the instruction following EI, missing reads, queries, sticky faults, System budgets and exact single-cycle traces. No external ROM suite was run; interrupt dispatch is still outside scope.

Next: specify IF/IE storage and the CPU/System interrupt interface before implementing interrupt service, RETI and HALT. STOP remains separately scoped. Implementation coverage is 241/244 non-prefix base and 256/256 CB, leaving RETI, HALT and STOP. Changes are uncommitted; publication not verified.
