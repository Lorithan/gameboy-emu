## Goal
Implement ADC and SBC for register, (HL), and immediate operands. Milestone M2; follows #13, committed locally as bc13caf.

ADC computes A + operand + incoming C. SBC computes A - operand - incoming C. The incoming flag is a numeric 0 or 1. Result and all Z/N/H/C flags are updated; other registers and memory are preserved.

| Source | ADC opcode | SBC opcode | Bytes | M-cycles |
| --- | --- | --- | --- | --- |
| B | 0x88 | 0x98 | 1 | 1 |
| C | 0x89 | 0x99 | 1 | 1 |
| D | 0x8A | 0x9A | 1 | 1 |
| E | 0x8B | 0x9B | 1 | 1 |
| H | 0x8C | 0x9C | 1 | 1 |
| L | 0x8D | 0x9D | 1 | 1 |
| (HL) | 0x8E | 0x9E | 1 | 2 |
| A | 0x8F | 0x9F | 1 | 1 |
| d8 | 0xCE | 0xDE | 2 | 2 |

## Arithmetic and flags
Use original A, the original operand, and captured incoming C.
- Result wraps modulo 256.
- Z reflects the final result.
- N is zero for ADC and one for SBC.
- ADC H reflects overflow of the low-nibble sum including incoming C; C reflects overflow of the complete three-term sum.
- SBC H reflects low-nibble borrow including incoming C; C reflects complete unsigned borrow, equivalently original A < operand + incoming C evaluated without truncation.
- Prior Z/N/H do not affect the calculation. Outgoing C replaces incoming C.
- Do not first wrap operand + carry into a byte: FF + 1 needs an intermediate value of 100. If using multiple overflowing steps, combine the carry/borrow evidence from both steps before setting flags.
- Self-source operations read original A twice.

Examples with incoming C=1:
| Operation | Result | Final F |
| --- | --- | --- |
| ADC 0F,00 | 10 | 20 |
| ADC FF,00 | 00 | B0 |
| ADC FF,FF | FF | 30 |
| SBC 00,00 | FF | 70 |
| SBC 10,0F | 00 | E0 |
| SBC FF,FF | FF | 70 |

## Implemented architecture contract
- Add decoded ADC/SBC source operations following current register/memory/immediate patterns. Keep decoding pure.
- Reuse the existing register completion and immediate/memory accumulator read paths. No additional cycle types or new pending-state variants are needed.
- Make incoming carry explicit arithmetic input. Shared add/sub helpers may accept a carry input or delegate to carry-aware helpers; ordinary ADD/SUB must supply false and continue ignoring live C.
- For register forms, capture A, source, and incoming C before assigning any result/flags; complete in the opcode cycle, advance PC once, return to FetchOpcode.
- For memory/immediate forms, capture incoming C together with original A during opcode completion. Retain it as pending execution data; the implementation uses AddC(bool)/SubC(bool) selector payloads. Existing Add/Sub pending operations plus explicit carry input can represent the arithmetic; distinct ADC/SBC selector variants are also acceptable if completion remains explicit.
- Ordinary ADD/SUB pending states must use no carry. Do not accidentally start consuming C for existing instructions.
- At operand-read completion, use captured A and carry, update A/flags, and return to FetchOpcode. Memory never writes its operand; only immediate operand completion advances PC again.
- Preserve opcode/data flag-phase visibility, missing-read retry/error addresses, captured HL, PC wrapping, request immutability, traces, and budgets from #13.
- Unary INC/DEC continue preserving C. Keep their selector and memory-write continuation separate.
- No new public API, dependencies, CP/logic, 16-bit arithmetic, or interrupt/HALT behavior.

## Acceptance criteria — assistant-owned tests
- [x] Existing 138 tests stay green after decoder supported-byte/exclusion updates.
- [x] Explicit independent mapping for all 18 opcodes; unrelated operations stay unsupported.
- [x] All 65,536 input pairs with both carry values for each non-A register source and memory/immediate kind, across ADC/SBC, match a widened reference (2,097,152 cases). Self-source cases cover all A values and both carries.
- [x] Boundary cases test all 16 initial flag combinations; only incoming C influences the result.
- [x] Three-term overflow/borrow, low-nibble boundary cases, and self-source SBC with carry (always FF) are explicit regressions.
- [x] Full snapshots prove source/unrelated register preservation; memory operands remain unchanged.
- [x] Captured A/address/carry survive pause, repeated request, missing-read retry, and tests independent of live register/flag values.
- [x] ADD/SUB still ignore C; INC/DEC still preserve it.
- [x] Public traces and zero/exact/excess/resume budgets preserve one-cycle register and two-cycle operand timing, completion, PC advancement, and no extra reads/writes.
- [x] PC wrapping works at opcode/immediate boundaries; fault behavior remains unchanged.
- [x] Mixed carry/borrow chains across instructions include intervening flag-preserving transfers.
- [x] Formatting passes; report existing strict Clippy debt separately.

## First implementation slice
Make the binary arithmetic helper's carry input explicit, with existing ADD/SUB passing false; retain all current tests. Then implement register ADC/SBC. Finally retain incoming C in pending accumulator states and add memory/immediate decoding and completion. The assistant owns tests and architectural review.

## References
[RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7), ADC/SBC sections, and [gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json), consulted 2026-11-03.

## Completion review — 2026-11-03

The user implemented all 18 opcodes, explicit carry input to arithmetic helpers, and captured carry as the payload of AccumulatorOperation::AddC(bool)/SubC(bool). This representation satisfies the pending-input invariant; a separate state field was an earlier proposal, not a requirement. Ordinary ADD/SUB pass false and unary operations retain their original carry behavior.

Validation: 146 tests pass (107 unit, 39 integration); formatting passes. Independent references check 2,097,152 non-A operand/source/operation/carry cases, exhaustive self-source inputs/flags, and boundary flag combinations. Other coverage checks captured carry/A/address across missing reads and live-register perturbations, wrapping PC, public traces/budgets, and carry/borrow chains through flag-preserving transfers. No correctness findings remain. Clippy reports only existing unread CpuFault fields and unused set_af. Changes remain local and uncommitted.
