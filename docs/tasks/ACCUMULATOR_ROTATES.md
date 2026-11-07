# Accumulator rotates

Issue: [#26](https://github.com/Lorithan/gameboy-emu/issues/26). Contract prepared 2026-11-06; implementation remains user-owned.

## Goal
Add the four base accumulator rotates after #25. Baseline: 248330a, 231 passing tests.

## Behavior
- 07 RLCA: rotate A left circularly; original bit 7 becomes bit 0 and C.
- 0F RRCA: rotate A right circularly; original bit 0 becomes bit 7 and C.
- 17 RLA: rotate A left through carry; original C becomes bit 0, original bit 7 becomes C.
- 1F RRA: rotate A right through carry; original C becomes bit 7, original bit 0 becomes C.
All clear Z/N/H unconditionally, including when A becomes zero. Circular forms ignore original carry. Each is one byte/one M-cycle (four ticks), preserves other registers/SP/memory, and completes during opcode fetch. PC advances once with wrapping; no extra reads/writes/Idle.

## Architecture proposal
Use a constrained accumulator-rotate selector for direction and circular/through-carry behavior, or four explicit decoded variants. Keep targets limited to A. Compute from original A and original carry before committing either output. No new execution states or Bus/System changes. Do not force these through binary accumulator operand paths.
A small helper may return result plus outgoing carry, with the caller owning flag rules. This can later serve CB operations without conflating Z semantics; do not build CB support now.

## Acceptance criteria (assistant tests)
Independent four mappings/all-byte classification; every A value with all 16 initial flag combinations; original-carry dependence/independence, zero result always Z=0, repeated rotations, unrelated state preservation; missing-read retries and PC wrap; exact public one-fetch traces, zero/exact/excess budgets; full memory preservation; carry-dependent arithmetic and branches; full regression suite, formatting, strict Clippy.

Start RLCA, then RRCA, then RLA/RRA. Excludes CB prefix/other targets, shifts, DAA and interrupts. User implements; assistant owns tests/docs/review. No dependencies or ROM fixtures.

Reference: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7). Local contract: docs/tasks/ACCUMULATOR_ROTATES.md.
## Review - 2026-11-07

All four operations are correct in the reviewed implementation. Each computes its result and outgoing bit from original A, reading original C before any C assignment. No correctness findings. All 236 tests pass (164 unit, 72 integration); formatting, strict Clippy and diff whitespace checks pass. Source implementation remains user-authored; assistant changed tests/docs only.

Evidence: independent four mappings and all-byte classification; every A/all 16 flags at three PC locations including wrap (49,152 starting cases), each followed through nine repeated rotations against an independent widened-arithmetic oracle; missing-read retry atomicity, repeated requests, original carry dependence/independence and unrelated-state preservation. Public one-fetch traces, zero/exact/excess budgets, full-address-space preservation and arithmetic/branch interaction pass. The new integration test's expected sentinel address was corrected to 0118 during development. No external ROM suite was run.

Optional readability proposal, not a correctness gate: snapshot carry_in beside A, let the direction match return (result, carry_out), then commit A/C and clear Z/N/H in one shared block. Rename selector carry to through_carry to distinguish instruction mode from input flag value. A helper is optional while only this handler uses the operation; later CB reuse should keep flag policy separate.

Acceptance criteria satisfied locally; pending commit/publication.

## Readability refactor review - 2026-11-07

User snapshots carry_in beside A; direction branches now return (result, carry_out) and one shared block commits A/C and clears N/H/Z. Selector field is through_carry, and mapping tests were updated for that name. This implements the optional proposal and keeps execution behavior unchanged. Fresh full suite: 236 tests pass; formatting and strict Clippy pass. No further refactor is required for this task.
