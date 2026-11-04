# Absolute jumps

Issue: [#18](https://github.com/Lorithan/gameboy-emu/issues/18), M2 CPU execution. Baseline: relative jumps in f34571f, with 162 passing tests (117 unit, 45 integration).

## Scope and required behavior

| Opcode | Instruction | M-cycles |
| --- | --- | --- |
| C3 | JP a16 | 4 |
| C2 | JP NZ,a16 | 4 taken / 3 untaken |
| CA | JP Z,a16 | 4 taken / 3 untaken |
| D2 | JP NC,a16 | 4 taken / 3 untaken |
| DA | JP C,a16 | 4 taken / 3 untaken |
| E9 | JP HL | 1 |

Immediate targets are unsigned 16-bit little-endian addresses. Both bytes are fetched even if the condition is false. A taken immediate jump has opcode, low-byte, high-byte, and Idle cycles; an untaken jump completes on the high-byte read. One M-cycle remains four ticks. JP HL copies the value of HL into PC during opcode completion, with no dereference of HL and no additional cycle. All flags, other registers/SP, and memory are preserved.

Example: C3 34 12 at 0x0100 targets 0x1234, regardless of the instruction's address. Sequential PC reaches 0x0103 before the internal cycle commits the target. A false conditional form finishes with PC=0x0103. JP HL uses HL as the complete target, including values outside mapped storage.

Sources: [RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7) for behavior/timing and [Pan Docs encoding](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) for byte mappings and little-endian operands.

## Architecture proposal

Reuse JumpCondition and existing ApplyJump { target }. Resolve the condition at successful opcode completion and capture the decision, as for JR. A shared side-effect-free condition evaluator would avoid copying that match; it should only inspect flags and return a boolean.

Suggested states: ReadJumpLow { taken } followed by ReadJumpHigh { taken, low }. Each requests a read at PC and increments PC once after successful completion, wrapping at 16 bits. Low completion retains the captured decision and byte. High completion assembles the target; taken queues ApplyJump and returns incomplete, while untaken returns FetchOpcode and completes. ApplyJump already commits the target during Idle.

These two small jump-specific states keep the current load destination selector unchanged. Do not add PC to Register16 merely to reuse immediate-load handlers; loads and jumps have different completion timing. A general immediate-word continuation type can be discussed if concrete duplication later warrants it. State/type names are implementation choices within this contract.

Represent JP HL separately in decoding, since it has no immediate operand or condition. Its final PC must equal HL despite the common opcode-fetch PC increment. Do not route it through ApplyJump or read memory at HL.

Missing opcode/operand data leaves PC, registers, flags, and pending decision/low byte unchanged for retry. Requests remain immutable and repeatable. Budget pauses preserve captured data, and target commitment stays at our existing M-cycle observation boundary; this does not assert physical edge timing. No System or Bus changes are expected.

## Assistant-owned acceptance tests

- Independent mappings for all six opcodes and updated all-byte supported/unsupported classification.
- Every 16-bit immediate target and every HL target, with independent little-endian expectations; all flag combinations and condition truth tables.
- PC wrap during opcode, low-byte, and high-byte reads; no partial PC target assignment after the low byte.
- Missing-read retry at each read, preservation of captured decision/low byte/target, repeated requests, and no changes to flags or unrelated snapshot state.
- Public traces: taken immediate Read/Read/Read/Idle; untaken Read/Read/Read; HL opcode Read only. Check exact ticks, read data, completion markers, and no memory write or read-ahead.
- Zero/partial/exact/excess instruction budgets and resumes at each phase; bounded self-loops and a mixed JR/JP program that skips unsupported sentinels.
- Full existing regression suite and formatting; report Clippy warnings separately. Synthetic in-memory programs only; no ROM fixtures or dependencies.

## Implementation sequence

Begin with unconditional C3 and its low/high operand states, reusing ApplyJump. Then add the four conditions and E9. User writes implementation; assistant writes tests and reviews. CALL, RET, RST, interrupts, stack operations, and other instruction families remain separate tasks.
