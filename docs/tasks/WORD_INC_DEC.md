# Word INC and DEC

Issue: [#28](https://github.com/Lorithan/gameboy-emu/issues/28). Prepared 2026-11-08.

## Goal
Add INC/DEC for BC, DE, HL and SP. Baseline: published af8b12b, 244 passing tests. M2 CPU execution.

## Behavior
INC: 03 BC, 13 DE, 23 HL, 33 SP.
DEC: 0B BC, 1B DE, 2B HL, 3B SP.
Each changes the selected 16-bit value by one with wrapping, preserves every flag and all unrelated registers/memory, occupies one byte and takes two M-cycles/eight ticks. These operations do not use the byte INC/DEC flag rules.

## Proposed execution contract
Reuse Register16 (BC/DE/HL/SP) and an increment/decrement selector; exclude AF and PC. Decode without side effects.
During successful opcode completion advance PC once, capture destination and the original operand or resolved wrapped result, report incomplete. The second cycle requests Idle; only its completion commits the selected 16-bit result, returns to FetchOpcode and reports complete. Requests are immutable; missing opcode data leaves state untouched. No memory action beyond opcode fetch.
This final-cycle commit is our initial M-cycle model, not a claim about physical internal edges or interactions with future hardware. Revisit hardware-specific side effects when relevant devices are modeled.

Keep one focused pending word-update state carrying its captured destination/result rather than fabricating immediate reads or a general operation queue. Reuse ordinary register pair accessors; a small pair-selection helper is reasonable if it removes real duplication. Exact representation belongs to the user. No Bus/System changes or dependencies required.

## Acceptance criteria (assistant-owned)
Independent eight opcode mappings and all-byte supported classification; all 65,536 starting values per pair/direction using independent modular arithmetic; all initial flags at representative nibble/byte/word boundaries, especially FFFF->0000 and 0000->FFFF.
Verify no early destination change after opcode fetch, captured operand/destination, retry atomicity, PC wrapping, repeated immutable requests, preserved unrelated state. Exact Read/Idle traces, completion false/true, two-cycle timing, every zero/partial/exact/excess budget and resume without double increments; full memory preservation.
Exercise updated HL as a pointer and updated SP with stack operations; INC/DEC inverse sequences. Full regression suite, formatting and strict Clippy.

Begin INC BC, then DEC BC, then remaining pairs/SP. ADD HL, SP-relative arithmetic, CB operations, interrupts and devices remain separate issues. No ROM fixtures. User implements; assistant tests/docs/review.

Sources: [RGBDS INC/DEC reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7), [gbdev opcode tables](https://gbdev.io/gb-opcodes/optables/), checked 2026-11-08. Local contract: docs/tasks/WORD_INC_DEC.md.
## Review - 2026-11-08

Correctness finding: Unary16 completion commits its captured value and reports true but leaves the CPU in Unary16. Next request remains Idle, repeating completion instead of fetching the next opcode. Restore FetchOpcode after committing the pending update. The continuation/inverse/NOP regression fails on opcode 03.

Independent eight mappings and 524,288 pair/direction/value cases pass, including no early destination write and full snapshot preservation. All flags at boundaries, captured values under test-only mutation, retries, request immutability and PC wrapping pass. Four new tests were added; the continuation test fails. An assistant test import was corrected before running checks; obsolete byte INC/DEC unsupported assertions were updated. With the known continuation regression explicitly skipped, all other 247 tests pass (173 unit, 74 integration); formatting and strict Clippy pass. The full suite remains failing, not green. Public word-operation traces/budgets, memory preservation and HL/SP interaction coverage remain before completion. Assistant changed tests/docs only.
## Completion - 2026-11-08

User restored FetchOpcode after Unary16 commits. All 251 tests pass (175 unit, 76 integration); formatting and strict Clippy pass. No correctness findings remain. The 524,288 arithmetic cases, all-flag boundary/capture/retry/wrapping checks, continuation/inverse/NOP regression, exact Read/Idle traces and every budget pause/resume pass. Full-address-space preservation and a 156-tick program using updated HL and SP with memory transfers/PUSH/POP pass. No external ROM suite was run. Assistant changed tests/docs only; implementation remains user-owned.

User proposed returning state transitions to avoid independent completion/state updates. Architectural proposal and error/fault requirements are recorded in ../CPU_REFACTOR_REVIEW.md for a separate refactor; no transition refactor is included in this checkpoint. Complete locally, publication pending.