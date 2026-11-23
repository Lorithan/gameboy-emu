# M4 completion review

**Proposed 2026-11-23 at [7b10751](https://github.com/Lorithan/gameboy-emu/commit/7b10751). The user agreed to defer #78 and added F5 ([#89](https://github.com/Lorithan/gameboy-emu/issues/89), opening a ROM from the window) to M4, so M4 closes after F5 and this review is updated then.** M4 meets its declared gate: "visual fixtures, PPU/DMA/input checks and an agreed ROM-only interactive demo" ([plan](PROJECT_PLAN.md)). Every M4 issue except [#78](https://github.com/Lorithan/gameboy-emu/issues/78) is closed. #78 states that it is not required for the M4 gate unless a compatibility case needs it, and none has appeared. The proposal is to close [milestone M4](https://github.com/Lorithan/gameboy-emu/milestone/5) and carry #78 forward as a documented limit.

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

**Regression at this checkpoint (2026-11-23):**
- **Tests.** `cargo test --workspace --all-features --no-fail-fast -- --include-ignored` passed **638, zero failed, zero ignored** (613 core, 25 frontend).
- **Lint.** `cargo fmt --all --check` and workspace-wide strict Clippy pass with no warnings.
- **Fixture scripts.** All eight pass: Mooneye reg-f, CPU subset, timer, PPU, CPU interrupts, OAM DMA, dmg-acid2 and demo ROMs.
- **Fixture counts.** All 43 external fixture M-cycle counts (42 Mooneye plus dmg-acid2) are identical to the pre-frontend baseline `b446039`, compared ROM by ROM.

## Remaining limits and separate work

- **Scanline renderer.** Each line is drawn at the start of Drawing, and mode 3 has a fixed length. Writes during mode 3 apply from the next line. A pixel FIFO with variable mode-3 length would be a separate calibration issue; the `hblank_ly_scx_timing-GS` and `intr_2_mode0_timing_sprites` probes belong to it.
- **Unmodelled DMG quirks.** WX 0 and WX 166, the colour-0 pixel when the window is disabled, and whether the window counter advances with LCDC bit 0 clear. The first frame after LCD enable is shown normally; hardware shows it blank (provisional).
- **[#78](https://github.com/Lorithan/gameboy-emu/issues/78).** The OAM corruption bug and FEA0-FEFF reads stay open and non-blocking.
- **No APU.** Sound registers are unimplemented; both demo games run without hanging. Audio is M6, and pacing then moves to the audio queue as agreed.
- **Frontend.** No rebinding, gamepads or save states. CPU use is about 20% of one core, above what emulation needs; profiling would be a separate issue. Release builds need the MSVC C runtime.
- **Startup.** It still skips the boot ROM. The M3 limits (four-tick scheduling granularity, STOP model, #64, #52) are unchanged.
- **Cartridges.** ROM-only only. MBC1 is M5. Candidate MIT-licensed MBC1 homebrew were found during F4: Porklike, PostBot, JP, Domination, and Tobu Tobu Girl.

## Next

On agreement: close milestone M4, keep #78 open in a later milestone, and start M5 (MBC1 and applicable persistent RAM) with its architecture and contract before implementation.
