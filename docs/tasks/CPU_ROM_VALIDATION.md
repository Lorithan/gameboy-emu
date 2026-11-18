# First external CPU ROM validation

Task: [#45](https://github.com/Lorithan/gameboy-emu/issues/45), M2. Status: proposed contract, researched 2026-11-18. No fixture or runner has been added and no external ROM has been run. Baseline 6df35c5 has 379 passing tests; they were not rerun for this documentation-only work.

## First slice

Start with Mooneye acceptance/bits/reg_f. It tests F's low-bit masking through assembled stack/register instructions. Its source records passing results on DMG. This is a narrow external check, not an arithmetic/timing suite or completion of M2's entire validation gate.

The source audit suggests this fixture can run with our current RAM/HRAM, CPU and fallback I/O, without a PPU or serial implementation. This is an inference to verify by execution, not a passing result. The suite explicitly supports a fast reporting path when LY (FF44) and SC (FF02) read FF, matching the current Bus fallback. Do not add production overrides to make a test pass.

## Provenance before fixture introduction

| Item | Recorded value or requirement |
| --- | --- |
| Suite | Gekkio/mooneye-test-suite |
| Source revision | 31510e12eea6286d36eea060a6adde755e1067aa, resolved from upstream main on 2026-11-18 |
| Selected source | acceptance/bits/reg_f.s |
| Suite source license | MIT; preserve upstream copyright/license notices |
| Included asset | common/common.s includes Darkrose's font.bin; record the asset's separate provenance/notices before binary redistribution |
| Proposed hardware scope | DMG, skipped boot ROM/current documented post-boot startup |
| Build reference | Upstream CI pins WLA-DX 89a90a56be5c2b8cf19a9afa3e1b32384ddb1a97; retain its assembler/linker identity and exact invocation if building locally |
| Required binary shape | 32 KiB, ROM-only, no cartridge RAM, valid header accepted by the existing Cartridge loader |
| Expected outcome | Explicit passing report, described below |
| Still required | Exact artifact origin, ROM SHA-256, matching build/map details and component notices before introducing a fixture |

First use an externally supplied/cache fixture with provenance recorded in a small manifest. Do not commit ROM binaries as part of this planning task, patch a ROM/header to bypass the loader, or fetch a moving latest release during tests. If using a prebuilt release, verify that its recorded revision matches the pin; an upstream main filename alone is insufficient.

The font's attributed upstream page distinguishes GPLv3-or-later font.c from the image's additional GPLv2/CC-BY-SA choices. The exact derivation of font.bin has not been established here. Do not describe an entire generated ROM as exclusively MIT based only on the suite's root license; record the component provenance when preparing fixtures.

## Runner ownership and flow

Assistant owns the test-only helper and its tests, using existing public APIs. User owns any application changes justified by resulting failures. No SDL2, production test-opcode handling, serial device stub, generic emulator framework or new application API is proposed for the first fixture.

1. Resolve the fixture from an explicit external location. Validate provenance/digest during fixture preparation, then load through Cartridge::from_filepath and construct System normally. Record loader failure separately from execution failure.
2. Call System::step_machine_cycle repeatedly, keeping a bounded tail of traces and a real-cycle count.
3. For each successful MachineStep::Cycle, inspect FF01 write actions and feed their values to the fixture-specific result observer. Inspect CPU snapshots only as needed to validate the report.
4. Stop on an explicit report, CPU/System error, unexpected stopped state, malformed report or exhausted limit.
5. Return diagnostics including the fixture identity, outcome, cycles/ticks, CPU snapshot and last 64 real traces. The test assertion decides whether the run passed.

System remains unaware of Mooneye. FF01 writes remain ordinary bus actions, even though Bus currently ignores the unmapped register. Observing those requested writes through CycleTrace is a harness capability; it does not establish serial transfer correctness.

## Result detection

The pinned reg_f body performs no SB writes before its common reporting path. quit.s selects the fast path when SC is unavailable and emits six FF01 writes. The general suite also supplies an LD B,B breakpoint protocol, but a raw byte 40 in an arbitrary read is not enough to identify an opcode fetch or a result.

For this audited fixture, observe the first complete six-byte SB report and require matching register values:

| Result | FF01 write packet | B/C/D/E/H/L at packet completion |
| --- | --- | --- |
| Passed | Decimal 3, 5, 8, 13, 21, 34 | Same six values |
| Reported failure | Hex 42, 42, 42, 42, 42, 42 | All 42 hex |
| Malformed report | Any other packet or register mismatch | Diagnostic failure |

Do not scan past malformed output looking for a later accidental match. A partial packet at timeout is not success. Ignore writes to other addresses for packet collection. Apply this observer only to fixtures whose reporting paths have been audited; this is not a universal ROM success detector.

## Bounded execution and outcomes

Proposed initial reg_f limits: 1,000,000 M-cycles and a secondary 10-second host-time limit. These are generous test caps, not expected hardware duration; measure and record the first real run before finalizing them. Do not silently raise a cap when a test hangs. Keep host timing in the harness and use emulated cycles for deterministic execution accounting.

Distinguish Passed, ReportedFailure, MalformedReport, TimedOut, UnexpectedStop and Cpu/SystemError. Preserve the actual emulator error and completion-cycle record when available. Invalid or missing fixtures are setup errors and must be visible; an optional fixture test may be explicitly ignored, but a required external-validation job cannot turn missing data into success.

MachineStep::Stopped must immediately produce UnexpectedStop for reg_f. A cycle budget alone cannot bound a loop that receives no-cycle results forever. HALT idles are real cycles and consume the budget normally. Reaching any instruction count, arbitrary breakpoint or stable PC does not establish success.

## Assistant acceptance tests for the runner

Before relying on a real ROM, use small original synthetic ROM programs to exercise:

- Exact success packet plus register signature, and exact explicit failure packet.
- Wrong registers despite matching output, malformed packet and partial packet timeout.
- Correct bytes sent to another address, unrelated read value 40 and an ordinary self-loop; none can pass.
- Exact cycle-budget boundary, CPU fault with preserved diagnostics and immediate unexpected STOP termination.
- Fixture setup errors and bounded trace-buffer retention.

Then prepare the pinned fixture with its provenance manifest, run reg_f through the same public path and record the actual result, packet, cycle count and any failures. No ignored/empty test run counts as external evidence. Audit further ROMs separately for memory mapping, timer/interrupt, startup, PPU, serial and hardware-model prerequisites before expanding the selection.

## Sources consulted

Pinned upstream resources:

- [README and result protocol](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/README.markdown).
- [reg_f](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/reg_f.s).
- [Reporter](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s), [PPU probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s), [serial probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_serial_broken.s).
- [Common cartridge/library configuration](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/common.s), [build rules](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/Makefile), [CI toolchain pin](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/.github/workflows/ci.yml), [suite license](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/LICENSE).
- [Attributed Darkrose font page](https://opengameart.org/content/8x8-ascii-bitmap-font-with-c-source).
