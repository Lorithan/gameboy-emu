# Immediate byte-register loads

## Goal
Extend the existing LD A, immediate path to all seven byte registers: B, C, D, E, H, L, and A. Milestone: **M2 — CPU execution**. Published prerequisite: #5, checkpoint 3a388e9.

## Scope
| Opcode | Destination |
| --- | --- |
| 0x06 | B |
| 0x0E | C |
| 0x16 | D |
| 0x1E | E |
| 0x26 | H |
| 0x2E | L |
| 0x3E | A (existing behavior) |

Each instruction is two bytes, takes two M-cycles/eight base ticks, and preserves flags. Opcode 0x36 writes an immediate to memory at HL and is deliberately excluded; keep it unsupported for this task.

## Architecture contract
Replace the A-specific pending execution state with one immediate-fetch state carrying a destination RegisterValue. The opcode fetch selects that destination; operand completion uses the retained destination and the existing store_immediate helper.

- Requesting either opcode or operand data is immutable and reads the current PC.
- Opcode completion increments PC once, records the destination, leaves destination/flags unchanged, and reports incomplete.
- Operand completion increments PC once, writes only the selected register, returns to opcode fetch, and reports complete.
- Use wrapping PC increments. Preserve existing error, sticky-fault, trace, and budget behavior.
- The destination selector may derive Debug/Clone/Copy as required by the enclosing state. Keep it internal; do not expose execution state or add a universal instruction abstraction.
- No System scheduler redesign, new dependencies, memory-target loads, flags arithmetic, or extra instruction families.

## Acceptance criteria and assistant-owned tests
- [ ] All existing tests still pass; assistant migrates state-specific tests after the internal state change.
- [ ] Every listed opcode writes every possible byte value correctly (00 through FF); verify independent opcode-to-destination expectations.
- [ ] Only the destination changes; the paired register's other byte, other pairs, flags, SP, and unrelated state remain intact.
- [ ] All valid initial flag combinations remain unchanged.
- [ ] On the first cycle, destination and flags are unchanged; on the second, only the selected register is updated. PC advances once each cycle.
- [ ] Public traces show opcode/operand addresses and bytes, four-tick spans, and false/true completion. No prefetch beyond the operand.
- [ ] Budget-one exhaustion preserves each destination for a later call; resuming consumes the operand without refetching.
- [ ] CPU unit tests verify PC wrapping and that missing read data preserves the selected destination for retry.
- [ ] 0x36 and an unrelated opcode still produce the existing unsupported-opcode diagnostic and fault behavior.
- [ ] Formatting passes; record strict Clippy results without suppressing unused-code warnings or expanding unrelated scope.

## Reference
[gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json) and [Pan Docs instruction groups](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md), consulted 2026-11-02. Tests will use explicit expected mappings, not derive their expectations from the production decoder.

The user implements; the assistant maintains tests and reviews architecture. Start by changing the pending state to carry a destination, keep A working through it, then add the other opcode mappings.

## Verification

The generalized FetchImmediate(RegisterValue) state and all seven mappings are implemented. Migrated state-specific tests and added two CPU unit tests plus four public integration tests. The byte matrix covers all 256 values for each of seven destinations, with explicit expected mapping independent of the decoder. Tests also cover every initial flag combination, register isolation, read traces, budget resumption, PC wrapping, missing-data retry, and excluded opcodes.

All 79 tests pass (63 unit, 16 integration); formatting passes. Strict Clippy reports existing unread CpuFault fields and unused pair setters; unused destination variants are resolved by this implementation. No implementation logic changed by the assistant. Issue #6 is complete locally; new work remains uncommitted/unpublished.
