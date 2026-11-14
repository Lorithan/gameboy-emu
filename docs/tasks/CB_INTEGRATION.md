# CB integration validation - completed locally 2026-11-14

## Contract

Validate implemented CB instructions through CPU, System and Bus using synthetic ROM bytes. Keep application implementation and dependencies unchanged. Cover exact bus cycles/ticks, instruction completion, resumable budgets, mapped/unmapped memory and echo behavior, branch interactions and PC wrapping. No ROM binary fixtures are introduced.

## Evidence

- tests/cb_instructions.rs: all 256 encodings, five operand values and four initial flag patterns (5,120 cases). Exact prefix/subopcode/data reads and write actions, returned data, four ticks per M-cycle, completion boundaries, independent arithmetic/flag oracle and full CPU snapshots. A following NOP checks continuation.
- All 256 encodings with two carry patterns at every budget boundary and a large budget (2,160 cases). Zero-budget calls preserve snapshots/ticks; resumed results and elapsed time match uninterrupted execution.
- src/system/tests/cb_integration.rs: all 32 memory-target encodings across eight addresses (ROM, unmapped memory, WRAM, WRAM boundary, echo, HRAM and IE) and every budget boundary. Full 65,536-address comparisons before/after resume prove no early changes, BIT memory preservation, ignored writes to unmapped/ROM addresses and modification only of the completed target plus its WRAM echo alias. Remaining-cycle traces assert one modifying write or zero BIT writes.
- A synthetic program combines WRAM loads/stores, SET, BIT, RES, RR and taken/untaken conditional branches. Checked PC checkpoints, observed cleared memory, final AF=C010 and 144 ticks.
- A prefix at FFFE is followed by the current Bus's FF read at FFFF (IE unimplemented). SET 7,A completes with PC=0000; execution continues into ROM. This records current Bus behavior, not a final IE hardware contract.
- Shared independent test oracle in tests/support/cb.rs uses arithmetic rather than the implementation's bit operations.

All 326 tests pass (228 unit, 98 integration). Formatting, strict Clippy and git diff whitespace checks pass. Five new tests were added, with minimal test-module wiring. No emulator application code was changed by the assistant. Changes remain uncommitted; publication not checked.

## Limits and next step

This completes the focused System/Bus validation checkpoint. It is not hardware certification: no external ROM suite was run, the timing model remains one coarse M-cycle per action, and transform coverage does not exhaust every input/flag combination on every register target. Interrupt, device and mapper semantics remain separate work.

Next: define the interrupt/IME contract and acceptance criteria before the user implements DI/EI/RETI/HALT; STOP requires its own power-state scope. Refactoring remains separate user-owned work.
