## Goal
Implement the first CPU data-memory operations through HL, after #7.

| Opcode | Instruction | Bytes | M-cycles |
| --- | --- | --- | --- |
| 0x77 | LD (HL),A | 1 | 2 |
| 0x7E | LD A,(HL) | 1 | 2 |
| 0x36 | LD (HL),d8 | 2 | 3 |

All preserve flags and HL. Timing follows the existing four-tick M-cycle model, not physical edge accuracy.

## Implemented architecture contract
- Keep byte/word register selectors separate. Memory at HL is an addressing operation, not another byte-register enum member.
- Pure decoder identifies these operations; CPU execution states retain pending address/data.
- After opcode fetch, PC advances once. Capture HL as the pending data address in this initial model.
- LD (HL),A: opcode read, then Write { addr: captured_HL, value: captured_A }. Capture A with the opcode; complete on the write cycle.
- LD A,(HL): opcode read, then Read { addr: captured_HL }. Store the supplied byte in A only when that read completes.
- LD (HL),d8: opcode read, immediate read at PC (advance PC once), then write the buffered byte to captured_HL. Complete only on the write.
- Data-memory cycles do not advance PC. Preserve unrelated registers.
- request_cycle remains immutable and repeatable. It never performs the write. System owns Bus access and timing; write completion uses None and returns to opcode fetch.
- A missing byte for a read preserves all CPU state and allows retry. A write completion requires no read byte.
- Reuse System's existing write path and trace shape: write traces contain the address/value in action and read_value=None. Avoid changing System unless review reveals a concrete defect.
- Bus mapping remains authoritative: ROM writes ignored; WRAM/echo alias; unimplemented addresses still read FF/ignore writes.
- Keep this task to these three opcodes. No auto-increment/decrement HL, other register-memory loads, stack, arithmetic, interrupts, or device work.

## Acceptance criteria (assistant-owned tests)
- Existing 90 tests stay green after updating decoder supported-byte tables and the old assertion that 0x36 is unsupported.
- Explicit opcode mapping; all other unsupported bytes still fault and remain sticky.
- Every byte value round-trips through a synthetic program: set HL to WRAM, set A, store, overwrite A, reload, verify original byte and unchanged flags/HL.
- Immediate store also round-trips; operand byte is data even when it resembles an opcode.
- Per-cycle traces prove exact fetch/read/write addresses, values, four-tick spans, and completion only on the last cycle.
- Request inspection does not mutate CPU, memory, or clock; budget exhaustion before a write causes no write yet.
- Zero/exact/excess budgets and resume at every boundary retain address/value and stop without fetching the next opcode.
- Reads preserve pending state after missing data; PC wraps on instruction bytes and does not advance during data access.
- All valid flag combinations and unrelated registers are preserved.
- WRAM boundaries/echo aliases work; ROM writes leave bytes unchanged; unimplemented regions follow current bus behavior.
- Synthetic fixtures only, no ROM downloads/dependencies.
- Formatting passes; report strict Clippy debt separately.

## References
[gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json), unprefixed 36/77/7E, consulted 2026-11-02.

User owns implementation; assistant owns tests and architecture review. Commit the immediate-load/decoder checkpoint before starting this issue. First define the decoded operations and pending data-access states; retain the existing CPU/System boundary.

## Completion review — 2026-11-03

All listed acceptance cases have been exercised. The suite passes 102 tests (77 unit, 25 integration), and formatting passes. Added coverage includes all 256 byte values through both store forms and reloads with exact traces, pause/resume at every cycle boundary with no early write, WRAM/echo and ROM/unimplemented-region boundaries, all flag combinations, instruction-fetch wrapping and read retries, and captured address/data invariants. The execution-state refactor is reviewed in [CPU_EXECUTION_STATES.md](CPU_EXECUTION_STATES.md).

Strict Clippy still reports unread CpuFault fields and unused set_af. This is local validation of the initial M-cycle model and existing bus approximations. Changes remain uncommitted; commit before starting another instruction family.
