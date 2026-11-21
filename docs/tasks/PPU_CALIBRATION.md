# PPU calibration plan (#73 slice D)

Proposed 2026-11-20, awaiting agreement. Slices A-C ([LCD registers](LCD_REGISTERS.md)) publish LY/STAT, STAT/VBlank interrupts and VRAM/OAM access restrictions on a provisional model. This plan says which pinned Mooneye PPU ROMs calibrate which provisional decisions, records an exploratory baseline, and splits the work into reviewable steps. [#78](https://github.com/Lorithan/gameboy-emu/issues/78) (OAM corruption, FEA0-FEFF) is outside it.

## Sources

Mooneye revision 31510e12eea6286d36eea060a6adde755e1067aa, `acceptance/ppu`. ROM SHA-256s are of the official prebuilt archive entries (archive SHA-256 18aa2946…d7f3f2, as in the existing manifests); the local ignored copies were checked against the archive on 2026-11-20. No binaries are tracked.

| ROM | Source blob | ROM SHA-256 (prefix) | DMG verified | Measures |
| --- | --- | --- | --- | --- |
| stat_irq_blocking | ed47fa3d | 604436ae | DMG ABC, MGB, CGB, AGB | STAT-line blocking (mode 1 write, LYC sweep) |
| stat_lyc_onoff | 6d8a34fd | 29f04aaf | yes | Held LY=LYC flag across LCD off/on |
| vblank_stat_intr-GS | 317ee7ea | f7de9a3e | DMG, MGB, SGB | VBlank and line-144 mode-2 STAT at the same time |
| intr_1_2_timing-GS | 00bc583b | 3bac47fc | DMG, MGB, SGB | Mode 1 STAT interrupt to mode 2 STAT interrupt |
| intr_2_0_timing | e3ea136b | 6ea58d69 | all | Mode 2 STAT interrupt to mode 0 STAT interrupt |
| intr_2_mode0_timing | bdbe4cf3 | be1555d5 | all | Mode 2 STAT interrupt to STAT reading mode 0 |
| intr_2_mode3_timing | 33318150 | b5cb7d22 | all | Mode 2 STAT interrupt to STAT reading mode 3 |
| intr_2_oam_ok_timing | ae468622 | 38d7acfd | all | Mode 2 STAT interrupt to readable OAM |
| lcdon_timing-GS | 3bcf2b2c | 2a9d46b6 | DMG, MGB, SGB | LY, STAT, OAM/VRAM readability at fixed M-cycles after enable |
| lcdon_write_timing-GS | 4ba40c7a | e28b34ce | DMG, MGB, SGB | OAM/VRAM write acceptance at fixed M-cycles after enable |
| hblank_ly_scx_timing-GS | a27c8fba | 3adec917 | DMG, MGB, SGB | Mode 0 interrupt to LY increment, by SCX |
| intr_2_mode0_timing_sprites | 39b646a9 | 52b10bb0 | all | Mode 2 to mode 0 with sprites |

All use the common `quit` reporter. Failure paths report the all-42 packet. Each still needs the standard per-fixture audit (no earlier SB reports, prerequisites, caps) before it becomes a gated fixture.

## Exploratory baseline (not fixture evidence)

Run 2026-11-20 at be09c94 through the existing runner from an uncommitted, deleted probe. Measured registers come from the writes to Mooneye's `hram.regs_save` (FF80-FF87) and `hram.regs_assert` (FF89-FF90), per each ROM's `.sym` file.

| ROM | Outcome | M-cycles | Measured vs expected |
| --- | --- | ---: | --- |
| stat_irq_blocking | Passed | 197,274 | — |
| stat_lyc_onoff | Passed | 245,527 | — |
| vblank_stat_intr-GS | Passed | 323,803 | — |
| intr_1_2_timing-GS | Failed | 236,114 | D 14 = 14; E 14, expected 15 |
| intr_2_0_timing | Failed | 218,560 | D 07 = 07; E 07, expected 08 |
| intr_2_mode0_timing | Failed | 218,553 | D 01 = 01; E 01, expected 02 |
| intr_2_mode3_timing | Failed | 218,551 | D 01 = 01; E 01, expected 02 |
| intr_2_oam_ok_timing | Failed | 218,553 | D 01 = 01; E 01, expected 02 |
| lcdon_timing-GS | Failed | 330,712 | Table mismatch; needs a dedicated diagnostic |
| lcdon_write_timing-GS | Failed | 629,625 | Table mismatch; needs a dedicated diagnostic |
| hblank_ly_scx_timing-GS | Failed | 304,326 | Expected: SCX-dependent mode 3 is rendering |
| intr_2_mode0_timing_sprites | Failed | 361,685 | Expected: sprite-dependent mode 3 is rendering |

Every count is below one silent LY-wait timeout (851,968 M-cycles), so no wait timed out.

**Shared pattern.** Each `intr_*` test measures with delay N and N-1 around a boundary, and every one gets the first value right and misses the second by one. All five start from `ei; halt` woken by a STAT interrupt. So most likely one phase offset is shared rather than five independent mode-length errors. The candidates, still to be separated:

1. When the STAT line rises relative to the mode change (PPU).
2. HALT-wake / interrupt-dispatch latency (CPU, M3 scope, which would reopen M3 evidence).
3. Where CPU reads sample within an M-cycle relative to PPU dots (the Bus schedule).

Each candidate predicts different results for intr_1_2 (which ends on an interrupt) versus the mode0/mode3/oam_ok tests (which end on a polled read), and for the already-passing vblank_stat_intr-GS. Working that out comes before any constant changes.

## Grouping

- **Regression fixtures now (3):** stat_irq_blocking, stat_lyc_onoff, vblank_stat_intr-GS, after the audit. They protect slice B.
- **In-scope calibration (7):** the five `intr_*` tests, lcdon_timing-GS and lcdon_write_timing-GS. All seven must pass for the #73 gate.
- **Deferred to rendering (2):** hblank_ly_scx_timing-GS and intr_2_mode0_timing_sprites need variable mode-3 length. They are listed in the M4 rendering gate, not #73.
- **No source among these ROMs:** the line-153 visible-LY window and the startup seed phase. They stay documented as provisional unless an audited source is found, and are named as limits in the #73 gate.

## Steps

1. **D1, fixture wiring (assistant).** Audit all twelve, add a manifest, a checksum-gated script and gated pass tests for the three passing ROMs. Add a diagnostic probe for the other nine that reports outcome, M-cycles and measured vs expected registers without asserting a pass, so the remaining calibration work is measurable and no failing test sits in the suite.
2. **D2, the shared `intr_*` offset.** The assistant works out what each candidate predicts per ROM, picks the one consistent with all seven in-scope ROMs plus the three passing ones, and proposes a contract change with local tests. The user implements; the target is the five `intr_*` passing with no regressions among the 18 + 3 fixtures. If the answer is CPU-side (candidate 2), the M3 evidence is rerun and recorded.
3. **D3, enable timing.** Decode lcdon_timing-GS and lcdon_write_timing-GS sample by sample against our model. Calibrate the first-line timing (Mooneye: "late by 2 T-cycles"), the enable phase, and the earlier OAM write blocking near the mode 2 edge. This may need a dot-level offset that cannot be expressed at M-cycle granularity. Propose its representation before implementing.
4. **D4, gate review.** All ten in-scope ROMs (3 + 7) pass with recorded counts; the deferred two and the unsourced provisional items are listed as M4 limits.

## Decisions to agree

1. The grouping above: three regression fixtures, seven calibration targets, two deferred to rendering, two provisional items with no source.
2. Known-failing ROMs go into a non-asserting diagnostic probe, not into ignored tests that fail.
3. Order: D1, then D2 (shared offset) before D3, because the offset probably affects the lcdon tables too.
