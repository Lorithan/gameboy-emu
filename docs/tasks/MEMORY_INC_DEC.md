## Goal
Implement INC (HL) (0x34) and DEC (HL) (0x35). Milestone M2; depends on #10, committed locally as 172556b (not yet pushed).

Each instruction is one byte and takes three M-cycles/twelve base ticks: opcode read, data read, data write. Arithmetic and final flags match the register forms; C is preserved. The addressed byte changes modulo 256; HL and all other registers are preserved, except flags and PC.

## Implemented architecture contract
- Introduce a small operation selector for increment/decrement and one decoded memory-modification operation carrying it (or equivalent explicit decoded variants). Keep it separate from RegisterValue.
- Add one pending read-modify state retaining a resolved address and the operation. Suggested shape: ReadModifyMemory { addr, operation }.
- Keep ReadMemory register-directed: its plain load completion differs from the arithmetic continuation.
- Opcode cycle: capture HL, advance PC once with wrapping, enter the pending modification read, report incomplete.
- Data-read cycle: request Read at the captured address. Validate supplied data, compute the new byte and Z/N/H, preserve C, then queue the existing WriteMemory { addr, value } and report incomplete. Do not change memory here.
- Data-write cycle: reuse the existing write path and completion; System writes the buffered byte, CPU returns to FetchOpcode, report complete.
- PC does not advance during either data cycle; HL never changes. No extra operand fetch, idle cycle, or instruction-wide Bus call.
- Shared arithmetic logic may calculate the result and update Z/N/H for both register and memory forms. Register read/write helpers continue to have no flag, PC, state, or timing side effects.
- Missing read data reports the captured address and preserves all CPU state, including flags, operation, and PC. Retry reads the same address.
- request_cycle is immutable and repeatable. Pausing after the read retains the transformed byte and captured address; do not re-read or recompute on resume.
- Initial project flag timing: flags remain unchanged after opcode fetch, update on successful data-read completion, and remain unchanged through the pending write. This explicitly defines observable snapshots in our M-cycle model; the instruction reference does not establish physical flag sampling edges.
- Preserve System timing/traces/budgets and Bus mapping. A write ignored by Bus (ROM or unimplemented region) still completes the CPU instruction and updates flags from the read byte.
- No new public API, dependencies, 16-bit arithmetic, other ALU operations, interrupts, or HALT.

## Arithmetic rules
Use the original byte for H: INC when its low nibble is F, DEC when its low nibble is 0. Z follows the result; INC clears N and DEC sets N. C remains unchanged, even at FF/00 wrap.

## Acceptance criteria — assistant-owned tests
- [x] Existing 117 tests remain green after supported-opcode expectations are updated.
- [x] Explicit 0x34/0x35 mapping; no unrelated opcodes become supported.
- [x] Independent reference checks both operations for all 256 inputs and all 16 valid initial flag combinations (8,192 cases).
- [x] Public traces prove opcode read, data read, data write at the correct addresses/values, false/false/true completion, twelve ticks, and no extra fetch.
- [x] Snapshots prove opcode-only PC advancement, unchanged HL/unrelated registers, and the specified flag timing after each cycle.
- [x] Memory remains unchanged until the write cycle, including when paused after opcode or data read; repeated request/inspection has no effect.
- [x] Zero/exact/excess budgets and resumes after either boundary retain address, result, flags, and per-call counts without repeating the modification.
- [x] Missing-data retry and opcode PC wrapping preserve state and use captured addresses even when tested independently of live HL.
- [x] WRAM boundaries and echo aliases modify the right storage. ROM writes leave bytes intact; unimplemented addresses follow current FF-read/ignored-write behavior while flags still reflect the arithmetic.
- [x] Mixed programs store a byte, increment/decrement it, reload it, and preserve arithmetic flags across transfers.
- [x] Formatting passes; report strict Clippy debt separately.

## First implementation slice
Define the operation selector and pending modification-read state first. Wire opcode capture and request_cycle next, then the successful read continuation into the existing WriteMemory state. Share arithmetic behavior only as needed, preserving the existing exhaustive register tests.

## References
[RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7), INC [HL]/DEC [HL], and [Pan Docs instruction groups](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md), consulted 2026-11-03. Flag-phase visibility is a project proposal, not a sourced physical-edge claim.

## Completion review — 2026-11-03

The user implemented Operation::{Increment,Decrement}, ReadOperation { addr, operation }, and shared arithmetic helpers for register/memory forms. Derive syntax, the decrement return value, and captured-address error reporting were corrected during review. No correctness findings remain.

Validation: 124 tests pass (91 unit, 33 integration), including 8,192 exhaustive memory arithmetic/flag cases and the previous 57,344 register cases. Public traces cover every memory input byte. Additional tests prove phase-specific flags, no early writes, pause/resume without double arithmetic, captured addresses, missing-read retries, PC wrapping, WRAM/echo/ROM/unimplemented-region behavior, and mixed arithmetic/transfers. Formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af. Changes are local and uncommitted; the preceding register commit 172556b is also still unpushed.

Future proposal: retain operation kind separately from captured operand values in pending state as arithmetic expands. Binary arithmetic can have a different completion destination: ADD A,(HL) writes A, while INC (HL) queues a memory write. A shared operation selector alone does not specify that continuation. Define those interfaces with their tasks rather than extending the current memory-write continuation speculatively.
