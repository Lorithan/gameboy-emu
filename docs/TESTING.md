# Testing and ROM evidence

Latest fixture-inclusive regression, #89 frontend F5 on 2026-11-26: **657 passed, zero failed, zero ignored**, with `--include-ignored` across the workspace (613 core, 44 frontend), including 43 hash-verified gated external fixtures (dmg-acid2 among them), the demo ROM smoke tests and the PPU calibration probe. Formatting and workspace-wide strict Clippy pass. The core is unchanged since the M4 checkpoint at 7b10751, where all eight checksum-gated fixture scripts passed.

## Frontend F5: opening a ROM from the window (#89)

Contract and results: [frontend](tasks/FRONTEND.md#f5-result-complete-2026-11-26). On 2026-11-26, `cargo test --workspace --all-features --no-fail-fast -- --include-ignored` passes **657, zero failed, zero ignored** (613 core, 44 frontend). fmt and workspace-wide strict Clippy pass. The core is unchanged by F5.

- **Layout tests (19),** `frontends/sdl/src/layout/tests.rs`: text size by characters, hit-test edges, the text scale, word wrapping (with a random test of 2,000 texts), and the picker layout: centring, stack order and gap, a button that never moves, fitting windows down to 160×144, text centred at each label's scale, and long errors cut to fit.
- **Validation:** checked against scratch references with SDL stubbed out; each of 27 deliberate breaks failed at least one test. During review the tests caught one wrapping bug and three layout bugs.
- **Manual checklist:** all items pass, run by the user (dialog, errors, drag-and-drop, switching games, Explorer) and by the assistant (title, Esc, the argument, GUI subsystem, CPU use, 59.60 frames per second over 20 s).
- **What this proves:** the picker's geometry and text handling, and that the window flows work by hand. The event loop, dialog and screen changes have no automated tests.

## Frontend F3 and F4: input and demo ROMs (#87, #88)

Manifest: [demo-roms.json](../tests/fixtures/demo-roms.json). On 2026-11-23, `cargo test --workspace -- --include-ignored` passes **638, zero failed** (613 core, 25 frontend). fmt and workspace-wide strict Clippy pass.

- **Selection:** Homebrew Hub's 657 Game Boy entries were filtered to the 43 with a declared licence, and the permissive ones' headers were read. Of the four that are ROM-only, MaxPirate v1.0 (MIT, official release asset) and GB Snake (WTFPL, pinned commit) are pinned. The other two were rejected: one has no licence file in its repository, and the other's repository was deleted.
- **Pinning:** `run-demo-roms.ps1` checks the SHA-256 of both ROMs and both licence files before running Cargo. Nothing is committed: the files stay under `target/test-roms`.
- **Smoke tests (2),** `tests/demo_roms.rs`: each ROM runs headless like the frontend, with no core error or STOP. After button presses, Snake goes from 1 to 159 distinct pictures and MaxPirate from 3 to 20. With the presses disabled both fail, so the check depends on input reaching the game.
- **APU:** neither game hangs without sound hardware. MaxPirate's hUGEDriver music runs against the unimplemented sound registers.
- **Manual play-through (2026-11-23):** the user played both games in the release frontend with no problems, and the F3 key checklist passed ([results](tasks/FRONTEND.md#f3-and-f4-result-complete-2026-11-23)).
- **What this proves:** both games boot, draw, keep running and react to buttons through the core. It does not show that they play correctly; the play-through checks that.

## Frontend F2: SDL3 window and timer pacing (#86)

Contract and checklist results: [frontend](tasks/FRONTEND.md#f2-result-complete-2026-11-22). On 2026-11-22, `cargo test --workspace -- --include-ignored` passes **626, zero failed** (611 core, 15 frontend), and fmt and workspace-wide strict Clippy pass.

- **Frontend unit tests (15),** in `frontends/sdl/src/display/tests.rs` (8) and `pacing/tests.rs` (7), as described in the contract.
- **Before implementation:** against stubs in a scratch worktree 14 failed. A throwaway reference passed all 15, and each of 16 deliberate breaks failed at least one. Two of the tests' own expected values were corrected against the reference first.
- **During implementation,** the tests caught a row divisor of `HEIGHT` and an off-centre viewport in `display`, and a first frame one period late, a count that never advanced, the frame index returned as the count, and a cap that could never resynchronise in `Pacer`.
- **Manual checklist:** all eight items pass: usage and errors, dmg-acid2 shown, resizing, closing, 59.73 frames per second measured over 60 s, 20% of one core, and a statically linked release build.
- **What this proves:** the frontend shows the core's frames correctly scaled and at the right long-run rate. Frame-to-frame timing jitter and input are not covered yet.

## Frontend F1: workspace split (#85)

Contract: [frontend](tasks/FRONTEND.md). The root package stays the core library (`src/main.rs` removed); `[workspace]` adds `frontends/sdl` (`gameboy-sdl`, a placeholder binary depending on the core by path). On 2026-11-22, `cargo build --workspace` passes, `cargo fmt --all --check` and the workspace-wide strict Clippy pass, and `cargo test --workspace -- --include-ignored` passes **611, zero failed** (446 unit, 165 integration; the frontend has no tests yet). The core still has no dependencies. All seven fixture scripts pass unchanged from the root. Every per-ROM M-cycle count, and dmg-acid2's breakpoint PC, M-cycles and frame count, is identical to `b446039`, compared ROM by ROM (43 fixtures). Cargo warns that `gameboy-sdl` does not use `gameboy-emu` yet; that warning goes away in F2.

## Rendering R4: dmg-acid2 (#84)

Contract: [rendering](tasks/RENDERING.md); provenance in [dmg-acid2.json](../tests/fixtures/dmg-acid2.json). Latest fixture-inclusive regression, 2026-11-22: **611 passed, zero failed** (446 unit, 165 integration), including 43 hash-verified gated fixtures and the PPU calibration probe. Formatting and strict Clippy pass; all seven fixture scripts pass.

- **Fixture:** dmg-acid2 v1.0 (MIT), the author's release ROM, with `img/reference-dmg.png` and LICENSE from the tag commit `dc22954`. The image and LICENSE match that commit's git blobs. Nothing is committed: the files stay under `target/test-roms/dmg-acid2`.
- **Comparison without a dependency:** `run-dmg-acid2.ps1` checks the ROM and PNG hashes, decodes the PNG with .NET `System.Drawing` (160×144, exactly the four greys FF/AA/55/00), and writes `reference-dmg.shades`: 144 lines of 160 digits, 0 White to 3 Black. The digit counts equal the PNG's colour counts, and the file's hash is pinned too. `tests/dmg_acid2.rs` reads it.
- **Completion:** the ROM executes `ld b, b` from its LY=0x90 handler after counting ten frames (source at the tag). The test stops with PC at that opcode, 02D7. The bytes there match the handler's `dec b; jr nz; ld b, b`. It then compares `System::frame()` pixel by pixel; on failure it reports the count, first mismatch and rows, and writes `actual-dmg.shades`.
- **Result:** all 23,040 pixels match, at 324,819 M-cycles with 12 frames completed.
- **Not vacuous:**
  - a one-pixel change to the reference fails with exactly that pixel;
  - in a scratch worktree, renderer bugs fail by these pixel counts:

    | Bug | Pixels wrong |
    | --- | --- |
    | window row from LY − WY | 818 |
    | no 10-object limit | 28 |
    | 8×16 tile bit 0 kept | 256 |
    | OAM-order priority | 12 |
    | no rendering | 10,191 |

  - removing BG-priority masking still passes, because dmg-acid2 never puts a BG-priority object over another object; `bg_priority_hides_the_winner_over_colours_1_to_3_and_masks_lower_objects` covers it.
- **What this proves:** the scanline renderer reproduces a hardware-verified reference frame for every case in dmg-acid2's failure table. Its writes from LY=LYC interrupts during mode 2 land before dot 80 in this model. It is not evidence for mid-mode-3 effects, which dmg-acid2 avoids.

## Rendering R3: objects (#83)

Contract: [rendering](tasks/RENDERING.md), rules 4-6 (checked against Pan Docs OAM, Palettes and LCDC). Latest fixture-inclusive regression after the user's R3 implementation, 2026-11-22: **610 passed, zero failed** (446 unit, 164 integration), including the 42 hash-verified gated fixtures and the PPU calibration probe. Formatting and strict Clippy pass; all six fixture scripts pass.

- **System tests (8),** `system::tests::rendering`, against the independent model extended with objects (BG priority applied to the winning object only): edge positions (Y 0, 1, 2, 8, 9, 15-17, 100, 152, 153, 159, 160, 200; X 0, 1, 4, 7-9, 80, 152, 159-161, 167, 168, 255) in 8x8 and 8x16 and with signed background tile data; every Y flip, X flip and palette combination with odd 8x16 tile numbers and with CGB-only bits 0-3 set; the 10-object limit with off-screen X counting and OAM order (not X) deciding who is dropped; smaller X then lower OAM index winning, with transparent pixels showing through; BG priority over background colours 1-3, masking of lower objects, LCDC bit 0 clear and over the window; dense random OAM (4 seeds, 5 LCDC settings); LCDC bit 1 clear; and LCDC bits 1-2 and OBP0/OBP1 written in HBlank applying from the next line. Both OBPs have non-zero low bits, which must be ignored.
- **Before implementation:** seven of the eight tests failed against the window renderer; the bit-1-clear test passed vacuously, as expected. A throwaway reference in a scratch worktree (never committed) passed all of them, and each of eighteen deliberate breaks failed at least one test: no 10-object limit, X counted for selection, OAM order only, ties to the higher index, no show-through, no masking, 8x16 Y flip per tile, 8x16 tile bit 0 kept, X flip ignored, palette from a CGB bit, colour 0 opaque (twice), BG priority ignored, BG priority always hiding, 8x16 selection height 8, Y and X off by one, LCDC bit 1 ignored.
- **Review:** the first draft did not check LCDC bit 1 in selection; it was fixed before the tests were run.
- **Fixtures:** all 42 gated counts are identical to #82, compared ROM by ROM against a single-threaded run of the previous commit.
- **What this proves:** objects follow the sourced rules at every pixel of the tested configurations, including the masking quirk. Not modelled: mid-line LCDC and OAM effects. dmg-acid2 (#84) is the external check.

## Rendering R2: the window (#82)

Contract: [rendering](tasks/RENDERING.md), rule 3 (checked against Pan Docs Window, Tile Maps and LCDC). Latest fixture-inclusive regression after the user's R2 implementation, 2026-11-22: **602 passed, zero failed** (438 unit, 164 integration), including the 42 hash-verified gated fixtures and the PPU calibration probe. Formatting and strict Clippy pass; all six fixture scripts pass.

- **System tests (10),** `system::tests::rendering`, against the same independent model extended with the window: WX 1, 6, 7, 8, 50, 159, 165, 167, 200 and 255 and WY 1, 77, 143, 144 and 255 under a scroll the window must ignore; all four combinations of LCDC bits 3 and 6 with both tile-data modes; bit 5 clear and bit 0 clear hiding the window; the line counter paused by bit 5 and by WX 200 and resuming on the next row; the Y condition latching (a WY already passed never triggers, a later one does, moving WY afterwards does not hide it); the Y condition set while bit 5 is clear (DMG); the Y condition and counter restarting each frame; LYC/STAT writes mid-frame leaving the window alone; and an LCD restart mid-window starting the next frame fresh.
- **Before implementation:** six of the first eight tests failed against the background-only renderer; the two hiding tests passed vacuously, as expected. A throwaway reference window in a scratch worktree (never committed) passed all of them, and each of eleven deliberate breaks failed at least one test: no latch, counter on every line, no frame reset, background map used, WX-8 offset, no WX ≤ 166 check, window scrolled, latch requiring bit 5, row from LY − WY, bit 5 ignored, window drawn with bit 0 clear.
- **Review:** the first implementation drew a fixed pixel and did not latch the Y condition (six failures), and reset the window state in `write_lyc`, which no test covered. The LYC/STAT test was added for it and fails with that reset restored. The LCD-restart test was added afterwards, when review found that no test covered the reset on LCD off; it fails with that reset removed.
- **Fixtures:** all 42 gated counts are identical to #81, compared ROM by ROM against a single-threaded run of the previous commit.
- **What this proves:** the window follows the sourced rules at every pixel of the tested configurations. Not modelled: the DMG quirks at WX 0 and WX 166, and the colour-0 pixel inserted when bit 5 is clear. dmg-acid2 (#84) is the external check. Objects are not drawn yet.

## Rendering R1: frame publication and background (#81)

Contract: [rendering](tasks/RENDERING.md). Latest fixture-inclusive regression after the user's R1 implementation, 2026-11-22: **592 passed, zero failed** (428 unit, 164 integration), including the 42 hash-verified gated fixtures and the PPU calibration probe. Formatting and strict Clippy pass; all six fixture scripts pass.

- **System tests (11),** `system::tests::rendering`, with the CPU spinning on `JR -2`: startup frame and count; `pixel` bounds; the background against an independent model for both tile-data modes, both maps and five scroll positions (including wrap), five BGP values and LCDC bit 0 clear; `pixel` against `pixels` row-major order; publication in exactly the VBlank-request M-cycle and every 17,556 M-cycles; no change to the published frame between publications (sampled every 61 M-cycles); register writes during OAM scan applying to that line and during Drawing or HBlank from the next (decision 1); LCD disable publishing one White frame with BGP index 0 Black, no publication while off or on enable, and a normal first frame after enable (decision 3).
- **Integration test (1),** `tests/video_api.rs`: `video::Frame`, `video::Shade`, `System::frame` and `System::frames_completed` are public.
- **Before implementation:** against an interface stub, the startup and bounds tests passed and the other eight failed. A throwaway reference renderer in a scratch worktree (never committed) passed all eleven, and each of nine deliberate breaks failed at least one test: render at HBlank, single buffer, no blank on disable, wrong signed base, SCY ignored, map bit ignored, LCDC bit 0 ignored, bit planes swapped, no SCX wrap.
- **Fixtures:** all 42 gated counts are identical to #80, checked from a single-threaded run. Rendering changes no timing.
- **What this proves:** the background follows the agreed rules at every pixel of the tested configurations, frames are published only whole and at VBlank, and the scanline timing choice behaves as agreed. It is not hardware evidence; dmg-acid2 (#84) is the external check. The window and objects are not drawn yet.

## OAM DMA (#80)

Contract and decisions: [OAM DMA](tasks/OAM_DMA.md). Latest fixture-inclusive regression after the user's #80 implementation, 2026-11-21: **580 passed, zero failed** (417 unit, 163 integration), including 42 hash-verified gated external fixtures and the PPU calibration probe. Formatting and strict Clippy pass; all six checksum-gated fixture scripts pass.

- **Bus tests (8),** `memory::tests::oam_dma`: FF46 startup value and readback at every M-cycle of a transfer; OAM readable at M1, FF from M2 to M161 and holding the source at M162; all 160 bytes from every one of the 256 source pages, against an independent model of rule 4 (ROM, VRAM, absent external RAM, WRAM, E0-FF minus 0x2000); restarts written at M2, 3, 4, 7, 8, 80, 159, 160 and 161; a second write before the start; CPU OAM writes ignored while active; the copy landing during OamScan and Drawing with the LCD on, from a VRAM source; and no other memory, IF or timer change against a reference Bus that never started a transfer (LCD on and off).
- **System tests (4),** `system::tests::oam_dma`: an oam_dma_timing reproduction run from ROM, instruction-budget pauses, a transfer continuing across HALT, and an HRAM routine like Mooneye's `dma_proc`.
- **Migrated tests (4):** the LCD gap test, the unimplemented-address test, the System LDH store test and the integration LDH round-trip test now treat FF46 as mapped. The unimplemented-address and integration LDH tests were missed when the DMA tests were written; they failed against the implementation and were then updated.
- **Before implementation:** 11 of the 12 new tests and the System LDH store test failed at their first DMA assertion, and all five ROMs reported failure. The no-conflict test passed vacuously, as expected with no transfer. During implementation, review caught a transfer ending one M-cycle early, the last byte not being copied and the CPU read path being used for the source (VRAM lock, pages FE/FF); the LCD-on test then caught the remaining VRAM lock on the DMA path. One test-model bug was fixed: the ROM pattern omitted the header checksum byte at 014D.
- **Fixtures (5),** [manifest](../tests/fixtures/mooneye-oam-dma.json) and `run-mooneye-oam-dma.ps1`:

| Fixture | M-cycles |
| --- | ---: |
| oam_dma/basic | 185,071 |
| oam_dma/reg_read | 181,355 |
| oam_dma_timing | 235,872 |
| oam_dma_restart | 235,871 |
| oam_dma_start | 257,248 |

  All are well below one silent 851,968 M-cycle LY-wait timeout. The other 37 gated counts are identical to a clean run of `834f631`.
- **What this proves:** the DMG start delay, 160-M-cycle blocking window, restart behaviour and FF46 readback that the five ROMs measure, and the source mapping reproduced from sources-GS. It does not prove bus-conflict behaviour (not modelled, decision 1), the FF46 startup value (no pinned ROM), or the restart-at-M161 and two-writes-within-two-M-cycles cases, which are tested locally against the derived rules only; those remain provisional.

## LCD-enable timing (#73 D3)

Contract and full result: [PPU calibration, D3](tasks/PPU_CALIBRATION.md#d3-result-complete-2026-11-21).

- **New unit tests (2),** `memory::ppu::tests::lcdon`: both DMG-verified Mooneye expectation tables (lcdon_timing-GS reads: LY, STAT with LYC 0 and 1, OAM, VRAM; lcdon_write_timing-GS: OAM and VRAM write acceptance), 158 cells, reproduced through Bus at dot 4N + 8 after enable.
- **Updated tests (5):** the shared position model (`memory::ppu::tests::model`) gained the line-end window, a comparison view and four separate locks; `status`, `timing`, `interrupts` and `access` use it, and `access` now also requires cases where the read and write locks differ in each region. `system::tests::ppu_access` derives expected values from the position and a shadow of the stored bytes instead of the STAT mode. Both `stat_lyc_onoff` reproductions step 2 M-cycles past the first LY sample, since that sample can fall in the window where the flag reads 0. All seven failed against the pre-D3 code only at a D3 window, and pass after it.
- **Fixtures:** lcdon_timing-GS (529,375 M-cycles) and lcdon_write_timing-GS (977,240) moved from the probe to the PPU regression group; ten PPU regressions in total. lcdon_write_timing-GS exceeds one silent LY-wait timeout by design (43 waits); a one-off trace confirmed its timeout label is never fetched. It runs with its own 1,200,000 M-cycle cap (the shared cap left only 22,760 of margin); one silent timeout still exceeds it, and the shared 1,000,000 cap is unchanged for every other fixture. The other 35 gated counts are identical to #79.
- **What this proves:** the access locks, LY and LY=LYC flag match DMG hardware at every M-cycle sampled by the two lcdon ROMs (the first three lines after enable). The model tests show the generalised rule is implemented consistently at every M-cycle of two frames; they encode the agreed rule and are not hardware evidence for other lines. The ends of lines 143-153, the line-153 window and VBlank-line transitions are not covered by any pinned ROM and remain provisional.

## HALT wake timing (#79)

Latest fixture-inclusive regression after the user's #79 fix: **559 passed, zero failed** (403 unit, 156 integration). That covers 35 hash-verified gated external fixtures plus the PPU calibration probe; fmt and strict Clippy pass.

- **System tests (2), in `system::tests::halt`:** `halt_with_ime_services_a_device_raised_request_like_a_nop_wait` checks that, for a Timer request raised mid-advance, a HALT wait selects and completes entry at exactly the same steps as a NOP wait (it was 27/28 vs 26/27 before the fix). `halt_without_ime_fetches_in_the_same_step_as_a_device_raised_request` checks that with IME=0 the next opcode fetch is the bus access of the request's own M-cycle, `INC B` executes once and the request stays pending. The first draft of the second test encoded the wrong rule (IME=0 unchanged); the corrected version failed against the IME=1-only fix with an Idle action where a fetch was due.
- **Interrupt fixtures (9),** [manifest](../tests/fixtures/mooneye-cpu-interrupts.json) and `run-mooneye-cpu-interrupts.ps1`:

| Fixture | M-cycles | Before #79 |
| --- | ---: | --- |
| halt_ime1_timing2-GS | 323,804 | Failed (E 11, expected 12) |
| di_timing-GS | 267,529 | Failed |
| halt_ime0_nointr_timing | 253,424 | Passed only through cancelling errors |
| halt_ime1_timing | 183,119 | Passed |
| halt_ime0_ei | 179,757 | Passed |
| intr_timing | 183,198 | Passed |
| ei_timing | 183,197 | Passed |
| rapid_di_ei | 183,362 | Passed |
| reti_intr_timing | 183,273 | Passed |

- **PPU fixtures:** eight gated regressions. stat_irq_blocking 197,274; stat_lyc_onoff 245,527; vblank_stat_intr-GS 323,802; intr_1_2_timing-GS 235,870; the four other `intr_*` 218,316 each. The probe now covers lcdon_timing-GS, lcdon_write_timing-GS and the two rendering ROMs.
- **Unchanged counts:** the 18 earlier fixtures (reg_f, four CPU subset, thirteen timer) are identical to #77; none wakes from HALT on a device request.

## PPU fixtures and calibration probe (#73 D1)

The pinned Mooneye PPU set is wired through [mooneye-ppu.json](../tests/fixtures/mooneye-ppu.json) and [run-mooneye-ppu.ps1](../tests/scripts/run-mooneye-ppu.ps1), which hash-checks all twelve ROMs and their symbols before Cargo:

- **Gated passes (3):** stat_irq_blocking 197,274 M-cycles, stat_lyc_onoff 245,527, vblank_stat_intr-GS 323,803.
- **Calibration probe (9, non-asserting):** reports outcome, M-cycles and measured vs expected registers. It still asserts complete reports below one silent LY-wait timeout. All nine currently report failure; see the [calibration plan](tasks/PPU_CALIBRATION.md) for values and the D2 HALT finding ([#79](https://github.com/Lorithan/gameboy-emu/issues/79)).

## VRAM/OAM access restrictions (#77)

Six new tests verify the [slice C contract](DEVICES.md#vram-and-oam-access-restrictions):

- `memory::ppu::tests::access` (4), against the shared independent position model (`memory::ppu::tests::model`, extracted from the #76 tests unchanged):
  - at every M-cycle for two frames, from the seed and from enable, reads and read-after-write at ten VRAM/OAM boundary and interior addresses match the model (FF when blocked); blocked and allowed accesses both occur in each region; the first enabled line allows OAM at exactly its 19 sampled HBlank M-cycles; LY/STAT/IF/timing equal an access-free reference Bus; with the LCD off afterwards every stored byte matches, so no blocked write landed;
  - LCD off: every access round-trips for a frame, and disabling in Drawing unblocks the very next access;
  - FEA0-FEFF reads FF and ignores writes in all four modes and with the LCD off, without aliasing OAM;
  - writing every VRAM/OAM byte in Drawing changes no visible memory or state, and the hidden storage equals a reference Bus once the LCD is off.
- `system::tests::ppu_access` (2): a program using `LD (HL),A`, `LD A,(HL)`, `LD (a16),A`, `LD A,(a16)`, `PUSH` and `POP` in VRAM and OAM runs three frames. Every VRAM/OAM read and store is checked against STAT's mode on a lockstep reference Bus at that M-cycle (FF and no store when blocked), with blocked and allowed reads and writes in both regions; hidden storage matches afterwards. Instruction-budget pauses give identical CPU state and storage.

Migration: `divider_bus_resets_phase_and_preserves_every_other_address` now disables the LCD before its final all-address comparison, since OAM is blocked at that point. No mutation check was run.

**Fixtures.** All eighteen hash-verified fixtures pass with M-cycle counts identical to #76 (16 at 183,194-183,522, ie_push 179,867, div_write 899,476), consistent with the audit that the reporter writes VRAM only with the LCD off. Tests verify the provisional model, not hardware timing.

## STAT and VBlank interrupts (#76)

Thirteen new tests verify the [slice B contract](DEVICES.md#stat-and-vblank-interrupts):

- `memory::ppu::tests::interrupts` (10), against an independent position/line/edge model that shares no production code:
  - `Ppu::tick` effects at every tick for two frames, from the seed and from enable, for all 16 select combinations × LYC 0, 1, 143, 144, 153, FF (with a non-vacuity check on request counts);
  - hand-derived per-frame counts: HBlank 144, OAM 145, VBlank 1, LYC 1 (0 for FF), HBlank+VBlank 144, OAM+VBlank 144;
  - the VBlank effect exactly once per frame at line 144 / dot 0 for every select combination;
  - Bus IF at every M-cycle for two frames matching the model's OR of four ticks' effects;
  - no request at construction or before the first VBlank;
  - LYC writes while on: immediate flag, request only on a rising edge, no request without the select;
  - the STAT-write quirk in OAM scan / HBlank / VBlank / with the flag set (one request per write), not in Drawing with the flag clear, and not when the line is already high;
  - stat_irq_blocking rounds 1 and 2 reproduced through Bus by M-cycle polling, asserting that mode 3 was passed on every LYC 0-143 iteration;
  - no requests while the LCD is off for every STAT/LYC byte, and none at either LCDC edge;
  - the four stat_lyc_onoff rounds through IF bit 1 (request only for held false then true).
- `system::tests::ppu_interrupts` (3): VBlank and STAT HBlank requests wake HALT at M-cycles 16,529 and 176 from the seed and dispatch to 0040 / 0048 with the same request-to-entry delay as a Timer control; a same-tick VBlank and line-144 OAM pulse dispatch 0040 then 0048; with IME off only IF changes; a STAT-write request is dispatched at exactly the same step as a direct IF store at the same program position.

Migrations: the slice A every-M-cycle readback test and the System LDH-read test no longer assert "no PPU interrupts"; the System test now compares IF with its reference Bus. Two snapshot tests (passive LCD all-byte and LDH store mapping) now expect the STAT flag to follow LYC writes immediately (STAT 85 → 81 after LYC 5A). One hand count (OAM+VBlank) was wrong in the first draft, 145 rather than 144, because line 0's OAM is blocked behind VBlank; the independent model and production agreed. No mutation check was run; detection of the review-found defects (STAT effect clearing instead of requesting IF; LCD-off writes evaluating the flag/line) rests on the Bus IF model, off-write and stat_lyc_onoff tests.

**Fixtures.** All eighteen hash-verified fixtures pass with M-cycle counts identical to #75: 16 at 183,194-183,522, ie_push 179,867, div_write 899,476. None enable PPU interrupt sources in IE, so the new IF bits do not alter their paths. stat_irq_blocking, stat_lyc_onoff and vblank_stat_intr are reproduced locally, not run as fixtures (#73). Tests verify the provisional model, not hardware timing.

## Live LY/STAT (#75)

Twelve new tests verify the [slice A contract](DEVICES.md#live-ly-and-stat-readback):

- `memory::ppu::timing::tests` (4): `stat_bits` for each mode; `visible_ly()` and `mode()` at every dot for two frames from enable and from the seed; line 153 shows 153 for exactly dots 0-3; LCD-off state reads LY 0 / HBlank.
- `memory::ppu::tests::status` (7), black-box through FF41/FF44:
  - startup LY 00 / STAT 85 with pure reads;
  - every STAT byte write (selects only);
  - STAT bit 7 and select persistence while the LCD is off;
  - every FF44 write ignored;
  - LY/STAT at every M-cycle for two frames, for LYC 0, 1, 143, 144, 153 and FF, with all selects set and IF unchanged, including exactly one LY 153 sample per frame;
  - the four stat_lyc_onoff held-flag states across LCD off/on.
- `system::tests::ppu_timing` (2, plus extended mirroring): every CPU LDH read of FF41/FF44 equals a reference Bus at that M-cycle over three frames, observing LY 0-152 and both flag states; budget pauses leave LY/STAT reads unchanged. The 9-M-cycle poll loop cannot sample the single LY 153 phase, so that case stays with the Bus-level tests.

The stat-bit-7 test was added during review after the original six missed a defect that a stray stored bit was masking. Existing tests migrated FF41/FF44 from fallback (startup 85/00), and two full-memory snapshots now exclude LY/STAT where time or LCDC edges legitimately change them. FF46 stays fallback.

**Fixture reporter path.** Live LY makes `is_ppu_broken` report a working PPU, so all eighteen fixtures now take Mooneye's on-screen result path. All still pass. Measured M-cycles:

| Fixture | Before #75 | After #75 |
| --- | --- | --- |
| 16 assertion fixtures | 3,943-4,677 | 183,194-183,522 |
| ie_push | 852,459 (LY-wait timeout) | 179,867 |
| div_write | 721,229 | 899,476 |

Each fixture added about 179,000 M-cycles. That is more than the contract's rough 53,000-cycle wait estimate, because screen setup costs more than estimated. `wait_ly_with_timeout` timeouts are silent and cost 851,968 M-cycles, so these counts show that every LY wait completed: all non-div_write fixtures stay far below one timeout, and a div_write timeout would exceed the unchanged 1,000,000 cap. Its margin is now about 100,000 M-cycles. Tests verify the provisional model, not hardware timing; calibration is #73.

## LCDC and dot clock (#74)

At 49b8dd4: 508 passed (370 unit, 138 integration). Fifteen new tests verify the [#74 contract](DEVICES.md#lcdc-and-internal-dot-clock):

- `memory::ppu::timing::tests` (5): constructor states; every dot for three frames after enable against an independent range-table model, which is itself spot-checked at each boundary; exact 70,224-dot frame period; the 452-dot post-boot tail.
- `memory::ppu::tests` (6): FF40 all-byte readback and read purity; 6 starting values × 256 writes for enable-edge effects; disabled freeze across all disabled bytes with an identical Timer/IF to an enabled bus, then restart from the enable write; four dots per Bus M-cycle and frame return to the seed; full-memory/Timer/instance isolation; STOP-entry phase preservation.
- `system::tests::ppu_timing` (4): per-M-cycle comparison with a reference Bus (LCDC stores applied after the cycle's four ticks), including zero elapsed dots after an enabling LDH store; budget-pause equivalence; HALT idles and interrupt entries (asserted counts); stopped polls add no dots and phase resumes after wake. They use a `#[cfg(test)]` timing probe because System tests cannot name private PPU types.

Four existing tests migrated FF40 from fallback to LCDC (startup 91). Tests verify the agreed provisional schedule, not hardware timing. A mutation check reintroducing the two review-found defects (VBlank only on line 144; enable-edge test never true) was not run, so their detection rests on reasoning about the every-dot and all-byte tests. The [passive LCD](DEVICES.md#passive-lcd-data-registers) and [video-memory](DEVICES.md#video-memory-foundation) tests remain. No PPU rendering or hardware timing result is claimed. The [M3 review](M3_COMPLETION_REVIEW.md) retains its earlier f0d38ef evidence, including the corrected rapid_toggle result in [timer evidence](testing/MOONEYE_TIMER.md).

## Commands

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

An ordinary fixture-free checkout explicitly ignores 46 external tests; ignored is not passed. With locally supplied pinned fixtures, run checksum-gated scripts:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-reg-f.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-cpu-subset.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-timer.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-ppu.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-cpu-interrupts.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-oam-dma.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-dmg-acid2.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-demo-roms.ps1
```

The execution-policy flag applies only to that process. Scripts reject missing/mismatched hashes before Cargo. After fixture verification, run the full suite including ignored tests:

```powershell
cargo test --workspace --all-features --no-fail-fast --quiet -- --include-ignored
```

The timer script and fixture-inclusive suite now pass rapid_toggle without changing its source expectation or budget. A negative invalid-byte probe previously verified that the timer script rejects a hash mismatch before Cargo starts. `cargo test --test cpu_rom_runner` runs synthetic runner acceptance and leaves external fixtures ignored. Unit/ignored/empty runs are not external passes.

## Actual external results

All eighteen artifacts are unchanged official prebuilt images from source revision 31510e12eea6286d36eea060a6adde755e1067aa, using ordinary Cartridge/System and current skipped-boot DMG startup. No binaries are tracked.

| Fixture | Result | M-cycles | Base ticks | Record |
| --- | --- | ---: | ---: | --- |
| reg_f | Passed | 183,194 | 732,776 | [Provenance/results](testing/MOONEYE_REG_F.md), [manifest](../tests/fixtures/mooneye-reg-f.json) |
| if_ie_registers | Passed | 183,364 | 733,456 | [Subset audit](testing/MOONEYE_CPU_SUBSET.md), [manifest](../tests/fixtures/mooneye-cpu-subset.json) |
| interrupts/ie_push | Passed | 179,867 | 719,468 | Same subset; LY waits complete since #75 |
| ei_sequence | Passed | 183,204 | 732,816 | Same subset |
| div_timing | Passed | 183,275 | 733,100 | Same subset; resets DIV explicitly |
| Thirteen timer fixtures | All passed | Per-fixture counts | Per-fixture counts | [Timer audit/results](testing/MOONEYE_TIMER.md), [manifest](../tests/fixtures/mooneye-timer.json) |

Every fixture matched the first FF01 packet and B/C/D/E/H/L decimal signature 3,5,8,13,21,34. Corrected rapid_toggle took 4605 M-cycles / 18420 ticks at M3 (183,203 since the #75 on-screen reporter); its earlier failure is preserved in the timer record. Caps remain 1,000,000 M-cycles and 10 host seconds per run, including a separately bounded diagnostic rerun on rapid_toggle failure. These results are finite checks, not complete timing/serial/PPU accuracy. External DAA [#52](https://github.com/Lorithan/gameboy-emu/issues/52) remains unrun and separate from the completed M3 gate.

## Runner contract

tests/support/rom_runner.rs observes public traces, with no production test special cases. FF01 writes are observable actions although serial is unimplemented. Only use this observer for sources audited to have no earlier SB reports and to select the common fast reporter under current FF44/FF02 fallback reads. Pinned sources: [reg_f](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/reg_f.s), [quit](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s), [PPU probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s), [serial probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_serial_broken.s).

The first six-byte packet plus register signature decides the result. Hex 42 in all six positions/registers is reported failure. Any other complete packet/register mismatch is malformed; partial output at timeout fails. Never scan past bad output for a later match. Other writes/incidental LD B,B bytes are not success.

Outcomes distinguish Passed, ReportedFailure, MalformedReport, TimedOut, UnexpectedStop and ExecutionError. Invalid/missing files fail setup. HALT consumes cycles; unexpected Stopped ends immediately rather than looping without a cycle budget advancing. Zero/exact budget boundaries and wall-time limits are tested. Do not silently increase caps to hide hangs.

Reports retain identity, packet, counts/ticks, snapshot and last 64 completed traces. Completion errors preserve/count the failed real cycle separately; request errors/stopped polls count zero. Host durations are diagnostics, not deterministic timing/performance promises. Eleven synthetic tests cover packet/signature/malformed/partial/wrong-address failures, loops/HALT/STOP, faults, limits, bounded history and file loading.

## Fixture policy

Before adding a fixture test, record source/revision, origin/SHA-256, accepted header, hardware/startup/device prerequisites, build/toolchain identity, component licenses, reporting and finite budgets. Keep ROM/archive binaries ignored under target/test-roms. Do not fetch moving latest data during tests or patch headers. Real bugs get isolated user-owned fixes and assistant regressions.

The [reg_f record](testing/MOONEYE_REG_F.md) preserves archive/build/MIT notices and separately attributed Darkrose font details; the whole ROM is not labeled exclusively MIT. No independent reproduced/signed build is claimed. Retain component provenance before redistribution. Other suites/flat-memory vectors need separate adapter/prerequisite/license audits.
