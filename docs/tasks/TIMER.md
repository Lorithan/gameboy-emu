# M3 programmable timer design brief

Active design [#53](https://github.com/Lorithan/gameboy-emu/issues/53). The register-only foundation below is specified and ready for user implementation; counting/reload sections remain proposals/questions. User writes application code; assistant owns source audit, design, tests and review. [DIV foundation](../DEVICES.md#divider-phase) and [schedule](../CORE_ARCHITECTURE.md#machine-cycle-schedule) already support real ticks, HALT/STOP and IRQ service.

## First implementation slice: register storage and mapping

[#65](https://github.com/Lorithan/gameboy-emu/issues/65), M3. Add private TIMA/TMA/TAC storage to the existing Timer; preserve the counter and ABC8 seed. Provide parent-visible register read/write methods and route FF05-FF07 in Bus. No new CPU/System interface or second divider clock.

| Register | Startup storage | Read | Write |
| --- | --- | --- | --- |
| FF05 TIMA | 00 | Stored byte | Retain all bits |
| FF06 TMA | 00 | Stored byte | Retain all bits |
| FF07 TAC | 00 | F8 OR stored value | Store value AND 07 |

TAC bit2 is enable and bits1-0 select the clock. Read operations are pure. This is intentionally a storage/readback checkpoint: TAC-driven counting, DIV/TAC edge effects, overflow/reload and Timer IF are not implemented here, and no functional timer pass is claimed. Preserve existing tick/DIV/STOP behavior; do not introduce a wrapping-add placeholder or fake IRQ. Later contracts extend these write methods with phase/edge effects.

Startup comes from the Pan Docs post-boot register table. TAC's unused-bit behavior is independently specified in pinned [Mooneye unused_hwio-GS.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/unused_hwio-GS.s), which checks upper mask F8 after writing F8 and 00. This is source review, not an executed new fixture.

Assistant acceptance: exhaust all written bytes, TAC mask/aliases, register isolation, startup/read purity and Bus routing. Writes must preserve existing counter phase/IF/IE/elapsed time and unrelated registers/memory. Migrate old FF05-07 fallback/full-memory test oracles explicitly, leaving all independent result/flag/trace/budget expectations intact. Verify DIV/STOP/joypad/interrupt regressions and the full five-fixture suite, formatting, strict Clippy and whitespace before publication. No new allowances/dependencies.

## Decisions before dependent code

1. **Registers/startup:** FF05 TIMA, FF06 TMA, FF07 TAC alongside existing FF04 DIV. Specify masks/readback, writes and ordinary-DMG skipped-boot values; do not invent them.
2. **Timer input:** enabled selected-bit input from the existing divider, explicit base-tick bit indexing and falling edges. Settle DIV/TAC writes and enable transitions without an independent countdown.
3. **Overflow/reload:** explicit delay/reload phases, TIMA zero window, TMA copy and Timer IF boundary, with precise per-tick ordering.
4. **Write collisions:** phase table for TIMA/TMA/DIV/TAC writes, cancellation/ignored writes/reload values. Reconcile contradictory source wording against pinned tests.
5. **Effects/order:** propose Timer reports effects, Bus requests Timer IF, System retains real-cycle clocking, CPU arbitrates normally. Fit reload/write phases to the current four-ticks-before-access boundary or deliberately revise it. Settle STOP-reset versus DIV-write effects without duplicate stopped state.
6. **Evidence:** independent frequency/edge/register/reload/collision/IF/HALT/STOP tests, then a pinned bounded timer-ROM selection with startup/device/reporting/license audit. No unrelated PPU/audio/frontend work.

## Proposed checkpoints

1. Register storage/mapping (#65), then a separately specified ordinary edge-counting slice.
2. Delayed reload, event-to-IF delivery and write collisions.
3. System regressions and explicitly audited external ROM validation.

Finalize interfaces/invariants before each slice. An overflow placeholder must not be presented as timer correctness. [M3 gate](../PROJECT_PLAN.md#milestones) requires timing/interrupt tests and relevant real ROM evidence, not just compilation.

## Source audit starting points

- [Timer registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).
- [Timer obscure behavior](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md).
- [Post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md).
- Pinned Mooneye [TIMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tima_write_reloading.s) and [TMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tma_write_reloading.s).

Initial 2026-11-19 review found conflicting same-reload-cycle TMA wording between the Pan Docs sections. Resolve exact timing/values against pinned tests before adopting either sentence. No programmable timer code/fixture/run is introduced by this brief. Physical STOP #64 and optional DAA #52 are separate.
