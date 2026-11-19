# Mooneye CPU, interrupt and divider subset

Task [#50](https://github.com/Lorithan/gameboy-emu/issues/50), completed 2026-11-18. Follow-up to the [first reg_f result](MOONEYE_REG_F.md). This record preserves original source/artifact audits and measurements; [testing guide](../TESTING.md) gives current aggregate verification. Assistant owns tests/review; no user application fix was needed. This is a finite four-ROM subset, not a complete CPU-suite pass.

## Source and prerequisite audit

All sources are pinned at [31510e12eea6286d36eea060a6adde755e1067aa](https://github.com/Gekkio/mooneye-test-suite/tree/31510e12eea6286d36eea060a6adde755e1067aa). Local ROMs and symbols match their official ZIP entries; hashes are recorded in [mooneye-cpu-subset.json](../../tests/fixtures/mooneye-cpu-subset.json). Shared archive/build/toolchain/MIT/font provenance is inherited explicitly from [mooneye-reg-f.json](../../tests/fixtures/mooneye-reg-f.json). All four font regions match the pinned 2032-byte source asset; no binary redistribution or independently reproduced build is claimed.

All four artifacts are unchanged 32 KiB ROM-only images with no cartridge RAM and header checksum 2D. They load through the normal Cartridge API. They use the current documented DMG post-boot profile; authentic boot execution, startup DIV phase and another console model are not being tested.

| Source | Independent assertions | Actual prerequisites and eligibility |
| --- | --- | --- |
| [if_ie_registers.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/if_ie_registers.s) | IF upper-bit readback, IE gating, one serviced request and acknowledgment | Initializes IF/IE, requests serial interrupt in software, uses startup SP/HRAM and RETI. No physical serial device or TIMA needed. Runnable now. |
| [interrupts/ie_push.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/interrupts/ie_push.s) | High stack-write cancellation/vector zero/IF retention, disabled IME after cancellation, low-write too-late cancellation and changed priority | Explicit SP and software IF/IE requests; requires wrapped stack writes and existing late selection. PPU-disable helper has a bounded wait described below. No timer IRQ generation needed. Runnable now. |
| [ei_sequence.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ei_sequence.s) | Consecutive EI does not keep postponing acceptance; saved return address is 01A2 | Initializes IME/IF/IE and requests serial in software; uses startup SP/HRAM and POP BC. No serial device or TIMA needed. Runnable now. |
| [div_timing.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/div_timing.s) | Read before/at DIV increment boundary and reset of the hidden counter | Resets DIV explicitly and uses controlled NOP/read spacing. Requires the implemented divider, not TIMA/TMA/TAC or startup divider calibration. Runnable now. |

Each body reaches the common [quit reporter](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s) and emits no earlier SB report. Assertion/quit macros disable IME before reporting. With ordinary FF44/FF02 fallback reads of FF, the PPU probe skips display setup and the serial probe selects six direct FF01 writes. Therefore the existing packet-plus-register observer is valid for these audited fixtures; this does not establish serial or PPU correctness. No production overrides were introduced.

ie_push explicitly calls [disable_ppu_safe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/disable_ppu_safe.s). Because LCDC/LY read FF, its [wait_ly_with_timeout](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/wait_ly_with_timeout.s) loops until its 16-bit counter wraps. Source review estimated approximately 851,968 M-cycles for the loop, so the original one-million-cycle cap was retained before execution. It returns normally after timeout; no PPU emulation or artificial wake was added.

## Actual execution results

Every result below had both the decimal FF01 packet and B/C/D/E/H/L signature 3,5,8,13,21,34. Limits stayed at 1,000,000 M-cycles and 10 host seconds per fixture.

| Fixture | Outcome | M-cycles | Base ticks | First explicit host duration, including load |
| --- | --- | ---: | ---: | ---: |
| if_ie_registers | Passed | 4,272 | 17,088 | 3.9235 ms |
| interrupts/ie_push | Passed | 852,459 | 3,409,836 | 194.5601 ms |
| ei_sequence | Passed | 3,943 | 15,772 | 3.8279 ms |
| div_timing | Passed | 4,235 | 16,940 | 3.9059 ms |

Host durations are observations of this parallel debug test run, not performance promises. Final PC is 4AB3 for the assertion-based fixtures and 4878 for ie_push, matching their symbol files' quit@halt labels; SP is E000. The reporter's terminal register signature replaces the body registers after its own assertions, so terminal values are not claimed as raw intermediate measurements.

## Reproduction and regression

Keep the extracted pinned cache at target/test-roms/mooneye, or supply -RomRoot. Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-cpu-subset.ps1
```

The script verifies all four ROM hashes before Cargo runs. To run one fixture, add -Fixture div_timing (or if_ie_registers, ie_push, ei_sequence). Both the four-fixture command and single div_timing selection were executed successfully. The process-only execution-policy option handles this machine's script restrictions without a persistent settings change. Missing fixtures/mismatched hashes fail setup.

The external tests are individually opt-in/ignored in an ordinary fixture-free checkout; ignored results are never external passes. After verified execution, `cargo test --all-features --quiet -- --include-ignored` passes all 430 tests (312 unit, 118 integration), including all five real external fixtures, with no ignored tests in that run. Formatting, strict Clippy and whitespace checks pass. No application/configuration/dependency changes or ROM binaries committed.

## Coverage and limits

#50 is complete. Together with reg_f, these are five narrow independent results for F masking, selected interrupt semantics and DIV timing. General arithmetic/DAA/CB coverage remains primarily local; the ROMs do not certify every instruction's timing, PPU/DMA or physical STOP. [#51 completed M2's functional review](../M2_COMPLETION_REVIEW.md); external DAA #52 remains unrun. Physical STOP assessment is separately tracked by #64, after deterministic integration #44 completed. Original run counts above are historical measurements; current aggregate checks are in the testing guide.
