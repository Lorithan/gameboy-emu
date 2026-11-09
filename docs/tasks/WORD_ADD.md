# Word addition into HL

Issue: [#29](https://github.com/Lorithan/gameboy-emu/issues/29). Prepared 2026-11-08.

## Goal
Implement ADD HL,BC (09), ADD HL,DE (19), ADD HL,HL (29), ADD HL,SP (39). M2 CPU execution; baseline 701b329, 251 passing tests.

## Behavior
Add the source to original HL, wrap to 16 bits and store in HL. Preserve Z; clear N; set H for carry from bit 11 into bit 12 and C for carry beyond bit 15. Ignore incoming C. Preserve A, BC, DE, SP and memory. ADD HL,HL doubles the original value, with both inputs aliased correctly.
Each form is one byte/two M-cycles/eight ticks: opcode read then Idle. No additional operand reads or writes.

## Architecture proposal
Reuse Register16 as the source selector; destination is always HL. A dedicated decoded operation is sufficient; do not route through byte accumulator arithmetic.
On successful opcode completion, increment PC once and capture original HL/source and preserved Z, or the resolved result/flag values. Report incomplete with HL and flags unchanged.
The pending operation requests Idle; its successful completion commits HL/N/H/C while preserving captured Z, restores FetchOpcode and reports complete. Preserve no-mutation missing-opcode retries, request immutability and pause/resume behavior. This atomic final-cycle commit is our initial M-cycle convention, not a hardware-edge claim.
Use a focused pending word-add state, or a constrained shared word-update representation if it simplifies real duplication. Do not force flag-updating addition through an unmodified Unary16 state. Exact enum/helper representation belongs to user. The proposed transition-return refactor remains a separate deferred change; no Bus/System changes or dependencies needed.

## Acceptance criteria (assistant tests)
Independent four mappings/all-byte classification. Independent widened sum and low-12-bit sum oracle; carry boundaries 000F/0010, 00FF/0100, 0FFF/1000, 7FFF/8000, FFFF/0000; all initial flags; Z preserved even when result disagrees with Z. Incoming C ignored.
Sweep every value of each operand against representative boundary values of the other for BC/DE/SP; exhaust every valid self-source HL value. Do not claim exhaustive all-pair coverage of two independent 16-bit inputs.
Capture both inputs/flags under test-only mutation, missing-opcode retries, wrapping PC, no early result/flag writes, immutable requests. Exact Read/Idle traces and false/true completion, every budget and resume point, full memory preservation, next-opcode continuation.
Mixed pair arithmetic with updated HL used for memory accesses and Z/C consumed by following branches. Full regressions, formatting, strict Clippy.

Start ADD HL,BC, then DE/SP and self-source HL. Excludes ADD SP,e8, LD HL,SP+e8, CB and interrupt work. User implements; assistant tests/docs/review. No ROM fixtures.

Reference checked 2026-11-08: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7), ADD HL,r16 and ADD HL,SP sections. Local contract: docs/tasks/WORD_ADD.md.
## Review and completion - 2026-11-08

Initial review found H mask 0800 rather than 1000 and recomputation of Z. User corrected both during review, removed the unused z local, and renamed ClearCarry to ComplementCarry so CCF naming matches its toggle behavior. No correctness findings remain. Final suite: all 259 tests pass (181 unit, 78 integration); formatting and strict Clippy pass. Assistant corrected new test-module ordering for formatting. Emulator implementation and descriptive opcode renames remain user-authored.

The XOR expression identifies carry into each bit position: H for ADD HL uses bit 12 (1000). Examples 0800+0800 sets H, while 0400+0400 and 07FF+0001 do not. 0FFF+0001 and FFFF+0001 set H. Incoming Z remains unchanged even for zero/nonzero results that disagree with it.

Evidence: independent four mappings/all-byte classification; 5,177,344 widened-sum/low-12-bit oracle cases sweeping both independent operands against 13 boundaries and all self-source values (not every independent 16-bit pair). All initial flags at selected boundaries; captured operands under private test-only mutation; missing-opcode retries, PC wrap, no early result/flag writes and immutable Idle requests pass. Exact Read/Idle traces, zero/partial/exact/excess budgets, next-opcode/NOP continuation and full-address-space memory preservation pass. A 104-tick program uses updated HL as a pointer and consumes preserved Z/new C in branches. No external ROM suite was run.

Contract clarification: Z is preserved by leaving the flag untouched; no separate captured Z payload is needed while CPU exclusively owns flags and no valid operation can mutate them between these cycles. The tests deliberately mutate captured numeric inputs, not invariant unrelated flags. Capture flags explicitly if a future completion helper replaces their entire representation.

Complete locally, pending commit/publication. See tests in src/cpu/tests/word_add.rs and tests/word_add.rs.