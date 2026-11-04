## Goal
Complete the non-register ADD/SUB forms. Milestone M2; follows #12, committed locally as 6a53606.

| Opcode | Instruction | Bytes | M-cycles |
| --- | --- | --- | --- |
| 0x86 | ADD A,(HL) | 1 | 2 |
| 0x96 | SUB A,(HL) | 1 | 2 |
| 0xC6 | ADD A,d8 | 2 | 2 |
| 0xD6 | SUB A,d8 | 2 | 2 |

Each takes eight base ticks and uses the existing binary arithmetic/flag rules. Memory operands are read only. Immediate bytes are data, even when they resemble opcodes.

## Implemented architecture contract
- Keep decoding pure; add the four operations explicitly or use equivalent typed variants. Source kind and operation must remain unambiguous.
- Introduce an accumulator arithmetic selector with Add/Subtract only. Keep it separate from the unary Operation used by the memory read-modify-write handler, so a memory-write continuation cannot accept binary accumulator operations.
- Add two clear pending read states: an immediate accumulator operand (operation, captured left byte), and a memory accumulator operand (captured address, operation, captured left byte). Names are implementation choices; ReadImmediateAccumulator/ReadMemoryAccumulator describe the roles.
- Opcode completion captures original A, advances PC once with wrapping, and queues the selected read. Memory form also captures HL. Report incomplete.
- Immediate state requests Read at PC. On success, advance PC once more, apply add/sub to captured A and supplied operand, store result in A, update Z/N/H/C, return to FetchOpcode, report complete.
- Memory state requests Read at the captured address. On success, apply arithmetic and finish in the same way, without advancing PC.
- Both read paths reuse the verified add/sub helpers. They never queue WriteMemory or modify the source memory. Preserve HL and all other registers.
- Missing data reports the requested address (PC for immediate, captured address for memory), preserves registers/flags/PC/state/left operand, and permits retry.
- request_cycle remains immutable and repeatable; a pause after the opcode retains the operation, original A, and address.
- A and flags remain unchanged after the opcode and update only when the operand read completes. Capturing A/HL here is the initial project model, not a claim about hardware sampling edges.
- Preserve snapshots, fault handling, System scheduling, traces, and instruction-budget behavior. No new public API or dependencies.
- Exclude ADC/SBC, CP, logic, other addressing forms, and 16-bit arithmetic.

## Acceptance criteria — assistant-owned tests
- [x] Existing 131 tests remain green after decoder supported-byte/exclusion expectations are updated.
- [x] Explicit four-opcode mapping; unrelated bytes and carry-consuming forms remain unsupported.
- [x] Independent references check all 65,536 left/right byte pairs for each operation and source kind (262,144 cases); boundary operands check all initial flag combinations.
- [x] Full snapshots verify only A/flags/PC change and that opcode completion leaves A/flags unchanged.
- [x] Memory forms advance PC by one byte total; immediate forms by two, including wrap at either opcode or operand boundary.
- [x] Public traces show opcode plus one correct read, false/true completion, and exactly eight ticks; memory arithmetic performs no write.
- [x] Missing-read retry and repeated request preserve captured operands/address and report correct error addresses.
- [x] Zero/exact/excess budgets and resume after one cycle return correct per-call counts and never fetch past completion.
- [x] ROM, WRAM, echo, and current unimplemented-region reads follow Bus mapping without changing source storage.
- [x] Mixed programs interleave all ADD/SUB source forms with loads, stores, and INC/DEC, preserving flags through transfers.
- [x] Formatting passes; report existing strict Clippy debt separately.

## First implementation slice
Start with the accumulator operation selector and the pending memory-operand state. Wire ADD A,(HL) into the existing add helper, storing its result in A and completing after the read. Then add SUB and the immediate state, whose successful read additionally advances PC. The assistant owns tests and architecture review.

## References
[RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7), ADD/SUB A,[HL] and A,n8, and [gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json), consulted 2026-11-03.

## Completion review — 2026-11-03

The user separated UnaryOperation and AccumulatorOperation, added immediate/memory accumulator states with captured A/address, and corrected completion timing, immediate addressing/PC advancement, and return to opcode fetch. MissingReadValue now consistently names its address field `addr`; assistant adapted the remaining test guards. No correctness findings remain.

Validation: 138 tests pass (101 unit, 37 integration); formatting passes. Independent operand references cover 262,144 input/source-kind/operation cases, plus all initial flags on boundary pairs. Other checks cover captured operands/address independently of live registers, opcode/operand wrapping, missing-read retries, traces and budget resumes, mixed programs, and complete memory preservation across ROM/WRAM/echo/unimplemented-region reads. Clippy reports only existing unread CpuFault fields and unused set_af. Changes remain local and uncommitted.
