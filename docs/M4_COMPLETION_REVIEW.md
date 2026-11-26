# M4 completion review

**Closed 2026-11-26.** Proposed 2026-11-23 at [7b10751](https://github.com/Lorithan/gameboy-emu/commit/7b10751). The user agreed to defer #78 and added F5 ([#89](https://github.com/Lorithan/gameboy-emu/issues/89), opening a ROM from the window) to M4; F5 is complete, so M4 closes. M4 meets its declared gate: "visual fixtures, PPU/DMA/input checks and an agreed ROM-only interactive demo" ([plan](PROJECT_PLAN.md)). Every M4 issue is closed; [#78](https://github.com/Lorithan/gameboy-emu/issues/78) was moved out of M4 as a documented, non-blocking limit.

## Gate evidence

| Gate | Evidence |
| --- | --- |
| PPU registers, timing and interrupts | #71-#77: video memory, LCD registers, LCDC and the dot clock, live LY/STAT, STAT/VBlank interrupts, VRAM/OAM access locks; local tests per contract ([devices](DEVICES.md), [contracts](tasks/LCD_REGISTERS.md)). #73 calibration: ten pinned Mooneye PPU ROMs pass ([calibration](tasks/PPU_CALIBRATION.md)). |
| OAM DMA | #80: Bus-owned transfer with timeline, restart, source mapping and CPU blocking; 12 local tests; five Mooneye DMA ROMs pass ([contract](tasks/OAM_DMA.md)). |
| Rendering | #81-#83: background, window and objects drawn by a scanline renderer into a double-buffered frame published at VBlank. 30 local System tests against an independent model, validated beforehand against throwaway references with 9, 11 and 18 deliberate breaks, all caught ([rendering](tasks/RENDERING.md)). |
| Visual fixture | #84: dmg-acid2 v1.0 matches `reference-dmg.png` at all 23,040 pixels; five of six renderer breaks fail it ([testing](TESTING.md#rendering-r4-dmg-acid2-84)). |
| Desktop frontend | #85-#86: Cargo workspace with a portable core and an SDL3 frontend, statically linked. 15 unit tests for frame conversion, scaling and pacing; manual checklist including 59.73 frames per second measured over 60 s ([frontend](tasks/FRONTEND.md#f2-result-complete-2026-11-22)). |
| Input | Core joypad (earlier milestones) plus #87: binding table, held keys, last-press-wins per axis, focus-loss release; 10 unit tests validated against 12 breaks; manual key checklist passed. |
| Interactive demo | #88: MaxPirate v1.0 (MIT) is the agreed ROM-only demo, with GB Snake (WTFPL) for Select; both pinned by hash. Headless smoke tests show they react to buttons. The user played both with no problems ([results](tasks/FRONTEND.md#f3-and-f4-result-complete-2026-11-23)). |
| Launchable program | #89 F5: a picker screen with a native file dialog, drag-and-drop, Ctrl+O, errors in the window, pause, and no console in release builds. 19 layout unit tests validated against 27 breaks; the manual checklist passed, run by the user and the assistant ([results](tasks/FRONTEND.md#f5-result-complete-2026-11-26)). |

**Regression at the proposal (2026-11-23):**
- **Tests.** `cargo test --workspace --all-features --no-fail-fast -- --include-ignored` passed **638, zero failed, zero ignored** (613 core, 25 frontend).
- **Lint.** `cargo fmt --all --check` and workspace-wide strict Clippy pass with no warnings.
- **Fixture scripts.** All eight pass: Mooneye reg-f, CPU subset, timer, PPU, CPU interrupts, OAM DMA, dmg-acid2 and demo ROMs.
- **Fixture counts.** All 43 external fixture M-cycle counts (42 Mooneye plus dmg-acid2) are identical to the pre-frontend baseline `b446039`, compared ROM by ROM.

**Regression at closing (2026-11-26):** `cargo test --workspace --all-features --no-fail-fast -- --include-ignored` passed **657, zero failed, zero ignored** (613 core, 44 frontend). fmt and Clippy pass. The core, its tests and Cargo.lock are unchanged since 7b10751, so the fixture scripts and counts above still apply.

## Remaining limits and separate work

- **Scanline renderer.** Each line is drawn at the start of Drawing, and mode 3 has a fixed length. Writes during mode 3 apply from the next line. A pixel FIFO with variable mode-3 length would be a separate calibration issue; the `hblank_ly_scx_timing-GS` and `intr_2_mode0_timing_sprites` probes belong to it.
- **Unmodelled DMG quirks.** WX 0 and WX 166, the colour-0 pixel when the window is disabled, and whether the window counter advances with LCDC bit 0 clear. The first frame after LCD enable is shown normally; hardware shows it blank (provisional).
- **[#78](https://github.com/Lorithan/gameboy-emu/issues/78).** The OAM corruption bug and FEA0-FEFF reads stay open and non-blocking.
- **No APU.** Sound registers are unimplemented; both demo games run without hanging. Audio is M6, and pacing then moves to the audio queue as agreed.
- **Frontend.** No rebinding, gamepads, save states or settings file. The event loop, dialog and screen changes are checked by hand only. CPU use is about 20% of one core, above what emulation needs; profiling would be a separate issue. Release builds need the MSVC C runtime.
- **Startup.** It still skips the boot ROM. The M3 limits (four-tick scheduling granularity, STOP model, #64, #52) are unchanged.
- **Cartridges.** ROM-only only. MBC1 is M5. Candidate MIT-licensed MBC1 homebrew were found during F4: Porklike, PostBot, JP, Domination, and Tobu Tobu Girl. The frontend titles the window with the ROM's file name. Reading the header title (`Cartridge::title()`, 0134-0143, 11 or 16 bytes) is a candidate for M5, which parses the header for the MBC type anyway; the frontend could then prefer a printable, non-empty header title.

## Next

M4 is closed. #78 stays open outside M4. Next is M5 (MBC1 and applicable persistent RAM), starting with its architecture and contract before implementation.
