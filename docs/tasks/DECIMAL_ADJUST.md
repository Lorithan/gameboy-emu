# Decimal adjust accumulator

Issue: [#27](https://github.com/Lorithan/gameboy-emu/issues/27). Prepared 2026-11-07.

## Goal
Implement 27 DAA after the accumulator arithmetic/rotate work. Baseline: cf357f7, 236 passing tests.

## Behavior contract
DAA adjusts A after packed-BCD arithmetic. One byte/one M-cycle (four ticks), completing during opcode fetch. Preserve N and unrelated registers/SP/memory; recompute Z from adjusted A, clear H. Incoming Z does not affect the adjustment.

For N=0, use original A/H/C: low correction 06 applies when H is set or the low nibble exceeds 9; high correction 60 applies when C is set or A exceeds 99 hex. Add the combined correction with byte wrapping. Outgoing C indicates the high correction.
For N=1, use original H/C only to select 06/60 corrections and subtract with byte wrapping. Preserve original C. Do not apply addition-path magnitude checks to subtraction.
Specify behavior for every A/N/H/C combination, including states not produced by valid BCD arithmetic.

## Architecture proposal
One dedicated decoded Daa operation handled during opcode completion. Snapshot A/N/H/C, compute the result and outgoing carry, then commit A/Z/H/C while preserving N. No new execution states, dependencies or Bus/System changes. A pure helper is optional; no generalized decimal-arithmetic framework.

## Acceptance criteria (assistant tests)
Independent mapping/all-byte classification; all 256 A values and 16 initial flags with independent expected behavior; input Z independence, N preservation, H clearing, subtraction C preservation, wrapping/retries and unrelated state preservation.
Independently validate all valid packed-BCD ADD/ADC/SUB/SBC operand pairs (00-99 decimal) and applicable carry/borrow inputs against decimal arithmetic, including 09+01=10, 99+01=00 with C, 10-01=09, 00-01=99 with C.
Verify exact one-fetch traces, budgets, full memory preservation and flags consumed by following arithmetic/branches. Full regressions, formatting and strict Clippy. A shared algorithm copied into tests is insufficient evidence.

User implements, assistant tests/docs/review. Start addition-mode behavior, then subtraction and carry/borrow boundaries; review as one focused issue. Excludes CB instructions, 16-bit arithmetic, interrupts, ROM fixtures and other instructions.

Source checked 2026-11-07: [RGBDS instruction reference, DAA](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7). Local contract: docs/tasks/DECIMAL_ADJUST.md.
## Review - 2026-11-08

Remaining correctness issue: correction arithmetic uses ordinary u8 +=/-= and panics with overflow checks enabled. A=9A/F=00 must yield A=00/F=90; A=00/F=70 must yield A=9A/F=50. Use explicit byte wrapping for every addition/subtraction correction. No production changes made by assistant.

Five new unit tests: debug run passes mapping and fails the other four tests on overflow/underflow. All five pass in release, covering all A/flag inputs at two PC locations (8,192 cases), retries/wrapping/preservation and 80,000 valid BCD ADD/ADC/SUB/SBC combinations compared against decimal arithmetic. Release success isolates the overflow issue but is not completion evidence for debug behavior. All 236 prior tests pass separately; formatting and strict Clippy pass. Public DAA cycle/budget/memory checks remain to be added before completion.
## Completion - 2026-11-08

User replaced every correction operation with explicit wrapping_add/wrapping_sub. No correctness findings remain. Fresh debug full-suite run passes all 244 tests (170 unit, 74 integration); formatting, strict Clippy and diff whitespace checks pass. This resolves the overflow blocker above.

Evidence: all 8,192 A/flag/PC input cases and 80,000 valid BCD arithmetic combinations pass with overflow checks enabled; independent decimal arithmetic verifies ADD/ADC/SUB/SBC sequences. Exact public opcode-fetch traces and one-cycle timing, zero/exact/excess budgets with no extra fetch, full-address-space preservation in both adjustment modes, and a 72-tick program using DAA flags for ADC and conditional branches pass. Existing regressions remain green. No external ROM suite was run.

Assistant changed tests/docs only; emulator behavior remains user-authored. Complete locally, publication pending.
