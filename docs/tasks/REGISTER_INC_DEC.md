## Goal
Implement INC and DEC for A, B, C, D, E, H, and L. Milestone M2; depends on #9, published in d826c0d. This is the first arithmetic slice, with explicit flag behavior.

Each instruction is one byte and completes in one M-cycle/four base ticks under the existing project model.

| Register | INC opcode | DEC opcode |
| --- | --- | --- |
| B | 0x04 | 0x05 |
| C | 0x0C | 0x0D |
| D | 0x14 | 0x15 |
| E | 0x1C | 0x1D |
| H | 0x24 | 0x25 |
| L | 0x2C | 0x2D |
| A | 0x3C | 0x3D |

## Behavior
- INC adds one modulo 256; DEC subtracts one modulo 256.
- Z reflects whether the final byte is zero.
- INC clears N; DEC sets N.
- INC sets H on carry out of the low nibble (old low nibble F).
- DEC sets H on borrow into the low nibble (old low nibble 0).
- Clear Z/H whenever their conditions are false; do not leave stale flags.
- C is preserved, even when the whole byte wraps.
- Preserve all other registers and the low-zero flag-nibble invariant. PC advances once with wrapping.

Boundary examples (C retains its previous value):
| Operation | Result | Z | N | H |
| --- | --- | --- | --- | --- |
| INC 0x0F | 0x10 | 0 | 0 | 1 |
| INC 0xFF | 0x00 | 1 | 0 | 1 |
| DEC 0x10 | 0x0F | 0 | 1 | 1 |
| DEC 0x01 | 0x00 | 1 | 1 | 0 |
| DEC 0x00 | 0xFF | 0 | 1 | 1 |

## Implemented architecture contract
- Add decoded increment-register and decrement-register operations carrying RegisterValue. Names can follow existing conventions.
- Keep decoding pure. CPU performs the register operation and Z/N/H updates in the opcode completion, returns to FetchOpcode, and reports complete.
- Reuse the byte-register read/write helpers. Keep those helpers free of PC, execution-state, and flag side effects; the arithmetic handler owns flag updates.
- Use the existing named flag accessors, preserving C and the private flag representation.
- No new ExecutionState variants are needed for this register-only task.
- Validate missing opcode data before mutating registers, flags, PC, or state.
- Preserve System scheduling, public snapshots, traces, bounded instruction stepping, and sticky unsupported behavior.
- Exclude INC/DEC (HL) (0x34/0x35), which will need a separately specified read-modify-write continuation. Also exclude 16-bit INC/DEC, other arithmetic, jumps, and interrupts.
- Do not introduce a general ALU framework or new dependencies for these two operations.

## Acceptance criteria — assistant-owned tests
- [x] Existing 111 tests stay green after updating supported-opcode expectations.
- [x] Explicit independent mapping checks all 14 opcodes; no unrelated bytes become supported, including 0x34/0x35 and HALT.
- [x] Every byte input, every valid initial flag combination, every destination, and both operations match an independent arithmetic/flag reference.
- [x] Boundary examples above pass, including carry preservation through FF/00 wrap and clearing stale Z/N/H.
- [x] Full snapshots prove unrelated registers and SP are preserved; H/L updates affect only the selected byte of HL.
- [x] Missing opcode retry and PC=FFFF wrapping preserve the existing protocol.
- [x] Public traces show exactly one opcode read, four ticks, and completion; no data access or additional fetch.
- [x] Zero/exact/excess instruction budgets and consecutive INC/DEC operations return correct per-call counts.
- [x] Mixed synthetic programs combine arithmetic with existing loads and stores while preserving flags across transfers.
- [x] Formatting passes; report existing strict Clippy debt separately.

## First implementation slice
Start with the two decoded operations and INC on one register using the shared selector-based helpers. Generalize to all seven registers through the selector, then add DEC. The assistant writes/runs the test matrix and reviews flag calculations.

## References
[RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7), sections INC r8 and DEC r8, and [Pan Docs instruction groups](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md), consulted 2026-11-03.

## Completion review — 2026-11-03

The user implemented both operations and corrected the wrapping assignment, original-value H calculation, and DEC N flag during review. No correctness findings remain in the current implementation. The assistant changed tests/documentation only.

Validation: 117 tests pass (86 unit, 31 integration); formatting passes. Exhaustive tests independently check all 256 inputs, all 16 initial flag combinations, seven destinations, and both operations (57,344 cases). Other coverage includes explicit mappings, unsupported-memory/HALT rejection, full snapshots, opcode-read retries and PC wrapping, public traces, budgets, boundary examples, and flags preserved by interleaved loads/stores.

Clippy reports only the existing unread CpuFault fields and unused set_af. Changes remain local and uncommitted. Commit this checkpoint before starting another arithmetic slice.
