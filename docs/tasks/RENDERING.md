# Rendering into a framebuffer

Proposed and agreed 2026-11-21 for M4 (decisions and host interface below). All four slices are complete (2026-11-22); see testing for [R1](../TESTING.md#rendering-r1-frame-publication-and-background-81), [R2](../TESTING.md#rendering-r2-the-window-82), [R3](../TESTING.md#rendering-r3-objects-83) and [R4](../TESTING.md#rendering-r4-dmg-acid2-84). The user implements; the assistant owns the tests, fixture wiring and review.

This slice draws the DMG picture into an in-core framebuffer. It does not open a window, map shades to colours or pace frames; that is the [desktop frontend](FRONTEND.md) (SDL3, in its own workspace crate). The core stays independent of SDL3.

## Sources

| Source | Use |
| --- | --- |
| [dmg-acid2](https://github.com/mattcurrie/dmg-acid2) v1.0 (MIT), `dmg-acid2.gb` and `img/reference-dmg.png` | The visual fixture. "A simple line based renderer is sufficient"; it "does NOT perform register writes during the PPU's mode 3"; it writes registers on chosen rows from LY=LYC interrupts during mode 2. Its failure table covers every rule below. Reference shades are greyscale 00, 55, AA, FF. Provenance, hashes and the completion condition are pinned in [dmg-acid2.json](../../tests/fixtures/dmg-acid2.json). |
| [Pan Docs](https://gbdev.io/pandocs/) (Rendering, Tile Data, Tile Maps, OAM, LCDC, Palettes) | Rule text for the cases below. |

## Rules

Coordinates: `x` 0-159, `ly` 0-143. A *colour index* is the 2-bit value from tile data (0-3); a *shade* is the value after the palette (0 lightest, 3 darkest).

1. **Background** (LCDC bit 0 set). Pixel `(x, ly)` reads the background at `((SCX + x) mod 256, (SCY + ly) mod 256)`. The tile map is 9C00 if LCDC bit 3 is set, else 9800 (32×32 tile numbers). Tile data: LCDC bit 4 set uses 8000 with unsigned tile numbers; clear uses 9000 with signed tile numbers (8800-97FF). Each tile row is two bytes: the low bit plane, then the high bit plane; bit 7 is the leftmost pixel. Shade = `(BGP >> (2 * index)) & 3`.
2. **Background disabled** (LCDC bit 0 clear, DMG): background and window are blank, colour index 0 shaded through BGP. Objects still draw and see background index 0 for priority.
3. **Window** (LCDC bits 5 and 0 set). Checked against Pan Docs (Window, Tile Maps, LCDC) on 2026-11-22.
   - **Y condition.** It becomes true at the start of a line where `LY == WY` and stays true until VBlank clears it. On DMG this does not depend on LCDC bit 5, so a window enabled after its WY line still appears (GBC resets it when bit 5 is cleared). The renderer checks at the start of Drawing, so WY writes during OAM scan are a known limit.
   - **Horizontal.** The window covers pixels with `x + 7 >= WX` on lines where the Y condition is true and `WX <= 166`. WX 1-6 start it off the left edge; WX 167 and above and WY 144 and above hide it.
   - **Content.** It uses its own tile map (9C00 if LCDC bit 6 set, else 9800) and the same tile data selection as the background. It ignores SCX and SCY. Window pixels replace background pixels.
   - **Internal line counter.** Row 0 on the first line the window is drawn. It increments only after lines where it was drawn, so hiding the window by LCDC bit 5 or by WX pauses it and showing it again resumes the next row. It resets each frame (Pan Docs does not state the reset; it follows from the Y condition clearing at VBlank).
   - **Not modelled (DMG quirks, provisional):** WX 0 (shifted left by `SCX % 8`), WX 166 (full-width window one line lower), and the single colour-0 pixel inserted when bit 5 is clear but the other conditions hold. dmg-acid2 does not use them. Whether the counter advances while LCDC bit 0 is clear is not stated and not tested.
4. **Object selection** (LCDC bit 1 set). For each line, scan OAM in order and select the first 10 objects with `ly + 16` in `[Y, Y + height)`, where height is 16 if LCDC bit 2 is set, else 8. X is not considered when selecting, so off-screen objects (X 0 or 168+) still count toward the 10. Y 0-1 and 160+ hide an object in both sizes; Y 2-8 show only the lower rows of an 8×16 object. (Pan Docs, OAM, checked 2026-11-22.)
5. **Object pixels.** Object `x` position is `X - 8`. Attributes: bit 7 BG priority, bit 6 Y flip, bit 5 X flip, bit 4 palette (OBP1 if set, else OBP0). Tile data always uses 8000 unsigned. For 8×16 objects bit 0 of the tile number is ignored (top tile `n & 0xFE`, bottom `n | 1`), and Y flip mirrors all 16 rows. Colour index 0 is transparent, so the low two bits of OBP0 and OBP1 are ignored (Pan Docs, Palettes). Attribute bits 0-3 are CGB-only and ignored.
6. **Object priority.** At each pixel, take the highest-priority selected object whose pixel there is not transparent: smaller X wins, and with equal X the lower OAM index wins. Transparent pixels let lower-priority objects show through. Object-to-object priority is resolved first, then the winner's BG-priority bit: it is hidden where the background/window colour index is 1-3, and in that case no lower-priority object shows there either. Pan Docs (OAM, checked 2026-11-22): "an object with a higher priority but with 'BG over OBJ' enabled will sort of 'mask' lower-priority objects". With LCDC bit 0 clear the background is colour 0, so BG priority never hides an object (derived from the colour 1-3 rule; Pan Docs states the DMG case only for CGB). Not modelled: LCDC bit 1 or 2 changes and OAM writes during mode 2 or 3 apply from the next line, so Pan Docs' mid-line size-change artifacts are out of scope.
7. **Frame output.** The PPU fills a frame of 160×144 shades. The frame becomes visible to the host when the PPU enters line 144 (VBlank), the same moment as the VBlank interrupt. Rendering never changes timing, STAT, interrupts or memory.

## Proposed design

- **Scanline renderer.** Each visible line is rendered in one step when timing enters Drawing (dot 80 of lines 0-143), using VRAM, OAM and registers as they are at that moment. Mode 3 keeps its fixed 172-dot length. This covers register writes made during HBlank and OAM scan, which is what dmg-acid2 and most games use. Mid-line effects (writes during mode 3) apply to the next line instead of mid-line, a known limit. A pixel-FIFO renderer would remove that limit but needs variable mode-3 length, which the D3 calibration does not have; it would be a separate later issue.
- **Ownership.** Ppu owns the renderer state (window line counter, WY latch, the frame in progress and the last complete frame). The line renderer reads Ppu storage directly, not through the CPU's locked accessors.
- **Boundary.** The per-line work can be a function from (VRAM, OAM, registers, line, window state) to 160 shades, in its own Ppu submodule. That keeps it testable without clocking a whole frame. Your choice of exact signature. Each visible line maps 2-bit colour indices to `Shade` through the palettes.
- **Host interface.** System exposes, read-only:
  - the last complete frame, as a public `Frame` type with width and height constants and a way to read a shade at `(x, y)` or all pixels as a slice;
  - a count of completed frames, so the host can tell when a new frame is ready by comparing it to the last value it saw.

  No new step result or callback is needed; the host polls after each batch of steps.

## Decisions (agreed 2026-11-21)

1. **Scanline renderer** at the start of Drawing, with mode 3 kept at its fixed length. Pixel FIFO and variable mode-3 length are deferred to a later calibration issue (the `hblank_ly_scx_timing-GS` and `intr_2_mode0_timing_sprites` probes belong to it).
2. **Double-buffered frame:** Ppu draws into one buffer and publishes it at VBlank entry; a completed-frame counter increments on each publication.
3. **LCD off publishes a blank frame:** turning the LCD off publishes one all-`White` frame and increments the counter once; no frames are produced while it stays off. The first frame after turning it back on is published normally (hardware shows it blank; provisional).
4. **Shades are an enum**, so an invalid shade cannot be represented.

## Host interface (agreed 2026-11-21, implemented in R1)

The tests name these. Internals are the user's choice. The implementation also adds a crate-internal `Shade::from_bits` (low two bits).

- A public `video` module at the crate root holds the host-facing types, with no PPU internals:
  - `Shade`: `White`, `LightGray`, `DarkGray`, `Black` (shades 0-3).
  - `Frame`: `Frame::WIDTH` (160) and `Frame::HEIGHT` (144) constants, `pixel(x, y) -> Shade` (panics out of range), and `pixels() -> &[Shade]` in row-major order (`y * WIDTH + x`).
- `System::frame(&self) -> &Frame`: the last published frame. Before the first publication it is all `White`.
- `System::frames_completed(&self) -> u64`: starts at 0 and increments on each publication (each VBlank entry with the LCD on, and once per LCD disable).

## Slices

Each slice is a separate issue with tests written first.

| Slice | Scope | Evidence |
| --- | --- | --- |
| R1 [#81](https://github.com/Lorithan/gameboy-emu/issues/81), complete 2026-11-22 | `Frame`, publication at VBlank, LCD-off behaviour, System accessors; background only (rules 1, 2, 7) | Local tests against an independent model: tile data modes, both maps, SCX/SCY wrap, BGP, LCDC bit 0; publication timing and counter; the 42 existing fixtures keep their counts |
| R2 [#82](https://github.com/Lorithan/gameboy-emu/issues/82), complete 2026-11-22 | Window (rule 3) | 10 tests in `src/system/tests/rendering.rs`: WX 1-8, 50, 159, 165 and 167+, WY 1, 77, 143 and 144+; both window maps with both tile data modes under scroll; bit 5 and bit 0 hiding; the counter paused by bit 5 and by WX; the Y condition latching, set while disabled, and restarting each frame. Validated against a throwaway reference: 11 mutants killed; mid-frame LYC/STAT and LCD-restart tests added in review. The 42 fixtures keep their counts |
| R3 [#83](https://github.com/Lorithan/gameboy-emu/issues/83), complete 2026-11-22 | Objects (rules 4-6) | 8 tests in `src/system/tests/rendering.rs`: edge positions in X and Y for both sizes; every flip and palette combination with odd 8×16 tiles and CGB bits set; the 10-object limit with off-screen objects counting and OAM order deciding; X then OAM-index priority with show-through; BG priority over background and window, masking, and LCDC bit 0 clear; dense random OAM (4 seeds × 5 LCDC settings); LCDC bit 1 clear; LCDC bits 1-2 and OBP splits in HBlank. Validated against a throwaway reference: 18 mutants killed |
| R4 [#84](https://github.com/Lorithan/gameboy-emu/issues/84), complete 2026-11-22 | dmg-acid2 fixture | The frame at the ROM's `ld b, b` breakpoint matches `reference-dmg.png` at all 23,040 pixels. The PNG is decoded by the checksum-gated script with .NET into a pinned shade text file, so no dependency is added. Five of six renderer mutants fail it; BG-priority masking is covered only by the local test |

## Not in scope

Pixel FIFO and variable mode-3 length; CGB; colour mapping, windowing, frame pacing and input (frontend); the OAM corruption bug (#78).
