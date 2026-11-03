## Goal
Implement ADD A,r8 and SUB A,r8 for A, B, C, D, E, H, and L. Milestone M2; follows #11, committed locally as cb42ea3. This is the first binary-arithmetic slice.

Each instruction is one byte and completes in one M-cycle/four base ticks.

| Source | ADD opcode | SUB opcode |
| --- | --- | --- |
| B | 0x80 | 0x90 |
| C | 0x81 | 0x91 |
| D | 0x82 | 0x92 |
| E | 0x83 | 0x93 |
| H | 0x84 | 0x94 |
| L | 0x85 | 0x95 |
| A | 0x87 | 0x97 |

## Behavior
Read the original A as the left operand and the selected register as the right operand. Store the modulo-256 result in A; preserve all other registers and SP. Advance PC once with wrapping.

| Flag | ADD | SUB |
| --- | --- | --- |
| Z | Result is zero | Result is zero |
| N | 0 | 1 |
| H | Low-nibble sum exceeds 15 | Left low nibble is less than right low nibble |
| C | Full unsigned sum exceeds 255 | Original A is less than source |

All four flags are assigned on every execution; initial flags do not affect the result. Unlike INC/DEC, C is recomputed. SUB does not subtract the previous C flag; that is SBC, a separate instruction family. Preserve the flag low-nibble invariant.

Aliasing matters: ADD A,A uses the original A twice. SUB A,A always gives A=0 with Z=N=1 and H=C=0.

Examples:
- FF + 01 -> 00, F=B0.
- 0F + 01 -> 10, F=20.
- 10 - 01 -> 0F, F=60.
- 00 - 01 -> FF, F=70.
- 42 - 42 -> 00, F=C0.

## Implemented architecture contract
- Add decoded register-source ADD-to-A and SUB-from-A operations carrying RegisterValue. Two explicit variants fit the current decoder conventions.
- Decoder stays pure. Opcode completion reads both input bytes before assigning A, calculates result/flags, advances PC, returns to FetchOpcode, and reports complete.
- Reuse the register selector helpers and named flag setters. Internal add/subtract helpers can return a byte while updating flags, consistent with current increment/decrement helpers. They do not move PC or choose execution states.
- No new pending state is needed for register sources. The current Operation/ReadOperation path exists for memory read-modify-write; it need not be broadened for this task.
- If sharing an operation-kind selector, keep captured operands separate from the selector. Do not add unused binary pending-state fields now.
- Future ADD A,(HL) updates A after reading memory; it must not inherit the existing ReadOperation handler's memory-write continuation. Define that separately with its task.
- Missing opcode data preserves all state and can be retried; preserve current unsupported faults, traces, System timing, and instruction budgets.
- Exclude memory forms 0x86/0x96, immediate forms 0xC6/0xD6, ADC/SBC, CP, logic, 16-bit arithmetic, interrupts, and dependencies.

## Acceptance criteria — assistant-owned tests
- [x] Existing 124 tests remain green after updating supported-opcode expectations.
- [x] Independent mapping checks all 14 opcodes; memory/immediate/carry forms and unrelated bytes remain unsupported.
- [x] All 65,536 operand pairs for each non-A source and operation match an independent widened arithmetic/flag reference; all 256 original A values cover self-source operations.
- [x] Initial flag combinations are overwritten correctly, including stale C/H/Z/N; boundary cases cover all 16 initial flag combinations for every source.
- [x] Full snapshots prove only A, flags, and PC change; sources and H/L pairs remain intact.
- [x] Boundary examples above, unsigned carry/borrow, half-carry/borrow, and self-source alias cases pass.
- [x] Missing opcode retries, PC wrapping, request immutability, and sticky unsupported behavior remain correct.
- [x] Public traces prove one opcode read/four ticks/completion with no extra access; zero/exact/excess budgets and per-call counts work.
- [x] Mixed synthetic programs combine loads, register/memory INC/DEC, ADD/SUB, and stores while preserving flags across transfers.
- [x] Formatting passes; report strict Clippy debt separately.

## First implementation slice
Start with the decoded ADD-register operation and its opcode-cycle completion. Capture both original inputs before changing A; set all four flags. Then add SUB with its unsigned borrow rules. The assistant owns exhaustive tests and architecture review.

## References
[RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7), ADD A,r8 and SUB A,r8, and [Pan Docs instruction groups](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md), consulted 2026-11-03.

## Completion review — 2026-11-03

Both decoded operations capture original operands, update A and all four flags, advance PC with wrapping, and explicitly return to FetchOpcode. The user corrected PC advancement and the accidental second C setter during review. The explicit state assignment matches other completion handlers; the former code already entered through FetchOpcode, so omitting that redundant assignment was not a separate runtime defect in this path.

Validation: 131 tests pass (96 unit, 35 integration); formatting passes. Tests independently check all 65,536 operand pairs across six non-A sources and two operations (786,432 cases), self-source A with all initial flags, boundary flag overwrites, snapshots, missing-read retries/PC wrap, public traces/budgets, and mixed unary/binary arithmetic and transfers. No correctness findings remain. Clippy reports only existing unread CpuFault fields and unused set_af. Changes remain local and uncommitted.
