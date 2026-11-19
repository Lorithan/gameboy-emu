# M3 programmable timer design brief

Active design [#53](https://github.com/Lorithan/gameboy-emu/issues/53). Register foundation #65 and tick/reload/write-phase slice #66 are implemented and verified for the current coarse schedule. DIV/TAC write edges, STOP interactions and external timer validation remain incomplete. User writes application code; assistant owns source audit, design, tests and review. [DIV foundation](../DEVICES.md#divider-phase) and [schedule](../CORE_ARCHITECTURE.md#machine-cycle-schedule) define the existing boundaries.

## First implementation slice: register storage and mapping

[#65](https://github.com/Lorithan/gameboy-emu/issues/65), M3, complete 2026-11-19. User added private TIMA/TMA/TAC storage, parent-visible register methods and FF05-FF07 Bus routing while preserving the counter and ABC8 seed. No new CPU/System interface or second divider clock.

| Register | Startup storage | Read | Write |
| --- | --- | --- | --- |
| FF05 TIMA | 00 | Stored byte | Retain all bits |
| FF06 TMA | 00 | Stored byte | Retain all bits |
| FF07 TAC | 00 | F8 OR stored value | Store value AND 07 |

TAC bit2 is enable and bits1-0 select the clock. Read operations are pure. This is intentionally a storage/readback checkpoint: TAC-driven counting, DIV/TAC edge effects, overflow/reload and Timer IF are not implemented here, and no functional timer pass is claimed. Preserve existing tick/DIV/STOP behavior; do not introduce a wrapping-add placeholder or fake IRQ. Later contracts extend these write methods with phase/edge effects.

Startup comes from the Pan Docs post-boot register table. TAC's unused-bit behavior is independently specified in pinned [Mooneye unused_hwio-GS.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/unused_hwio-GS.s), which checks upper mask F8 after writing F8 and 00. This is source review, not an executed new fixture.

Assistant added five tests: exhaustive byte/mask/isolation and startup/read-purity checks, Bus phase/IF/IE/instance/full-memory preservation, and CPU LDH/trace/time round trips. Existing divider test setups gained explicit register fields; fallback/full-memory and LDH sweep oracles now independently describe FF05-07 storage/readback. The first full run exposed the stale LDH expectation; that test-only migration fixes it without weakening result/flag/trace/budget checks. All 447 tests pass (322 unit, 125 integration), explicitly including five hash-verified external fixtures; formatting, strict Clippy and whitespace checks pass. No assistant application edits, allowances, dependencies or new external fixture.

## Tick, reload and write-phase contract

[#66](https://github.com/Lorithan/gameboy-emu/issues/66), complete for this declared slice. Timer owns Counting, Overflowing { ticks_left: 1..=4 } and Reloading. timer_input is enabled AND the selected divider bit (9/3/5/7 for TAC selectors 0/1/2/3). Normal tick falling edges increment TIMA. FF becomes00 and schedules four further ticks; the overflowing tick does not consume its own delay. DIV advances on every tick and the delay advances independently of enable/edges. Expiry copies current TMA, enters Reloading and emits one TimerEvent::InterruptRequested. Bus handles each tick event immediately, requesting IF independently of IE/IME.

Reloading is a coarse scheduling marker visible to a bus write after the reload tick. The next tick advances DIV, clears the marker and emits no event. This is not a physical subcycle fidelity claim; arbitrary boundary alignment still requires timer-ROM validation.

| Write | Counting | Overflowing | Reloading |
| --- | --- | --- | --- |
| TIMA | Store | Store and return to Counting, cancelling pending reload/request | Ignore |
| TMA | Store | Store; expiry uses current value | Store and copy to TIMA |

Cancellation does not clear already-set IF. Writing FF/00 directly is not an overflow trigger. Nine assistant tests added since #65 cover the exhaustive input waveform, ordinary periods, four-tick/wrapping/one-shot reload, Bus IF, all-byte delay/window writes, prior-IF preservation and CPU writes at the actual reload bus boundary. Timer literal setups now initialize Counting. All 456 tests pass (331 unit, 125 integration), including five hash-verified existing fixtures; formatting, strict Clippy and whitespace pass. No assistant application edits or new external timer fixture/result. DIV/TAC write-induced edges and STOP behavior with a pending reload remain next design work.

## Decisions before dependent code

1. **Registers/startup:** specified and implemented above. Extend these methods with reviewed edge/reload behavior in later slices.
2. **Write-induced edges:** specify DIV/TAC old/new input transitions through the same increment/overflow path, including pending/reload phases. Keep one divider clock.
3. **Reload calibration:** ordinary tick/delay/write-window rules are implemented above; audit and execute pinned hardware timer tests against the current access phase before claiming conformance.
4. **STOP interactions:** settle reset-edge/pending-reload/pause/resume behavior without guessing that STOP reset and DIV write are identical or duplicating stopped state.
5. **Effects/order:** Timer-to-Bus event delivery and normal CPU arbitration are implemented; preserve the declared ordering or explicitly revise it if evidence requires.
6. **Evidence:** independent frequency/edge/register/reload/collision/IF/HALT/STOP tests, then a pinned bounded timer-ROM selection with startup/device/reporting/license audit. No unrelated PPU/audio/frontend work.

## Proposed checkpoints

1. Register storage/mapping #65 and ordinary tick/reload/write-phase #66 are complete for their scope.
2. Specify and implement DIV/TAC write edges and STOP interactions.
3. Audit/run pinned timer ROMs, investigate phase mismatches and preserve System regressions.

Finalize interfaces/invariants before each slice. An overflow placeholder must not be presented as timer correctness. [M3 gate](../PROJECT_PLAN.md#milestones) requires timing/interrupt tests and relevant real ROM evidence, not just compilation.

## Source audit starting points

- [Timer registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).
- [Timer obscure behavior](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md).
- [Post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md).
- Pinned Mooneye [TIMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tima_write_reloading.s) and [TMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tma_write_reloading.s).

Initial review found conflicting same-reload-cycle TMA wording between the Pan Docs sections. The implemented coarse model follows the detailed obscure-behavior section: a reload-window TMA write updates TIMA too. Local CPU-boundary tests verify this chosen rule; pinned timer-ROM phase calibration remains required, not claimed. Physical STOP #64 and optional DAA #52 remain separate.
