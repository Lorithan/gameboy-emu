# PPU calibration plan (#73 slice D)

Agreed 2026-11-20 (grouping, non-asserting probe, D2 before D3). D1 and D2 are complete: D2 found and fixed a CPU-side cause ([#79](https://github.com/Lorithan/gameboy-emu/issues/79)), and the five `intr_*` ROMs now pass as regression fixtures. **D3 contract drafted below, awaiting agreement.** Slices A-C ([LCD registers](LCD_REGISTERS.md)) publish LY/STAT, STAT/VBlank interrupts and VRAM/OAM access restrictions on a provisional model. This plan says which pinned Mooneye PPU ROMs calibrate which provisional decisions, records an exploratory baseline, and splits the work into reviewable steps. [#78](https://github.com/Lorithan/gameboy-emu/issues/78) (OAM corruption, FEA0-FEFF) is outside it.

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

- **Regression fixtures (8):** stat_irq_blocking, stat_lyc_onoff, vblank_stat_intr-GS (slice B), plus the five `intr_*` since #79.
- **In-scope calibration (2 remaining):** lcdon_timing-GS and lcdon_write_timing-GS (D3). Both must pass for the #73 gate.
- **Deferred to rendering (2):** hblank_ly_scx_timing-GS and intr_2_mode0_timing_sprites need variable mode-3 length. They are listed in the M4 rendering gate, not #73.
- **No source among these ROMs:** the line-153 visible-LY window and the startup seed phase. They stay documented as provisional unless an audited source is found, and are named as limits in the #73 gate.

## Steps

1. **D1, fixture wiring (assistant).** Audit all twelve, add a manifest, a checksum-gated script and gated pass tests for the three passing ROMs. Add a diagnostic probe for the other nine that reports outcome, M-cycles and measured vs expected registers without asserting a pass, so the remaining calibration work is measurable and no failing test sits in the suite.
2. **D2, the shared `intr_*` offset.** The assistant works out what each candidate predicts per ROM, picks the one consistent with all seven in-scope ROMs plus the three passing ones, and proposes a contract change with local tests. The user implements; the target is the five `intr_*` passing with no regressions among the 18 + 3 fixtures. If the answer is CPU-side (candidate 2), the M3 evidence is rerun and recorded.
3. **D3, enable timing.** Decode lcdon_timing-GS and lcdon_write_timing-GS sample by sample against our model. Calibrate the first-line timing (Mooneye: "late by 2 T-cycles"), the enable phase, and the earlier OAM write blocking near the mode 2 edge. This may need a dot-level offset that cannot be expressed at M-cycle granularity. Propose its representation before implementing.
4. **D4, gate review.** All ten in-scope ROMs (3 + 7) pass with recorded counts; the deferred two and the unsourced provisional items are listed as M4 limits.

## Agreed decisions

1. The grouping above: three regression fixtures, seven calibration targets, two deferred to rendering, two provisional items with no source.
2. Known-failing ROMs go into a non-asserting diagnostic probe, not into ignored tests that fail.
3. Order: D1, then D2 (shared offset) before D3, because the offset probably affects the lcdon tables too.

## D1 status (complete)

[tests/fixtures/mooneye-ppu.json](../../tests/fixtures/mooneye-ppu.json) records all twelve ROM and symbol hashes, blobs, groups and the audit (no SB/SC writes, no DMA, common quit reporter, VRAM/OAM setup with the LCD off). [run-mooneye-ppu.ps1](../../tests/scripts/run-mooneye-ppu.ps1) verifies all 24 hashes before Cargo, then runs `mooneye_ppu_regression_*` (three gated passes) and `mooneye_ppu_calibration_probe`. The probe never asserts a pass. It does assert a complete Passed/ReportedFailure report below one silent timeout, and prints the measured vs expected `setup_assertions` registers. Those columns are meaningful only for ROMs using `setup_assertions` (the five `intr_*`); lcdon and the rendering ROMs use that HRAM differently.

## D2 finding: HALT wake with IME=1 is one M-cycle late (#79)

The interval analysis rules out the PPU. Each `intr_*` loop period P (4 M-cycles for the interrupt-ended loops, 8 for the polled ones) bounds our error: our wake-to-end interval is 1 to P-1 M-cycles shorter than hardware. intr_1_2 runs from one STAT interrupt to another, so no STAT-raise shift can shorten it. The shared start, `ei; halt` woken by STAT, is the cause.

Exploratory runs of pinned Mooneye CPU ROMs (hash-checked against the archive, not yet manifest fixtures) separate the candidates:

| ROM | Isolates | Result |
| --- | --- | --- |
| intr_timing | 5-cycle dispatch while running | Passed |
| halt_ime0_nointr_timing | VBlank interval, HALT with IME=0 | Passed |
| halt_ime1_timing2-GS | Same interval, HALT with IME=1 | Failed: D 11 = 11; E 11, expected 12 |
| di_timing-GS | HALT IME=1 VBlank wake, then delay to the next VBlank | Failed |
| halt_ime1_timing, halt_ime0_ei, ei_timing, rapid_di_ei, reti_intr_timing | Other HALT/EI/RETI rules | Passed |

The frame period and ordinary dispatch are right; only HALT with IME=1 plus dispatch is late. Cause: `System::step_machine_cycle` wakes a halted CPU only before the device advance, so a request raised during the advance waits for the next step, whereas a running CPU accepts it after the advance in the same step. A public-API probe with one Timer request measured entry at steps (27, 28) after HALT vs (26, 27) after NOPs. A one-cycle shift maps every observed `intr_*` value onto its expected value, so the prediction is that all five pass after the fix, with no PPU constant changed.

Contract for #79 (corrected 2026-11-20): a request that becomes eligible during a halted M-cycle's advance wakes the CPU in that same step regardless of IME. With IME=1 that step starts interrupt entry, as for a running CPU; with IME=0 its bus access is the next opcode fetch. The first draft said IME=0 was unchanged. That was wrong: halt_ime0_nointr_timing passed only because its IME=1 sync wake and IME=0 exit were each one M-cycle late and cancelled out. Fixing IME=1 alone made it measure D 12 (expected 11). Acceptance tests (assistant): `halt_with_ime_services_a_device_raised_request_like_a_nop_wait` and `halt_without_ime_fetches_in_the_same_step_as_a_device_raised_request` in `system::tests::halt`. After the fix, propose adding the HALT/EI/DI ROMs above to the CPU fixture manifest.

**D2 result, 2026-11-20.** The user's fix wakes the CPU after the device advance as well and refreshes the action when the wake or acceptance changes state. Results:

- All five `intr_*` ROMs pass with no PPU constant changed: intr_1_2_timing-GS 235,870 M-cycles; intr_2_0, intr_2_mode0, intr_2_mode3 and intr_2_oam_ok 218,316 each.
- All nine CPU interrupt ROMs pass.
- The 18 earlier fixture counts are unchanged.
- The full suite has 559 passing tests: 403 unit, 156 integration, 35 gated external fixtures and the probe.

Still in the probe: lcdon_timing-GS and lcdon_write_timing-GS (D3), plus the two rendering ROMs.

## D3: LCD-enable timing (proposed contract)

Drafted 2026-11-20, awaiting agreement.

### Decoding

Each sample in both ROMs is `ldh (LCDC),a` (enable), N NOPs, then a 2-M-cycle `ld a,(de)` or `ld (de),a`. Our enable store applies after its own M-cycle's ticks and the access happens after its cycle's ticks, so the access sees **dot 4N + 8** of the enabled timing. The local test `memory::ppu::tests::lcdon` (both Mooneye tables, 158 cells) reproduces this through Bus. Against the current model **10 cells fail**:

| Line dots | Lines sampled | Observer | Hardware | Current model |
| --- | --- | --- | --- | --- |
| 452-455 | end of lines 0, 1 | LY | next line (01, 02) | current line |
| 452-455 | end of lines 0, 1 | LY=LYC flag | 0 for LYC 0 and LYC 1 | compares the current line |
| 452-455 | end of lines 0, 1 | OAM read | blocked (FF) | accessible |
| 76-79 | lines 1, 2 | VRAM read | blocked (FF) | accessible |
| 76-79 | lines 1, 2 | OAM write | lands | blocked |

The other 148 cells match, including every STAT mode edge (mode 2 to 3 at 80, mode 3 to 0 at 252, line start at 0), all of the first line after enable, OAM writes and VRAM writes at 452-455 (both land), and VRAM writes at 76-79 (land). That confirms the 4N + 8 sampling phase.

**No sub-M-cycle state is needed.** Lines (456 dots) and frames (70,224) are multiples of 4, and both enable (dot 0) and the post-boot seed (dot 4) are 4-aligned. So CPU accesses only ever observe dots that are multiples of 4. Mooneye's "late by 2 T-cycles" first line is invisible on that grid: on line 0 the VRAM-read lock and mode 3 both fall in dots 77-80, which our dot 80 already satisfies.

### Proposed rules (provisional, 4-dot resolution)

A **normal line** is a visible line (0-143) that is not the first line after enable. Its STAT mode is unchanged: 2 for dots 0-79, 3 for 80-251, 0 for 252-455.

1. **Line-end window, dots 452-455 of lines 0-142:**
   - LY reads the next line;
   - the LY=LYC flag is 0, also for LYC writes in that window;
   - OAM reads are blocked;
   - STAT still reads mode 0, and OAM writes and VRAM still pass.
2. **Mode 2 tail, dots 76-79 of normal lines:**
   - VRAM reads are blocked;
   - OAM writes pass;
   - STAT still reads mode 2, OAM reads stay blocked and VRAM writes pass.
3. **Everything else is unchanged.** VRAM is blocked in Drawing for both reads and writes. OAM is blocked in OamScan and Drawing except for point 2's write gap. The first line after enable keeps its current behaviour.

The resulting locks, as half-open dot ranges within a line:

| Lock | Normal line | First line after enable |
| --- | --- | --- |
| OAM read | 0-251, plus 452-455 of the previous visible line | 80-251 |
| OAM write | 0-75 and 80-251 | 80-251 |
| VRAM read | 76-251 | 80-251 |
| VRAM write | 80-251 | 80-251 |

### Scope limits (decisions)

- **Covered by the evidence:** the ends of lines 0 and 1 and lines 1-2, generalised to every visible-line transition (line ends 0-142), since Mooneye says lines 1 and 2 have "normal timings".
- **Not covered, proposed unchanged:**
  - the line-end window at the ends of lines 143-153;
  - the line-153 LY quirk;
  - LYC=153 matching;
  - OAM read locking before line 0 of the next frame.
  If the LY register really updates 4 dots before each line, the line-153 window may also belong 4 dots earlier. No pinned ROM here settles that, and moving it would change LYC=153 matching, so it stays a documented provisional item for a later source.
- **The STAT interrupt line** keeps using the STAT-visible mode and the flag. One consequence of rule 1: with only the LYC and OAM selects set, a flag that matched the old line now drops at dot 452 rather than dot 0, so the next line's mode-2 source can raise a new edge. That is a genuine behaviour change; the slice B model tests will pin it.

### Proposed types and ownership

- **`PpuTiming` keeps owning "where in the frame"** and answers per-observer questions from line, dot and first-line state. Rough shapes, your choice:
  - `visible_ly()` gains the line-end window;
  - a comparison view returning `Option<u8>`, which is `None` in the line-end window. Ppu sets the flag to "Some and equal to LYC" after each tick and on LYC writes;
  - a small `Copy` value or four queries describing the CPU access locks (OAM read, OAM write, VRAM read, VRAM write).
- **`mode()` stays the STAT-visible mode.** The locks stop being "a function of mode" and become their own table, which is exactly what the hardware shows.
- **Ppu** combines the locks with LCDC enable, as now. Bus is unchanged.
- **Rejected alternative:** a second "internal line" counter offset by 4 dots. It is more stored state that could drift, and only 4-dot windows are observable anyway.

### Acceptance

- `memory::ppu::tests::lcdon`: both tables, all 158 cells (assistant; written, currently 10 failures).
- lcdon_timing-GS and lcdon_write_timing-GS pass and move from the probe to the PPU regression group.
- Slice A-C model tests are updated to the new windows: status, interrupts and access, with the shared position model extended. Every change is listed.
- All 35 gated fixtures still pass with counts recorded. LY waits can now end one M-cycle earlier, so small count changes are expected and each is explained.
- Full suite, fmt and strict Clippy pass.
