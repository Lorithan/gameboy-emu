## Goal
Implement CP (compare A with an operand) across register, (HL), and immediate sources. Milestone M2; follows #14, committed locally as 17802f7.

CP updates flags as if subtracting the operand from A, but discards the result and preserves A. This prepares the flags for later conditional branches.

| Source | Opcode | Bytes | M-cycles |
| --- | --- | --- | --- |
| B | 0xB8 | 1 | 1 |
| C | 0xB9 | 1 | 1 |
| D | 0xBA | 1 | 1 |
| E | 0xBB | 1 | 1 |
| H | 0xBC | 1 | 1 |
| L | 0xBD | 1 | 1 |
| (HL) | 0xBE | 1 | 2 |
| A | 0xBF | 1 | 1 |
| d8 | 0xFE | 2 | 2 |

## Behavior
- Z is set when original A equals the operand.
- N is always set.
- H is set when subtracting the operand would borrow from bit 4.
- C is set when original A is less than the operand (unsigned).
- All four flags are overwritten. Incoming C is ignored.
- A, the source, other registers, SP, and memory remain unchanged. Preserve the flag low-nibble invariant.
- CP A,A always produces F=C0, with A unchanged.
- Examples: A=10, operand=01 -> F=60; A=00, operand=01 -> F=70; A=42, operand=42 -> F=C0.

## Implemented architecture contract
- Add decoded register, memory, and immediate compare operations using the current source patterns; decoding stays pure.
- Add a Compare variant to AccumulatorOperation for pending operand reads. Reuse the existing immediate/memory accumulator states and captured-left/address behavior. No new ExecutionState variants are needed.
- Reuse subtraction flag calculation with carry input false, then discard the returned arithmetic byte. Do not assign the result to A and do not temporarily change A then restore it.
- Register comparison completes in the opcode cycle, advances PC once with wrapping, and explicitly returns to FetchOpcode.
- Memory/immediate opcode completion captures original A and, for memory, HL; advances PC once and reports incomplete. Flags remain unchanged at this boundary.
- Operand completion computes comparison flags using captured A and the supplied operand, returns to FetchOpcode, and reports complete. Immediate completion advances PC once more; memory completion does not.
- Keep the compare path's flags-only completion explicit, rather than inheriting the arithmetic path's write-to-A behavior.
- Missing opcode/data reads report the requested address and preserve registers, flags, PC, pending operation, and captured inputs. Retry uses the same captures.
- Keep request inspection immutable; memory forms are read-only. Preserve existing System scheduling, traces, budgets, faults, and public API.
- Existing ADD/SUB/ADC/SBC and INC/DEC behavior must stay intact.
- No branches, logic operations, 16-bit comparisons, interrupts/HALT, or dependencies in this task.

## Acceptance criteria — assistant-owned tests
- [x] Existing 146 tests stay green after supported-byte/exclusion expectations are updated.
- [x] Independent mapping covers all nine opcodes; unrelated instructions remain unsupported.
- [x] All 65,536 operand pairs across six non-A register sources and memory/immediate kinds match independent flags-only references (524,288 cases).
- [x] All 256 self-source A values and every valid initial flag combination produce F=C0 without changing A.
- [x] Boundary inputs with all initial flags prove incoming carry is ignored and stale flags are cleared.
- [x] Full snapshots verify A/source/unrelated-register preservation; memory checks prove no writes.
- [x] Public traces show one register opcode cycle or opcode plus one operand read; exact timing/completion and no extra fetch.
- [x] Zero/exact/excess budgets and pause/resume preserve A and pre-comparison flags until completion, with correct per-call counts.
- [x] Missing-read retries, captured-left/address checks, PC wrap at opcode/operand boundaries, and sticky faults retain current behavior.
- [x] Bus-boundary memory reads follow ROM/WRAM/echo/unimplemented-region behavior without storage changes.
- [x] Mixed arithmetic/compare/load/store programs preserve comparison flags through transfers while retaining A's value.
- [x] Formatting passes; report existing strict Clippy debt separately.

## First implementation slice
Start with register CP: use the existing subtraction helper with carry false and discard its result. Then extend the pending accumulator operation selector and memory/immediate decoder paths, preserving A at operand completion. The assistant owns exhaustive tests and architectural review.

## References
[RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7), CP A,r8 / A,[HL] / A,n8, and [gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json), consulted 2026-11-03.

## Completion review — 2026-11-03

All nine CP opcodes preserve A/source storage and reuse subtraction flags with carry false. The user centralized opcode PC advancement after successful data validation, retained original opcode addresses for faults, and corrected the memory/immediate comparison states. Existing fault-address and PC-wrapping regressions pass. No correctness findings remain.

Validation: 152 tests pass (111 unit, 41 integration); formatting passes. Independent flags-only references check 524,288 non-A operand/source pairs; self-source and boundary cases cover all initial flags. Additional tests cover captured comparison inputs, missing reads, PC wrap, public traces/budgets, mixed arithmetic/transfers, and complete source-storage preservation across Bus boundaries.

The obsolete unused addr local has been removed. Formatting and Clippy were checked after that cleanup; Clippy reports only the existing unread CpuFault fields and unused set_af. The full 152-test run preceded this unused-local removal. Implementation/tests/docs remain local and uncommitted.
