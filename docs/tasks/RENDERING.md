# Rendering into a framebuffer (proposal)

Proposed 2026-11-21 for M4; **not yet agreed**. No issue exists yet; issues follow once the slices are agreed. The user implements; the assistant owns the tests, fixture wiring and review.

This slice draws the DMG picture into an in-core framebuffer. It does not open a window, map shades to colours or pace frames; that is the later SDL2 frontend, whose scope is agreed separately. The core stays independent of SDL2.

## Sources

| Source | Use |
| --- | --- |
| [dmg-acid2](https://github.com/mattcurrie/dmg-acid2) v1.0 (MIT), `dmg-acid2.gb` and `img/reference-dmg.png` | The visual fixture. "A simple line based renderer is sufficient"; it "does NOT perform register writes during the PPU's mode 3"; it writes registers on chosen rows from LY=LYC interrupts during mode 2. Its failure table covers every rule below. Reference shades are greyscale 00, 55, AA, FF. Provenance (asset hashes, tag commit, reference image revision, completion condition) is recorded before the fixture slice. |
| [Pan Docs](https://gbdev.io/pandocs/) (Rendering, Tile Data, Tile Maps, OAM, LCDC, Palettes) | Rule text for the cases below. |

## Rules

Coordinates: `x` 0-159, `ly` 0-143. A *colour index* is the 2-bit value from tile data (0-3); a *shade* is the value after the palette (0 lightest, 3 darkest).

1. **Background** (LCDC bit 0 set). Pixel `(x, ly)` reads the background at `((SCX + x) mod 256, (SCY + ly) mod 256)`. The tile map is 9C00 if LCDC bit 3 is set, else 9800 (32×32 tile numbers). Tile data: LCDC bit 4 set uses 8000 with unsigned tile numbers; clear uses 9000 with signed tile numbers (8800-97FF). Each tile row is two bytes: the low bit plane, then the high bit plane; bit 7 is the leftmost pixel. Shade = `(BGP >> (2 * index)) & 3`.
2. **Background disabled** (LCDC bit 0 clear, DMG): background and window are blank, colour index 0 shaded through BGP. Objects still draw and see background index 0 for priority.
3. **Window** (LCDC bits 5 and 0 set). The window starts on the first line of a frame where `ly == WY` (latched for the rest of the frame) and covers pixels with `x + 7 >= WX` when `WX <= 166`. It uses its own tile map (9C00 if LCDC bit 6 set, else 9800), the same tile data selection, and an **internal line counter**: row 0 on the first line it draws, incremented only after lines where it was actually drawn, reset each frame. Window pixels replace background pixels.
4. **Object selection** (LCDC bit 1 set). For each line, scan OAM in order and select the first 10 objects with `ly + 16` in `[Y, Y + height)`, where height is 16 if LCDC bit 2 is set, else 8. X is not considered when selecting, so off-screen objects still count toward the 10.
5. **Object pixels.** Object `x` position is `X - 8`. Attributes: bit 7 BG priority, bit 6 Y flip, bit 5 X flip, bit 4 palette (OBP1 if set, else OBP0). Tile data always uses 8000 unsigned. For 8×16 objects bit 0 of the tile number is ignored (top tile `n & 0xFE`, bottom `n | 1`), and Y flip mirrors all 16 rows. Colour index 0 is transparent.
6. **Object priority.** At each pixel, take the highest-priority selected object whose pixel there is not transparent: smaller X wins, and with equal X the lower OAM index wins. Transparent pixels let lower-priority objects show through. Object-to-object priority is resolved first, then the winner's BG-priority bit: it is hidden where the background/window colour index is 1-3, and in that case no lower-priority object shows there either. (Pan Docs, object priority; the exact text is to be confirmed before R3.)
7. **Frame output.** The PPU fills a frame of 160×144 shades. The frame becomes visible to the host when the PPU enters line 144 (VBlank), the same moment as the VBlank interrupt. Rendering never changes timing, STAT, interrupts or memory.

## Proposed design

- **Scanline renderer.** Each visible line is rendered in one step when timing enters Drawing (dot 80 of lines 0-143), using VRAM, OAM and registers as they are at that moment. Mode 3 keeps its fixed 172-dot length. This covers register writes made during HBlank and OAM scan, which is what dmg-acid2 and most games use. Mid-line effects (writes during mode 3) apply to the next line instead of mid-line, a known limit. A pixel-FIFO renderer would remove that limit but needs variable mode-3 length, which the D3 calibration does not have; it would be a separate later issue.
- **Ownership.** Ppu owns the renderer state (window line counter, WY latch, the frame in progress and the last complete frame). The line renderer reads Ppu storage directly, not through the CPU's locked accessors.
- **Boundary.** The per-line work can be a function from (VRAM, OAM, registers, line, window state) to 160 shades, in its own Ppu submodule. That keeps it testable without clocking a whole frame. Your choice of exact signature.
- **Host interface.** System exposes, read-only:
  - the last complete frame, as a public `Frame` type with width and height constants and a way to read a shade at `(x, y)` or all pixels as a slice;
  - a count of completed frames, so the host can tell when a new frame is ready by comparing it to the last value it saw.

  No new step result or callback is needed; the host polls after each batch of steps.

## Decisions

1. **Scanline renderer at the start of Drawing** (proposed above) vs. pixel FIFO now.
2. **Frame publication:** two buffers in Ppu, copied (or swapped) at VBlank entry, plus a completed-frame counter (proposed) vs. a single buffer the host reads whenever it likes (simpler, but it can show a half-drawn frame).
3. **LCD off:** proposed: when the LCD is turned off, the published frame becomes all shade 0 and the counter increments once, so the host shows a blank screen; no frames are produced while it stays off. The first frame after turning it back on is published normally (hardware shows it blank; provisional). Alternative: leave the last frame visible and let the host decide.
4. **Pixel type:** a `u8` holding 0-3 with that invariant documented, or a small `Shade` enum. The enum makes invalid shades unrepresentable; the `u8` is simpler to compare against reference images. Your call; tests can be written either way.
## Proposed slices

Each slice is a separate issue with tests written first.

| Slice | Scope | Evidence |
| --- | --- | --- |
| R1 | `Frame`, publication at VBlank, LCD-off behaviour, System accessors; background only (rules 1, 2, 7) | Local tests against an independent model: tile data modes, both maps, SCX/SCY wrap, BGP, LCDC bit 0; publication timing and counter; the 42 existing fixtures keep their counts |
| R2 | Window (rule 3) | WX/WY edges, the line counter across lines where the window is turned off and on, WY latching |
| R3 | Objects (rules 4-6) | Selection limit and order, X/Y flip, 8×16, palettes, priority ties, BG priority with LCDC bit 0 clear |
| R4 | dmg-acid2 fixture | Provenance recorded first; the frame matches `reference-dmg.png` exactly; how the reference is compared without adding a dependency is decided then |

## Not in scope

Pixel FIFO and variable mode-3 length; CGB; colour mapping, windowing, frame pacing and input (frontend); the OAM corruption bug (#78).
