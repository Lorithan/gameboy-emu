# Divider clock and STOP reset

Task: [#46](https://github.com/Lorithan/gameboy-emu/issues/46), first prerequisite of [#44](https://github.com/Lorithan/gameboy-emu/issues/44), targeting the M3 timer foundation. Status: proposed contract, 2026-11-18; application implementation has not started. User owns application changes; assistant owns tests, documentation and review.

Baseline: current main has documentation through 676f330; the last emulator checkpoint is 6df35c5 with 379 passing tests (279 unit, 100 integration), formatting and strict Clippy passing. No emulator tests were rerun for this planning task. M2's ROM gate remains open; the user selected these STOP prerequisites as current work.

## Scope and ownership

Add private memory/timer.rs, declaring Timer visible to its owning memory module. Bus owns one Timer. Initially its only stored field is a private u16 system counter expressed in base ticks. Name the component Timer because the later programmable timer shares this counter; do not create independent DIV and TIMA clocks.

CPU continues to request/complete cycles without borrowing Bus. System owns scheduling and elapsed_ticks. The counter is resettable device state; elapsed_ticks is accumulated execution time and must never be used as the backing storage for DIV.

Proposed operations (exact spelling is user-owned):

| Owner | Operation | Contract |
| --- | --- | --- |
| Timer | dmg_post_boot() | Explicit normal-DMG startup profile described below; no Default |
| Timer | tick() | Advance exactly one base tick with u16 wrapping |
| Timer | read_div() | Return counter bits 8-15 without mutation |
| Timer | write_div(value) | Ignore the written byte and reset the entire counter |
| Timer | on_stop_entered() | Reset the counter once for this entry event; store no stopped flag |
| Bus | advance_mcycle() | Advance owned clocked devices four base ticks; initially calls Timer::tick four times |
| Bus | on_stop_entered() | Forward the event to Timer |

Map FF04 reads/writes in Bus. Leave FF05-FF07 and other unmapped device registers at their existing fallback behavior for this slice. Do not add TIMA/TMA/TAC fields, timer requests, joypad state, CGB speed switching, frontend setup, dependencies or production counter setters.

Separate DIV-write and STOP-entry operations preserve their intent. They may share a private counter-clear helper now, but this does not assert that future TIMA/APU effects of the two events are identical. Specify those interactions when their devices exist.

## Counter behavior

Read the upper byte of the wrapping counter. DIV therefore changes every 256 base ticks (64 M-cycles), with FF wrapping to 00. Preserve the lower byte as internal phase; storing only the visible DIV byte loses the time until its next change.

Any byte written to FF04 resets the whole counter to zero. It does not load that byte, and clearing only the upper byte is incorrect. After reset, a full 256 base ticks are required for DIV to become 01.

HALT idles and interrupt-entry phases are actual cycles and keep the counter ticking. STOP is already represented as no-cycle polling, so System will not advance the timer while stopped. Do not duplicate CPU stopped state inside Timer or Bus.

## Startup profile and evidence limits

Target the existing normal DMG post-boot profile, not DMG0, MGB, CGB or power-on execution. Pan Docs records visible DIV = AB at PC 0100 for normal DMG. It does not specify the hidden lower byte in that table.

Propose counter seed **ABC8** at our elapsed_ticks = 0 boundary, immediately before requesting the first cartridge opcode. This is a model-phase inference from the hardware-verified Mooneye boot_div-dmgABCmgb source, not a claim that the physical hidden counter is ABC8 at an independently measured hardware boundary.

The pinned common header executes NOP (1 M-cycle), JP 0150 (4), then the phase test executes six NOPs and LDH A,(DIV) (3). Our advance-before-access convention samples that first DIV read after 14 M-cycles. The subsequent PUSH/NOP/read spacings give:

| Read | Cumulative M-cycles | Counter from seed ABC8 | Expected DIV from the source |
| --- | ---: | --- | --- |
| 1 | 14 | AC00 | AC |
| 2 | 78 | AD00 | AD |
| 3 | 141 | ADFC | AD |
| 4 | 205 | AEFC | AE |
| 5 | 269 | AFFC | AF |
| 6 | 334 | B100 | B1 |

The initial visible byte remains AB; the first visible increment occurs after 56 base ticks/14 M-cycles. ABC8 selects an M-cycle-aligned phase consistent with these observations. The source does not establish sub-M-cycle phase in our implementation. If scheduling is refined, revisit this translation rather than preserving the constant without evidence.

Do not choose AB00 solely because it has the right visible byte; it fails the sourced read-phase sequence. Do not substitute a familiar emulator's hidden seed without accounting for its fetch/access convention. Actual external-ROM conformance remains unverified until a pinned/licensed fixture is run; these source-derived acceptance vectors are useful before that gate.

## System ordering

Preserve the existing coarse schedule and add device advancement in one place:

1. Perform existing HALT wake/interrupt acceptance checks and request a CPU cycle.
2. For Stopped or a request error, return without advancing time/devices. A zero instruction budget never reaches this step.
3. For a real action, record tick_before, advance elapsed_ticks by four and advance Bus devices by four base ticks, before servicing the action.
4. Perform its read/write/idle once, then complete CPU with the existing response samples. DIV reads see the advanced counter; DIV writes reset it after those four ticks.
5. If successful completion emits StopEntered, forward on_stop_entered once before returning the trace. The opcode cycle still counts four ticks; the device counter ends at zero. Preserve existing interrupt-selection acknowledgment handling.

Device time advances even if CPU completion then reports an unsupported opcode, matching the existing completion-error trace. Request errors advance neither time counter. Resetting DIV or entering STOP never resets elapsed_ticks. No new public cycle event, response variant or trace field is needed.

This is a specified M-cycle ordering, not a verified physical sampling edge. Future timer overflow/reload and audio interactions need more precise contracts; using one-base-tick advancement internally leaves room for those behaviors without changing public stepping granularity.

## Implementation slices and assistant acceptance

1. **Timer model.** User adds the module, private counter, named constructor and small operations above. Assistant adds unit tests without introducing a public counter-mutation API. Internal construction at controlled phases is allowed in tests.
2. **Bus/System integration.** User adds Timer ownership, FF04 mapping, real-cycle advancement and the existing StopEntered hook. Assistant adds end-to-end phase/reset/pause tests and migrates existing time-dependent memory oracles.
3. **Review/checkpoint.** Run focused/full tests, formatting, strict Clippy and whitespace checks. Record the declared startup/ordering limits. Completing #46 does not complete #44; headless JOYP and wake remain next contracts.

Acceptance cases:

- Sweep all 65,536 counter states for upper-byte reads and one-tick wrap behavior. Verify reads are repeatable and do not advance time.
- Cover hidden phase and all 256 write values; clear both bytes, then verify 255 versus 256 base ticks after reset. Also cover four-tick crossings and full-counter wrap.
- Verify initial DIV AB, the proposed startup phase and all six source-derived read vectors. Translate the source sequence into original synthetic programs; no external ROM fixture is introduced for this task.
- System traces show exactly four ticks/device ticks per real cycle, including HALT and all interrupt-entry phases. A DIV read crossing a byte boundary observes the increment in that cycle.
- DIV writes occur after cycle advancement, reset hidden phase, and preserve cumulative elapsed_ticks. Verify the first later visible increment occurs after 64 subsequent real M-cycles.
- STOP's entry cycle advances once then resets. Positive-budget stopped polls and zero budgets preserve device time; no wake shortcut is added. Startup and direct reset operations do not consume CPU cycles.
- Unsupported-opcode completion retains its elapsed cycle/device advancement; sticky request errors do not advance again.
- Preserve CPU/IME/protocol, memory, trace and budget regressions. Update old unmapped-FF04 expectations and time-dependent full-memory snapshots with independent clock/reset expectations. Do not broadly skip I/O checks or hide actual CPU writes; FF04 can now change with time and reacts to writes.

## Sources consulted 2026-11-18

- [Pan Docs DIV](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md): rate, write reset and STOP reset/pause.
- [Pan Docs shared counter](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md): DIV as visible system-counter state and the future edge-sensitive timer/audio connections.
- [Pan Docs startup table](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md): visible AB for normal DMG, distinct from DMG0.
- [Mooneye boot_div-dmgABCmgb](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/boot_div-dmgABCmgb.s) and [common header](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/common.s): MIT-licensed source, DMG ABC phase/result reference. Source version and expected values are recorded here; no external source/binary fixture was copied or executed.
