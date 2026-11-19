# M2 completion decision

[#51](https://github.com/Lorithan/gameboy-emu/issues/51), reviewed 2026-11-18 at f0283fa: **M2 is complete for the functional DMG CPU scope**. [Milestone M2](https://github.com/Lorithan/gameboy-emu/milestone/3) was closed. That checkpoint passed 430 tests including five real fixtures; subsequent cleanup retained the evidence, with 442 tests last verified at dcedd79.

| Original gate | Evidence |
| --- | --- |
| Registers/decoding/instructions | [500 legal encodings](OPCODE_COVERAGE.md), independent decode and register/flag tests |
| Arithmetic/edge cases | Exhaustive byte/word/flag inputs, independent widened/nibble arithmetic, ADC/SBC, all A/flag DAA inputs and valid decimal combinations, CB result/flag oracles |
| Loads/stack/control flow | Family unit/public tests for endian order, captured values, PC/SP wrapping, branches and cycle actions |
| Deterministic stepping/diagnostics | Typed rejection/retry, sticky faults, snapshots/traces, bounded/zero budgets, bounded ROM runner |
| Interrupt/control | DI/EI/RETI, late selection/cancellation, HALT/conditional STOP and selected IRQ/DIV ROMs |
| Selected ROM results | Actual pinned reg_f, if_ie_registers, ie_push, ei_sequence and div_timing passes, with provenance/signatures/counts/reproduction in [TESTING.md](TESTING.md) |

This does not certify all hardware behavior, every game's compatibility or every external suite. Coarse M-cycle timing, skipped boot, diagnostic illegal faults and physical STOP limits remain explicit in [CPU](CPU.md) and [devices](DEVICES.md). PPU/DMA/mapper/audio are later milestones. Reopen a gate if demonstrated evidence invalidates its scope.

## Separate validation work

- [#52 pinned Mooneye DAA](https://github.com/Lorithan/gameboy-emu/issues/52): nonblocking, unrun. Its 4,096 A/flag table cases need artifact/header/reporter verification and bounded explicit execution before claiming a pass.
- Inspected [SingleStepTests SM83](https://github.com/SingleStepTests/sm83/tree/f9c30210245dd691661db39f5ace022c465ecc2f) vectors require a test-only flat-memory adapter, not production Bus; their emulator-derived generator/IME cautions are not independent hardware evidence.
- The inspected [Blargg individual builder](https://github.com/retrio/gb-test-roms/blob/c240dd7d700e5c0b00a7bbba52b53e4ee67b5f15/cpu_instrs/source/common/build_rom.s) needs a separate mapper/prerequisite/reporting audit.
- [#64 physical STOP assessment](https://github.com/Lorithan/gameboy-emu/issues/64) is separate from completed deterministic integration #44.

Next: [M3 timer contract #53](tasks/TIMER.md), before application implementation.
