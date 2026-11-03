# Immediate 16-bit register loads

## Goal
Implement LD BC,d16 (01), LD DE,d16 (11), LD HL,d16 (21), and LD SP,d16 (31). Milestone: **M2 — CPU execution**. Build on the verified byte-load family and pure decoder.

Each instruction has an opcode plus a little-endian two-byte operand, takes three M-cycles/twelve base ticks, and preserves flags. Example: 21 34 12 loads HL=1234.

## Architecture contract
- Add an internal 16-bit destination selector with BC, DE, HL, SP. Do not include AF or reuse an untyped byte-register selector.
- Add one decoded operation for immediate 16-bit loads carrying that selector. Decoder remains pure.
- Execution state retains the destination while waiting for the low byte, then retains destination plus the low byte while waiting for the high byte.
- Opcode cycle: increment PC once, record destination, report incomplete.
- Low-byte cycle: increment PC once, retain the byte, report incomplete.
- High-byte cycle: increment PC once, assemble the value, write only the selected pair/SP, return to opcode fetch, report complete.
- Use existing pair helpers for BC/DE/HL and controlled parent access for SP. Do not create a partial destination update after the low byte: our initial model commits at completion.
- All PC increments wrap. Missing read data leaves registers, destination, buffered low byte, and execution state unchanged.
- Preserve System's current tick ordering, traces, errors, and resumable instruction budgets. No memory writes, stack operations, HRAM, arithmetic, or additional opcodes in this task.

## Opcode table
| Byte | Destination |
| --- | --- |
| 0x01 | BC |
| 0x11 | DE |
| 0x21 | HL |
| 0x31 | SP |

## Acceptance tests (assistant-owned)
- [x] Existing 81-test baseline stays green; decoder rejection test is updated for the four newly supported bytes.
- [x] Explicit mapping and independent expected byte order for all destinations.
- [x] Values 0000, 00FF, FF00, FFFF, 1234, and 8001 load correctly; unit tests sweep all u16 values for each destination.
- [x] After each cycle PC advances once; destination is unchanged until the high byte completes; flags and all unrelated registers are unchanged.
- [x] Public traces contain three sequential read addresses/bytes, spans 0–4/4–8/8–12, and false/false/true completion.
- [x] Budgets zero, one, two, and three behave correctly; resume after either operand boundary, retaining destination/low byte and reporting per-call counts.
- [x] No fetch beyond completion, even with a larger budget.
- [x] Missing-data retries and PC wrapping at opcode/low/high stages work for every destination.
- [x] All valid initial flag combinations are preserved; supported byte loads/NOP still work when interleaved with pair loads.
- [x] Unsupported 0x36 and unrelated bytes retain existing errors/sticky faults.
- [x] Formatting passes; report remaining strict Clippy failures without unrelated visibility expansion or warning suppression.

## References
[Pan Docs instruction groups](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) and [gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json), consulted 2026-11-02.

The user owns implementation, the assistant owns tests and architecture review. First establish the pair destination type and decoded variant; then implement the low/high operand states. Keep System unchanged. Commit the verified byte-load/decoder checkpoint before starting this separate task.

Validation: 90 tests pass (69 unit, 21 integration), including exhaustive word operands, all fetch-stage wrapping/retries, and mixed instruction boundaries. Formatting passes. Strict Clippy still reports unread CpuFault fields and unused set_af. Implementation is local and uncommitted.
