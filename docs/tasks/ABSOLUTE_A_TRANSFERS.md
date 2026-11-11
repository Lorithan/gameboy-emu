# Absolute-address accumulator transfers

Issue: [#33](https://github.com/Lorithan/gameboy-emu/issues/33). Prepared 2026-11-10.

## Goal
Implement EA LD (a16),A and FA LD A,(a16). Baseline 41da2bb, 282 passing tests. M2 CPU execution.

## Behavior
The two bytes following the opcode encode an unsigned 16-bit address, low byte first. EA writes A to that memory address; FA reads one byte from it into A. Example EA 34 C1 stores A at C134.
Both occupy three bytes/four M-cycles (16 ticks), preserve all flags and unrelated registers/SP, and use ordinary Bus mapping. The a16 operand is an address, not data copied into A. No branch or extra Idle.

## Architecture/cycle contract proposal
Use explicit decoded load/store operations with A fixed. Share a low-address-read stage and a high-address-read stage carrying a constrained transfer continuation. Capture A at successful opcode completion for stores; retain it through both operand reads. Preserve the buffered low byte through retries.
Cycles: opcode Read, address-low Read, address-high Read, data Read/Write. Advance PC once at each successful instruction-byte completion, wrapping; never advance PC on the final data access. First three completions report false.
After the high byte, assemble the address and enter existing ReadMemory (destination A) or WriteMemory (captured A). Final completion updates A or acknowledges the single write, restores FetchOpcode and reports true. No early register/memory changes other than instruction PC progression. Missing opcode/address/load data leaves pending state/registers unchanged.
Two focused address-read states with an explicit load/store continuation fit the current design. Do not reuse jump states with fictitious branch conditions or broaden immediate-register destinations to mean memory accesses. No new final data-cycle states, Bus/System changes, dependencies or transition-return refactor expected. Exact representation remains user-owned.

## Acceptance criteria (assistant tests)
Independent two mappings/all-byte classification; all 65,536 target addresses per opcode with independent byte-order oracle; all byte data values and flags at representative mapped addresses.
Snapshots and exact public four-action traces, no early writes/A changes, completion false/false/false/true, PC wrap at each instruction-byte position. Captured A/low address under test-only mutation, repeated requests, missing-data retries at every read, every zero/partial/exact/excess budget/resume and following opcode continuation.
ROM/WRAM/echo/HRAM/unmapped boundaries; full-memory preservation on loads, only permitted writes/echo aliases for stores, no operand-address confusion. Round trips and mixed absolute/pair/HL transfer programs. Full regression suite, formatting and strict Clippy.

Start EA store, then FA load. LDH, LD (a16),SP, LD SP,HL, interrupts and CB remain separate issues. User implements; assistant owns tests/docs/review. No ROM fixtures.

Reference checked 2026-11-10: [gbdev opcode data](https://raw.githubusercontent.com/gbdev/gb-opcodes/master/Opcodes.json), unprefixed EA/FA entries. Local contract: docs/tasks/ABSOLUTE_A_TRANSFERS.md.
## Completion - 2026-11-10

All 289 tests pass (201 unit, 88 integration); formatting and strict Clippy pass. No correctness findings. Four separate low/high states instead of the proposed shared pair are accepted: explicit load/store payloads retain the right captures without requiring a broader continuation abstraction. Existing final ReadMemory/WriteMemory states are reused; no Bus/System changes.

Evidence: independent mappings/all-byte classification, all 131,072 opcode/target combinations and little-endian byte-order checks; all flags, no early register/memory effects, captured original A through private test-only mutation; retries at opcode/low/high/data reads and PC wrapping at every instruction byte. Exact four-cycle public traces, all budgets/resumes, following-opcode continuation and every-byte mapped-memory round trips pass. Full-address-space memory oracle verifies preservation/permitted writes and echo aliases across bus boundaries, including a target overlapping operand ROM. A 116-tick program combines absolute, pair and auto-updating HL transfers. All prior regressions pass. No external ROM suite was run. Assistant changed tests/docs only.

The user's deferred read-validation/PC-advancement/endian extraction proposal is recorded in ../CPU_REFACTOR_REVIEW.md. Keep it separate from this completed behavior change. Complete locally, publication pending.