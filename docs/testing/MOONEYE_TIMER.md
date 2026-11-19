# Pinned Mooneye timer validation

Source/prerequisite audit and explicit execution, 2026-11-19, [#69](https://github.com/Lorithan/gameboy-emu/issues/69) under timer design #53. Twelve fixtures pass; rapid_toggle reports a reproducible failure tracked by [#70](https://github.com/Lorithan/gameboy-emu/issues/70). Assistant owns fixtures/tests/review; application fixes remain user-owned. No application/configuration/dependency change or ROM binary is introduced. Emulator base is 2b28f50; this work adds tests/metadata only.

## Identity and prerequisites

All thirteen sources under acceptance/timer at [31510e12eea6286d36eea060a6adde755e1067aa](https://github.com/Gekkio/mooneye-test-suite/tree/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/timer) were inspected. [Manifest](../../tests/fixtures/mooneye-timer.json) records per-ROM/symbol SHA-256 and source Git blob IDs. ROMs/symbols/LICENSE match their entries in the pinned official ZIP, whose SHA-256 is 18aa29462dfe1fcd32a2cb3621733abdc72410074918b9245aa99a6920f7d3f2. Each unchanged image is 32768 bytes, type/ROM-size/RAM-size codes zero, valid header checksum 2d. Symbols place main/test at 0150 and quit@halt at 4878 (div_write) or 4ab3 (others).

Shared archive/build/toolchain, suite MIT notice and separate Darkrose font provenance are inherited from [reg_f](MOONEYE_REG_F.md) and its [manifest](../../tests/fixtures/mooneye-reg-f.json). Every font region at 4000-47ef matches the pinned 2032-byte asset hash. No independently reproduced/signed build is claimed; the whole binary is not labeled exclusively MIT. Binaries stay in ignored target/test-roms.

The sources use ROM/WRAM/HRAM, the implemented CPU/timer/IF/IE and normal skipped-boot startup. They initialize relevant timer/IRQ state and explicitly reset DIV, so this is not authentic boot or startup-phase certification. No body uses a mapper, PPU/audio device, STOP or an earlier SB report. Assertions/quit use the already audited common reporter; normal fallback LY/SC reads of FF select its fast six-write path. The observer checks first FF01 packet and B/C/D/E/H/L decimal signature 3,5,8,13,21,34. This is not serial/PPU validation and does not require production overrides.

## Selected cases

| Fixtures | Independent source checks |
| --- | --- |
| tim00, tim01, tim10, tim11 | TIMA read timing for all four selected rates; expected D/E are 04/05 except tim01 08/09 |
| tim00_div_trigger, tim01_div_trigger, tim10_div_trigger, tim11_div_trigger | DIV reset falling-edge effects; expected D/E are 04/05, 0a/0b, 05/06, 04/05 respectively |
| div_write | Repeated DIV reset prevents slow-clock overflow/service; any Timer interrupt is failure |
| rapid_toggle | Enable/disable preserves phase and adds expected edge increments; timer handler asserts BC=ffd9 |
| tima_reload | Zero window and current-TMA reload without shifting next increment; expected B/C/D/E/H/L=fe/fe/ff/00/ff/00 before reporter |
| tima_write_reloading | Ignored reload-window TIMA write and neighboring writes; expected C/D/E/L=fe/80/7f/7f |
| tma_write_reloading | TMA propagation around reload; expected C/D/E/L=fe/7f/7f/fe |

## Limits and reproduction

Retain 1000000 M-cycles and 10 host seconds per fixture, last 64 completed traces and failed completion record diagnostics. div_write's 65535-iteration loop takes eleven M-cycles per taken iteration, approximately 720884 M-cycles total before setup/reporting, fitting the original cap. Other bodies are bounded short sequences or rapid_toggle, whose expected timer interrupt ends the loop promptly; absent service must fail, not raise limits. Unexpected STOP, missing/malformed output, faults or either timeout remain failures.

Place unchanged artifacts under target/test-roms/mooneye, then use the checksum-gated command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-timer.ps1
```

Select one with -Fixture tima_reload (or another manifest ID); optionally supply -RomRoot. The process-local policy flag changes no persistent settings. Default fixture tests are explicitly ignored, never counted as passes. Source audit is complete before this first run; no outcome is presumed from source annotations or local synthetic tests.

## Actual results

The checksum-gated thirteen-fixture command was executed, followed by a sequential rerun to capture complete reports and an isolated rapid_toggle reproduction. Counts/outcomes agree. Limits stayed unchanged. Passing fixtures emitted the first packet/register signature decimal 3,5,8,13,21,34; rapid_toggle emitted six42 hex in packet and registers, an explicit reported failure. No timeout, STOP, CPU fault or malformed report occurred. Full snapshots and observed host durations are in the manifest; durations are environment-specific diagnostics, not performance promises.

| Fixture | Actual outcome | M-cycles | Base ticks |
| --- | --- | ---: | ---: |
| div_write | Passed | 721,229 | 2,884,916 |
| rapid_toggle | ReportedFailure | 4,852 | 19,408 |
| tim00 | Passed | 4,476 | 17,904 |
| tim00_div_trigger | Passed | 4,210 | 16,840 |
| tim01 | Passed | 3,996 | 15,984 |
| tim01_div_trigger | Passed | 3,994 | 15,976 |
| tim10 | Passed | 3,991 | 15,964 |
| tim10_div_trigger | Passed | 3,995 | 15,980 |
| tim11 | Passed | 4,092 | 16,368 |
| tim11_div_trigger | Passed | 4,017 | 16,068 |
| tima_reload | Passed | 4,677 | 18,708 |
| tima_write_reloading | Passed | 4,321 | 17,284 |
| tma_write_reloading | Passed | 4,337 | 17,348 |

These are finite hardware-source comparisons, not full physical timing or STOP certification. Reload/write passes provide independent evidence for the current chosen TMA propagation rule and M-cycle access convention, while rapid_toggle shows that at least one timer/CPU boundary remains wrong. Expected pass criteria remain unchanged for the failing fixture; it is not marked expected-failure or omitted from required runs.

## rapid_toggle investigation

Pinned source asserts BC=FFD9 at test_finish=0190. Bounded test-only diagnostics observe BC=FFD8 there at tick2792, before report macros replace the registers. The final trace includes TAC=00 write at tick2748 with BCFFD9, DEC BC fetch at tick2752, idle completion at tick2756 changing BC to FFD8, interrupt-entry start at2760, Timer selection/acknowledgment at2772, entry complete at0050 at2776, and JP to0190 at2792. Stacked return PC is016E.

The extra decrement before service is measured. Whether the cause is request assertion phase, fetch-boundary acceptance or another CPU/timer timing detail needs source-backed investigation; public traces alone do not prove a fix. Do not shorten the reload delay, raise budgets, patch ROMs or weaken the BC assertion simply to force a pass. [#70](https://github.com/Lorithan/gameboy-emu/issues/70) owns the user implementation follow-up; the fixture regression and bounded diagnostics are retained. #53 and M3 remain open. DAA #52 and physical STOP #64 remain separate.
