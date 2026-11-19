# M3 programmable timer design brief

Active [#53](https://github.com/Lorithan/gameboy-emu/issues/53). **Proposal/questions, not a finalized implementation contract.** User writes application code; assistant owns source audit, design, tests and review. [DIV foundation](../DEVICES.md#divider-phase) and [schedule](../CORE_ARCHITECTURE.md#machine-cycle-schedule) already support real ticks, HALT/STOP and IRQ service.

## Decisions before dependent code

1. **Registers/startup:** FF05 TIMA, FF06 TMA, FF07 TAC alongside existing FF04 DIV. Specify masks/readback, writes and ordinary-DMG skipped-boot values; do not invent them.
2. **Timer input:** enabled selected-bit input from the existing divider, explicit base-tick bit indexing and falling edges. Settle DIV/TAC writes and enable transitions without an independent countdown.
3. **Overflow/reload:** explicit delay/reload phases, TIMA zero window, TMA copy and Timer IF boundary, with precise per-tick ordering.
4. **Write collisions:** phase table for TIMA/TMA/DIV/TAC writes, cancellation/ignored writes/reload values. Reconcile contradictory source wording against pinned tests.
5. **Effects/order:** propose Timer reports effects, Bus requests Timer IF, System retains real-cycle clocking, CPU arbitrates normally. Fit reload/write phases to the current four-ticks-before-access boundary or deliberately revise it. Settle STOP-reset versus DIV-write effects without duplicate stopped state.
6. **Evidence:** independent frequency/edge/register/reload/collision/IF/HALT/STOP tests, then a pinned bounded timer-ROM selection with startup/device/reporting/license audit. No unrelated PPU/audio/frontend work.

## Proposed checkpoints

1. Registers/mapping and ordinary edge-driven counting.
2. Delayed reload, event-to-IF delivery and write collisions.
3. System regressions and explicitly audited external ROM validation.

Finalize interfaces/invariants before each slice. An overflow placeholder must not be presented as timer correctness. [M3 gate](../PROJECT_PLAN.md#milestones) requires timing/interrupt tests and relevant real ROM evidence, not just compilation.

## Source audit starting points

- [Timer registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).
- [Timer obscure behavior](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md).
- [Post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md).
- Pinned Mooneye [TIMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tima_write_reloading.s) and [TMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tma_write_reloading.s).

Initial 2026-11-19 review found conflicting same-reload-cycle TMA wording between the Pan Docs sections. Resolve exact timing/values against pinned tests before adopting either sentence. No programmable timer code/fixture/run is introduced by this brief. Physical STOP #64 and optional DAA #52 are separate.
