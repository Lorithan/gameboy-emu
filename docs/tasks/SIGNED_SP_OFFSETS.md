# Signed SP offsets

Issue: [#30](https://github.com/Lorithan/gameboy-emu/issues/30). Prepared 2026-11-08.

## Goal
Implement signed SP-offset operations: F8 LD HL,SP+e8 and E8 ADD SP,e8. Baseline 68e8cfe, 259 passing tests. M2 CPU execution.

## Behavior
Treat the operand byte as a signed offset -128..127 for the 16-bit wrapped result.
F8 writes HL and preserves SP: two bytes/three M-cycles (12 ticks).
E8 writes SP and preserves HL: two bytes/four M-cycles (16 ticks).
Both clear Z/N; H is carry from bit 3, C from bit 7 of original SP plus the raw unsigned operand byte. These are low-byte addition flags even for negative offsets; do not use ADD HL's bit-11/15 rules or treat C as full 16-bit overflow. Incoming flags do not affect the result.
Example SP=0000 plus FF (-1) -> FFFF, H=0/C=0; SP=0001 plus FF ->0000, H=1/C=1. Preserve A/BC/DE and memory.

## Proposed cycle contract
F8: opcode Read, immediate Read, Idle. E8: opcode Read, immediate Read, Idle, Idle.
Capture original SP at successful opcode completion. Increment PC once at opcode completion and once at successful operand completion, with wrapping. After operand read retain the signed result, flags and operation destination/timing; destination and flags stay unchanged until final Idle completion.
Final completion commits destination/Z/N/H/C, restores FetchOpcode and reports true; earlier cycles report false. Preserve retry atomicity and captured data through pauses/repeated requests. This commit phase is the project M-cycle convention, not physical-edge evidence.

## Architecture proposal
Share the signed-SP arithmetic calculation and represent only the two supported modes (HL destination with one internal cycle, SP destination with two). Keep mode/destination/timing consistent by construction. No general arbitrary-register signed-add framework, Bus/System changes or dependencies. Shared arithmetic is suitable; unmodified byte or ADD HL helpers have different effects and flag rules. Implementation representation belongs to user; transition-return refactor remains deferred.

## Assistant acceptance tests
Independent mappings/all-byte classification; exhaustive 65,536 SP values x 256 raw offsets for each opcode, using signed widened modular result and unsigned nibble/byte flag oracles. All initial flags at representative boundaries; +0,+1,+127,-1,-128 and word/byte/nibble wraps.
Captured SP/operand/result, immutable requests, missing opcode/operand retry atomicity, PC wrapping, no early flag/destination changes. Exact traces, timing, completion markers, every budget pause/resume, continuation to a different next opcode and full memory preservation. HL pointer and SP stack interaction programs. Full regressions, formatting and strict Clippy.

Start F8 LD HL,SP+e8, then E8 ADD SP,e8. F9 LD SP,HL is a separate load task. Excludes interrupt/CB/device work and ROM fixtures. User implements; assistant tests/docs/review.

References checked 2026-11-08: [RGBDS](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7), [gbdev opcode data](https://raw.githubusercontent.com/gbdev/gb-opcodes/master/Opcodes.json). Local contract: docs/tasks/SIGNED_SP_OFFSETS.md.
## Review - 2026-11-10

Three correctness findings: increment PC once after successful ReadSpOffset; H sums only low four bits against 0F; C sums low eight bits against FF. Current H/C calculations are identical and wrong for these boundaries: SP=0010/raw=00 requires H=0; SP=000F/raw=01 requires H=1/C=0. The operand read currently leaves PC at 0101 rather than 0102.

Six new unit tests: mapping and atomic retry/captured-SP Idle progression pass; PC, H, C and full operand oracle fail. The exhaustive test stops at F8/SP=0000/raw=10, so do not claim exhaustive validation yet. All 259 prior tests pass with the six new tests explicitly skipped. Formatting passes. Strict Clippy fails on two identity operations (masking an already-u8 value with FF). No production changes made by assistant. Public traces/budgets/memory and pointer/stack integration checks remain before completion.

Design note: retaining captured SP and offset and calculating at final completion is behaviorally acceptable; the proposed earlier calculation is not required when inputs remain immutable. A descriptive two-case operation mode would be clearer than hl: bool, but this naming improvement is optional and does not block the arithmetic fixes.
## Completion - 2026-11-10

User corrected operand PC advancement, masks each nibble before summing H, and widens the raw unsigned offset byte for C. The signed result still uses captured SP plus signed offset. All 269 tests pass (189 unit, 80 integration); formatting, strict Clippy and diff whitespace checks pass. No correctness findings remain. Assistant changed tests/docs only; implementation remains user-authored.

Evidence: all 33,554,432 SP/raw-offset/opcode combinations pass independent signed widened modular result and unsigned nibble/byte flag oracles. All initial flags at representative positive/negative and wrap boundaries, opcode/operand retries, PC wrap, original-SP capture under test-only mutation and immutable Idle requests pass. Exact public Read/Read/Idle and Read/Read/Idle/Idle traces verify no early destination or flag updates. Every zero/partial/exact/excess budget resumes once, completes at the correct cycle, and runs a following NOP. All-address-space memory preservation holds at every pause; a 148-tick HL-pointer/SP-stack program passes. No external ROM suite was run.

Retaining captured SP/offset and computing at final completion is accepted. The optional operation-mode naming improvement is deferred. Complete locally, publication pending.