# DMG programmable timer

Implemented contract for completed M3, reviewed at f0d38ef on 2026-11-19. [Completion decision](M3_COMPLETION_REVIEW.md), [ROM evidence](testing/MOONEYE_TIMER.md), [System schedule](CORE_ARCHITECTURE.md#machine-cycle-schedule). This is the declared coarse DMG model, not arbitrary T-state/pin accuracy. Implementation history is in #53, #65-#70 and Git; this guide replaces the completed task proposal.

## Ownership and registers

Bus owns Timer and routes FF04-FF07. Timer owns the wrapping u16 system counter, TIMA, TMA, TAC and reload phase. DIV reads the counter's high byte. The counter's ABC8 startup seed is a [model-phase inference](DEVICES.md#divider-phase), not a hidden hardware measurement; authentic boot is deferred.

| Register | Storage at startup | Read | Write |
| --- | --- | --- | --- |
| FF04 DIV | Counter ABC8 | Counter high byte | Ignore byte, clear entire counter, apply falling-edge rule |
| FF05 TIMA | 00 | Stored byte | Phase-dependent below |
| FF06 TMA | 00 | Stored byte | Store; propagate during Reloading |
| FF07 TAC | 00 | F8 OR stored low three bits | Retain low three bits, apply falling-edge rule |

Reads are pure. TAC enable is bit2; selectors 0/1/2/3 use counter bits 9/3/5/7, giving periods 1024/16/64/256 base ticks. Input is enabled AND the selected counter bit. The counter advances even when the programmable timer is disabled.

## Counting, overflow and writes

Timer compares old/new input around each tick, DIV reset or TAC update. Only high-to-low increments TIMA, and only in Counting. TAC changes never reset or advance the counter. Repeating an effective TAC value creates no edge. DIV/TAC writes during pending/reload phases still update their own storage but do not cancel/restart the pending delay or increment TIMA.

FF incrementing to00 enters Overflowing with four further base ticks remaining. The overflowing tick consumes none of its own delay. Countdown progresses independently of enable or further falling edges; expiry copies current TMA into TIMA, enters Reloading and emits one InterruptRequested event. Bus handles the event on that tick and sets Timer IF regardless of IE/IME. Writing FF or00 directly is not an overflow trigger.

| Write | Counting | Overflowing | Reloading |
| --- | --- | --- | --- |
| TIMA | Store | Store and return to Counting, cancelling pending reload/request | Ignore |
| TMA | Store | Store; expiry uses current value | Store and copy to TIMA |

Cancellation preserves any already-set IF. Reloading is a coarse access-window marker: the next tick advances DIV, clears the marker and emits no event. The TIMA/TMA boundary fixtures support the chosen rule at tested M-cycle alignments, not all possible physical phases.

## System integration and interrupt fetch

Each executing M-cycle advances elapsed time once by four and calls Bus advancement for four Timer ticks. System initially offers pending interrupts for HALT wake and acceptance, then requests a provisional action. Stopped and existing request faults return without clocks.

After device advancement, System offers fresh pending interrupts before executing the bus action. CPU alone gates acceptance on FetchOpcode and enabled IME. If accepted, System refreshes the action from InterruptEntry and completes that idle in the already elapsed cycle. It never executes the stale read or advances again. Our entry trace remains idle/idle/high-write/low-write/idle; late IE/IF capture, selection/acknowledgment and cancellation remain [CPU policy](CPU.md#ime-and-interrupt-entry). Operand reads, CB suffixes and internal instruction cycles cannot be replaced.

This #70 correction fixed rapid_toggle's extra DEC before service. The source-backed fresh-fetch regression failed before the user change and passes afterward; unchanged rapid_toggle now reports success at 4605 M-cycles /18420 ticks. The original failure and finite ROM reporting/provenance are retained in [timer evidence](testing/MOONEYE_TIMER.md).

## HALT and agreed coarse STOP policy

HALT idles and interrupt-entry cycles keep advancing devices. Actual StopEntered clears only the counter once, without a synthesized TIMA edge or changes to TIMA/TMA/TAC, reload phase or IF. Keep on_stop_entered separate from write_div. HALT-like/Continue STOP branches do not reset DIV and keep clocking.

Stopped polling and input consume no ticks. Counting, each pending delay and Reloading remain frozen. Selected JOYP input wakes without clocks, DIV reset, reload or IF acknowledgment; real ticks resume the retained phase. IF/IE alone cannot wake Stopped. If delayed EI permits STOP to execute, a reload request during its fetch survives entry/pause and may be serviced after selected-input wake. Already-enabled eligible fetch-time interrupts replace STOP before it executes, so no padding skip or DIV reset occurs.

This counter-only reset and immediate selected-input wake are agreed project approximations. Oscillator startup, analog/reset-to-TIMA edges, revision-dependent glitches and vector corruption remain [#64](https://github.com/Lorithan/gameboy-emu/issues/64), separate from M3's declared scope. [Device guide](DEVICES.md#stop-entry-and-wake) records the branch table and limits.

## Evidence and references

The verified f0d38ef checkpoint passed 484 tests, including all thirteen timer fixtures and five prior external fixtures, with formatting and strict Clippy. Local coverage includes exhaustive input/counter/control/byte cases, pending/reload writes, IF independence, full-memory effects, exact cycle traces, entry pause budgets, in-progress instruction guards and STOP pause/resume. See [testing](TESTING.md) for actual commands, hashes, reporting, limits and unrun follow-ups.

- [Pan Docs timer registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md) and [timer circuitry/reload details](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md). The detailed reload section's TMA propagation rule is used; source wording elsewhere differed, and the pinned TMA-write fixture passes.
- [Gekkio GB Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf), revision 199 dated November 6, 2026, detailed LDH/DEC timing, and [CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#isr-and-nmi) support fetch-associated interrupt checking without establishing our exact subcycle sampling edge.
- [Pinned Mooneye timer sources](https://github.com/Gekkio/mooneye-test-suite/tree/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer), audited artifacts and measured results in the [fixture record](testing/MOONEYE_TIMER.md).
