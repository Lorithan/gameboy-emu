## Goal
Complete the unprefixed 0x40–0x7F byte-transfer block, excluding 0x76 (HALT). Milestone M2; depends on #8, published in b14d33f.

This covers 49 register-to-register copies and 14 transfers between a register and memory at HL. Two memory forms (0x77 and 0x7E) already work, so 61 additional opcodes become supported. Registers are A, B, C, D, E, H, and L.

| Form | Example | M-cycles | Base ticks |
| --- | --- | --- | --- |
| Register to register | LD B,C | 1 | 4 |
| Memory to register | LD D,(HL) | 2 | 8 |
| Register to memory | LD (HL),E | 2 | 8 |

Every instruction is one byte and preserves flags. Self-copies are valid and still consume their opcode cycle. 0x76 is HALT, not a memory-to-memory load; keep it explicitly unsupported until the halt/interrupt task.

## Implemented architecture contract
- Add one decoded register-copy operation carrying two RegisterValue selectors (source and destination). Reuse LoadFromHL and StoreToHL for the memory forms.
- Decoder remains pure. Decode the instruction family by its operand fields if useful: destination bits 5–3, source bits 2–0. Selector order is B,C,D,E,H,L,(HL),A. Handle the memory selector separately; do not add it to RegisterValue or cast encoding numbers to the existing enum.
- Register copies complete during the opcode cycle in our current M-cycle model: read the source, assign the destination, advance PC once, report completion. No extra pending state or bus cycle.
- Memory forms reuse the current ReadMemory/WriteMemory states and captured address/data contract.
- For LD H,(HL) and LD L,(HL), read the original HL address before modifying its component register. For LD (HL),H/L, capture the original address and source byte. These are targeted review cases.
- Internal register helpers read/write the selected byte without PC/state/timing side effects. Reuse store_register; an internal selector-based read helper can replace the existing source-register match.
- Preserve PC wrapping, missing-read retry behavior, sticky unsupported faults, snapshots, traces, and per-call instruction budgets.
- System, Bus, word-register selectors, and the existing immediate forms require no design changes.
- Do not add arithmetic, HALT behavior, interrupts, other addressing modes, or dependencies.

## Acceptance criteria — assistant-owned tests
- [x] Existing 102 tests pass after updating the decoder's supported-byte expectations.
- [x] Independent explicit opcode matrix checks all 63 transfers; 0x76 stays unsupported and no unrelated bytes become accepted.
- [x] All 49 register pairs copy every byte value correctly, including self-copies, preserving flags and unrelated registers.
- [x] All seven read and seven write forms select the correct register; values 00, FF, and asymmetric bit patterns plus exhaustive byte sweeps cover transfers.
- [x] H/L alias cases use the original HL address and produce the expected final register values.
- [x] Public traces show one opcode read for register copies, or opcode plus the correct data access for memory transfers; exact timing/completion and no extra fetch.
- [x] Zero, exact, and excess instruction budgets; paused memory access resumes with captured address/value and no early write.
- [x] PC wrapping and missing opcode/data retries preserve state; all valid flag combinations are unchanged.
- [x] Mixed synthetic programs interleave the new transfers with existing immediate loads and NOP; CPU memory accesses continue to respect Bus mapping.
- [x] Formatting passes; report strict Clippy debt separately.

## First implementation slice
Start with the decoded register-copy variant and its opcode-cycle completion. Then extend decoder coverage to the remaining HL forms using existing execution states. The assistant owns tests and review throughout.

## References
[Pan Docs instruction grouping](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) and [gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json), consulted 2026-11-03. Timing remains the existing project M-cycle convention.

## Completion review — 2026-11-03

The user added the register-copy operation and explicit decoder mappings. All 63 transfers pass an independent opcode matrix, exhaustive source-byte tests, all valid flag combinations, PC wrapping and missing-read retries. Public integration tests verify exact cycle traces, zero/exact/excess budgets, paused memory access, H/L alias behavior, mixed programs, and sticky rejection of HALT. Existing bus-mapping and no-early-write regressions remain green through the reused memory states.

Validation: 111 tests pass (82 unit, 29 integration); formatting passes. Clippy reports only the existing unread CpuFault fields and unused set_af. No correctness findings remain. The user extracted a shared source-register helper and renamed the store operand to `source`; the complete suite passes after that cleanup.
