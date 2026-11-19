# M3 programmable timer design brief

Active design [#53](https://github.com/Lorithan/gameboy-emu/issues/53), awaiting M3 completion review. Local slices #65-#68 are verified for their declared coarse scope. All [thirteen pinned timer fixtures](../testing/MOONEYE_TIMER.md) pass after the user's [#70](https://github.com/Lorithan/gameboy-emu/issues/70) System fetch-boundary correction. Latest full fixture-inclusive run: 484 passed, zero failed (346 unit, 138 integration), including eighteen external fixtures; formatting and strict Clippy pass. User owns application code; assistant owns design/tests/review. [DIV](../DEVICES.md#divider-phase) and [schedule](../CORE_ARCHITECTURE.md#machine-cycle-schedule) define current boundaries.

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

While Stopped, System returns before real ticks: Counting, every remaining overflow delay and Reloading are retained. Reads/polls/input do not progress them. Selected JOYP input wakes without clocks/reload/acknowledgment; remaining delay resumes on real ticks. IF/IE alone cannot wake Stopped. If IME gating permits STOP to execute (including delayed EI), a reload/IF produced during its fetch survives entry/pause and may be serviced by ordinary arbitration after selected-input wake. With IME already enabled, #70 arbitration replaces that STOP fetch before execution. HALT-like/Continue branches do not reset DIV and keep TIMA clocking.

Five new tests cover counter/control/phase reset preservation and exact remaining-tick resume, overflow on entry with whole-memory/register/time-pure stopped polls, IF-only rejection and press/release wake, reload/IF on STOP's fetch cycle followed by five-cycle late-acknowledged timer entry, and conditional HALT-like/Continue traces. All 466 tests pass (341 unit, 125 integration), including five hash-verified existing fixtures; formatting, strict Clippy and whitespace pass. Focused runs passed 27 STOP and 25 timer tests (overlapping sets). Tests/docs only; no new hardware STOP or external timer ROM result. Next audit pinned timer fixtures and calibrate the current model against actual execution.

## Decisions before dependent code

1. **Registers/startup:** specified and implemented above. Extend these methods with reviewed edge/reload behavior in later slices.
2. **Write-induced edges:** #67 above is locally verified; hardware timer audit/calibration remains required.
3. **Calibration:** all thirteen selected timer fixtures pass, including rapid_toggle after the #70 correction below. Review M3 scope/limits before closing the gate; finite fixture passes do not certify every T-state alignment.
4. **STOP interactions:** coarse policy above is agreed/verified; physical reset-edge fidelity remains #64, not silently equated with DIV writes.
5. **Effects/order:** Timer-to-Bus event delivery and normal CPU arbitration are implemented; preserve the declared ordering or explicitly revise it if evidence requires.
6. **Evidence:** independent frequency/edge/register/reload/collision/IF/HALT/STOP tests, then a pinned bounded timer-ROM selection with startup/device/reporting/license audit. No unrelated PPU/audio/frontend work.

## Proposed checkpoints

1. Register storage/mapping #65 and ordinary tick/reload/write-phase #66 are complete for their scope.
2. DIV/TAC write edges #67 and agreed STOP policy #68 are complete for their declared scope.
3. All thirteen pinned timer fixtures and the full 484-test suite pass after #70. Next review the #53/M3 completion gate and retain documented limits.

Finalize interfaces/invariants before each slice. An overflow placeholder must not be presented as timer correctness. [M3 gate](../PROJECT_PLAN.md#milestones) requires timing/interrupt tests and relevant real ROM evidence, not just compilation.

## #70 fetch-boundary correction

Investigation 2026-11-19: [Gekkio GB Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf), revision 199 dated November 6, 2026, detailed LDH timing (printed page36) checks interrupts in the following fetch_cycle; DEC rr (printed page80) separates its operation from the next fetch. [CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#isr-and-nmi) describes interrupt dispatch replacing the fetched instruction with return-PC correction. Together with pinned rapid_toggle's independent FFD9 expectation, these support making a device request visible before committing the next opcode in our coarse model. They do not establish an exact T-state sampling point or certify all four-tick alignments.

User implemented the reviewed System-only correction:

1. Preserve beginning-of-cycle arbitration and HALT wake. Request a provisional action; Stopped and existing faults still return before clocks.
2. Advance elapsed time and Bus devices exactly once, as today.
3. Offer the freshly computed pending mask to CPU::try_accept_interrupt before any bus action. CPU still owns the FetchOpcode/IME eligibility gate; do not infer fetch eligibility from a Read action, which can also be an operand or CB suffix.
4. If accepted, refresh the requested action from the CPU. Its new InterruptEntry state requests Idle. Execute/record that refreshed action and complete its matching response in the already elapsed cycle. Never execute or complete the stale opcode read, and never add a second advance or a sixth entry cycle.
5. Preserve late IE/IF capture, selection/acknowledgment, five-cycle entry, EI/DI/RETI, current HALT wake, STOP/fault clocks and timer phases. No Timer/CPU protocol change was required.

Post-advance sampling is an explicit coarse projection, not exact pin timing. The unchanged rapid_toggle now passes at 4605 M-cycles /18420 ticks, with the ordinary first pass packet and register signature. All thirteen timer fixtures pass; the timer delay and CPU protocol are unchanged.

Four original synthetic tests in src/system/tests/interrupt_entry.rs cover fresh requests during opcode fetch, already-pending requests across all pause budgets, IME/IE masking, and requests during an immediate operand, CB suffix or internal DEC completion. All pass. The fresh-fetch regression failed before the user change and now verifies idle/idle/high-write/low-write/idle, preserved BC, the unexecuted opcode address on the stack, one acknowledgment, twenty ticks and no instruction completion. The tests use real DIV/TIMA ticking and ordinary instructions. An existing STOP test now uses NOP;EI;STOP so delayed IME still permits entry and retains the reload IRQ; a separate new test preserves EI;NOP;STOP and verifies that fully enabled arbitration replaces STOP, retains its opcode return address and never resets DIV or skips padding. This explicitly supersedes that old enabled-fetch expectation without removing its delayed-EI STOP coverage.

## Source audit starting points

- [Timer registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).
- [Timer obscure behavior](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md).
- [Post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md).
- Pinned Mooneye [TIMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tima_write_reloading.s) and [TMA write/reload](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer/tma_write_reloading.s).

Initial review found conflicting same-reload-cycle TMA wording between Pan Docs sections. The model follows the detailed section: reload-window TMA writes update TIMA too. Local CPU-boundary checks and the actual pinned tma_write_reloading pass now support that chosen behavior; this does not establish arbitrary pin-phase fidelity. rapid_toggle demonstrates a remaining defect and prevents a full timer-conformance claim. Physical STOP #64 and DAA #52 remain separate.
