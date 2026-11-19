# M3 programmable timer design brief

Active design [#53](https://github.com/Lorithan/gameboy-emu/issues/53). Register foundation #65, tick/reload/write-phase #66, DIV/TAC write edges #67 and the user-approved coarse STOP policy #68 are locally verified. External hardware timer audit/execution/calibration remains incomplete. User writes application code; assistant owns source audit, design, tests and review. [DIV foundation](../DEVICES.md#divider-phase) and [schedule](../CORE_ARCHITECTURE.md#machine-cycle-schedule) define the existing boundaries.

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

## DIV and TAC write-edge contract

[#67](https://github.com/Lorithan/gameboy-emu/issues/67), complete for this declared slice. User extracted increment_tima, guarded by Counting, and reused it for tick and write edges. It advances no time: ordinary increments retain Counting; FF becomes00 with four subsequent ticks pending. No write emits an immediate interrupt; tick remains the only reload-event producer.

Both writes sample timer_input before and after their state change and offer the shared increment only for old=true/new=false. DIV clears the entire counter and ignores its byte; TAC stores value AND07 without resetting/advancing the counter. No other transition increments. This covers high-selected-bit DIV reset, disabling a high input on DMG and selecting a low bit from a high one. Same effective TAC values/upper-bit aliases produce no pulse.

The shared increment accepts only Counting, preserving the existing coarse phase gating. Overflowing/Reloading writes still reset counter or update TAC but do not increment TIMA or cancel/restart/shorten their pending phase. A write-created overflow waits four future ticks, including after disabling TAC. This is the current model contract to be calibrated against pinned timer ROMs, not arbitrary subcycle accuracy. Keep on_stop_entered separate and unchanged; do not silently route it through write_div while STOP reset-edge/pending-phase behavior is unresolved.

Five new assistant tests cover all divider phases/control settings, all selected-bit combinations/old TAC settings/written bytes, aliases/idempotence, Counting-only gating, write-created FF overflow with four future ticks, and CPU DIV/disable writes with delayed IF/trace preservation. The initial guard omission was caught by pending-phase acceptance and fixed by the user. An existing full-address-space oracle now explicitly accounts for its fixed TAC07-to00 sequence overflowing FF TIMA; all unrelated memory/instance checks remain. All 23 timer-focused tests and the full 461 tests pass (336 unit, 125 integration), including five hash-verified existing fixtures; formatting, strict Clippy and whitespace pass. No assistant application edits, dependencies or new external timer result.

Primary write-effect source: [Pan Docs timer circuitry](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md). Pinned [div_write.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/div_write.s) and [rapid_toggle.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/rapid_toggle.s) are future hardware-validation candidates, not executed results.

## Agreed coarse STOP policy

[#68](https://github.com/Lorithan/gameboy-emu/issues/68), verified without application changes. Actual StopEntered clears only the counter once: no manufactured TIMA edge, phase cancellation/restart, register change or IF request/clear. Keep on_stop_entered separate from write_div. This deliberately retains the existing project approximation; reviewed references establish reset and stopped clocks but not the physical reset-to-TIMA edge detail. That hardware assessment remains #64.

While Stopped, System returns before real ticks: Counting, every remaining overflow delay and Reloading are retained. Reads/polls/input do not progress them. Selected JOYP input wakes without clocks/reload/acknowledgment; remaining delay resumes on real ticks. IF/IE alone cannot wake Stopped. A reload/IF produced during STOP's final executing cycle survives entry/pause and may be serviced by ordinary arbitration after selected-input wake. HALT-like/Continue branches do not reset DIV and keep TIMA clocking.

Five new tests cover counter/control/phase reset preservation and exact remaining-tick resume, overflow on entry with whole-memory/register/time-pure stopped polls, IF-only rejection and press/release wake, reload/IF on STOP's fetch cycle followed by five-cycle late-acknowledged timer entry, and conditional HALT-like/Continue traces. All 466 tests pass (341 unit, 125 integration), including five hash-verified existing fixtures; formatting, strict Clippy and whitespace pass. Focused runs passed 27 STOP and 25 timer tests (overlapping sets). Tests/docs only; no new hardware STOP or external timer ROM result. Next audit pinned timer fixtures and calibrate the current model against actual execution.

## Decisions before dependent code

1. **Registers/startup:** specified and implemented above. Extend these methods with reviewed edge/reload behavior in later slices.
2. **Write-induced edges:** #67 above is locally verified; hardware timer audit/calibration remains required.
3. **Reload calibration:** ordinary tick/delay/write-window rules are implemented above; audit and execute pinned hardware timer tests against the current access phase before claiming conformance.
4. **STOP interactions:** coarse policy above is agreed/verified; physical reset-edge fidelity remains #64, not silently equated with DIV writes.
5. **Effects/order:** Timer-to-Bus event delivery and normal CPU arbitration are implemented; preserve the declared ordering or explicitly revise it if evidence requires.
6. **Evidence:** independent frequency/edge/register/reload/collision/IF/HALT/STOP tests, then a pinned bounded timer-ROM selection with startup/device/reporting/license audit. No unrelated PPU/audio/frontend work.

## Proposed checkpoints

1. Register storage/mapping #65 and ordinary tick/reload/write-phase #66 are complete for their scope.
2. DIV/TAC write edges #67 and agreed STOP policy #68 are complete for their declared scope.
3. Next audit/prepare/run pinned timer ROMs, investigate phase mismatches and preserve System regressions.

Finalize interfaces/invariants before each slice. An overflow placeholder must not be presented as timer correctness. [M3 gate](../PROJECT_PLAN.md#milestones) requires timing/interrupt tests and relevant real ROM evidence, not just compilation.

## Source audit starting points

- [Timer registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).
- [Timer obscure behavior](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md).
- [Post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md).
- Pinned Mooneye [TIMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tima_write_reloading.s) and [TMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tma_write_reloading.s).

Initial review found conflicting same-reload-cycle TMA wording between the Pan Docs sections. The implemented coarse model follows the detailed obscure-behavior section: a reload-window TMA write updates TIMA too. Local CPU-boundary tests verify this chosen rule; pinned timer-ROM phase calibration remains required, not claimed. Physical STOP #64 and optional DAA #52 remain separate.
