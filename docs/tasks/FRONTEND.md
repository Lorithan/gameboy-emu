# Desktop frontend (M4)

Proposed and agreed 2026-11-22, including the layout, behaviour, two-phase pacing and static linking below. Remaining question 2 (the demo ROM) is still open; issues F1-F4 are #85-#88. The user implements the frontend and the workspace change; the assistant owns tests, fixture wiring, review and documentation.

M4's gate needs "desktop input … and an agreed ROM-only interactive demo". The core already renders frames ([rendering](RENDERING.md)), accepts buttons (`System::set_button`) and leaves pacing to the host. This document covers the program that shows those frames in a window and feeds the keyboard into the core. Audio is M6.

## Agreed (2026-11-22)

1. **SDL3, through the `sdl3` crate,** pinned to an exact version. All SDL code stays in the frontend crate, so binding upgrades touch nothing else. SDL3 has everything the frontend and M6 audio need: a window, streaming textures, vsync and timers, keyboard and gamepad events, and audio streams. The Rust wrapper is pre-1.0 and still changing its API; `sdl3-sys` is the complete low-level fallback. This replaces the earlier SDL2 plan.
2. **A Cargo workspace.** The emulator core stays a library with no platform dependencies. Each frontend is its own crate: SDL now, web later (`wasm32-unknown-unknown` with `wasm-bindgen`; canvas, browser key events, WebAudio).

## Agreed design

### Layout and boundary

- **Core stays the root package.** The root `Cargo.toml` keeps `[package]` for `gameboy-emu` and gains a `[workspace]` with the frontend as a member. Fixture tests resolve ROMs through `CARGO_MANIFEST_DIR/target/test-roms`, and Cargo puts the workspace `target` at the root. So all seven fixture scripts, the manifests and the doc links keep working unchanged. Moving the core into a subfolder would break those paths.
- **Frontend crate:** `frontends/sdl`, package `gameboy-sdl`, a binary that depends on the core by path. The core's `src/main.rs` stub is removed, so the core is library-only.
- **The core stays portable.** It has no SDL types, threads, clocks or host I/O (beyond `Cartridge::from_filepath`, which a browser would replace with `from_bytes`) and no dependencies. A check that it builds for `wasm32-unknown-unknown` can be added when the web frontend starts; it needs that target installed.
- **Shared host logic** (shade colours, key map, pacing) stays in `gameboy-sdl` until a second frontend needs it. Then it moves to a small shared crate.

### Behaviour

| Topic | Proposal |
| --- | --- |
| Starting | `gameboy-sdl <rom.gb>`. A missing or invalid ROM prints the core's error and exits non-zero. No file dialog. |
| Window | 160×144 at integer scale 4 by default (640×576), resizable. The picture keeps integer scaling, centred, with nearest-neighbour filtering. |
| Colours | Greyscale FF, AA, 55, 00 for shades 0-3, the values dmg-acid2 uses, so screenshots compare directly. Other palettes can come later. |
| Presenting | Present whenever `frames_completed()` changes; between frames the window keeps the last image. While the LCD is off, the blank frame the core published is shown. |
| Pacing | Vsync off, in two phases behind one small pacing boundary, such as a `Pacer` that answers whether another frame may run. **Until M6, a timer:** 70,224 dots per frame at 4,194,304 Hz is about 59.73 frames per second; the loop runs the core for the elapsed real time, capped so a stall does not cause a burst of catch-up, then sleeps. **From M6, the audio queue** (the user's suggestion): samples generated from emulated time go into an `SDL_AudioStream`, and the core runs only while the queue holds less than a target of about 2-3 frames of audio. Audio is then the master clock, so sound never stutters. The timer stays as the fallback with no audio device. A 60 Hz vsync would run about 0.5% fast and drift against audio. |
| Keys | Arrows: D-pad. X: A. Z: B. Enter: Start. Right Shift or Backspace: Select. Esc or closing the window: quit. Key repeat is ignored; each key sends one press and one release to `set_button`. |
| Not included | Audio (M6), save states, rebinding, gamepads, debug views, fast-forward. |

### Testing

- **Pure logic is unit tested without SDL:** frame-to-RGBA conversion, the key map, and the pacing accumulator, including the catch-up cap and the 59.73 Hz long-run rate.
- **SDL glue is checked by hand** against a written checklist per slice. A passing build does not show that the window works.
- **The core's evidence is unchanged:** all existing tests, scripts and the 43 fixture counts must stay identical after the workspace change.

### Remaining questions

1. **How SDL3 is built and linked on Windows.** Agreed: build from source and **link statically**, through the `sdl3` crate's features. That gives one executable with no `SDL3.dll` to place next to it, and the SDL version is pinned by `Cargo.lock`. SDL's zlib licence allows static linking, and SDL's dynamic API override (`SDL3_DYNAMIC_API`) can still swap the library at run time; confirm at F2 that the static build keeps it. Costs: about one to two minutes of CMake on the first build (cached afterwards), and a few MB of executable. This machine has Visual Studio 2022 Community with the C++ tools (checked 2026-11-22); CMake still needs installing. The exact crate features are confirmed at F2.
2. **The demo ROM:** it must be ROM-only, interactive, freely licensed, and runnable from local fixtures without being committed. Candidates still need to be found and their licences checked. dmg-acid2 shows a correct still picture but takes no input.

## Slices

Each is a separate issue with acceptance criteria written first.

| Slice | Scope | Evidence |
| --- | --- | --- |
| F1 [#85](https://github.com/Lorithan/gameboy-emu/issues/85), complete 2026-11-22 | Workspace split, core library-only, empty `gameboy-sdl` binary | `cargo build --workspace`; full suite, fmt, strict Clippy across the workspace; all seven fixture scripts pass with identical counts |
| F2 [#86](https://github.com/Lorithan/gameboy-emu/issues/86) | SDL3 window shows frames from a ROM given on the command line, with pacing | Unit tests for RGBA conversion and pacing; dmg-acid2 shown matching its reference; the checklist is run and recorded |
| F3 [#87](https://github.com/Lorithan/gameboy-emu/issues/87) | Keyboard input | Unit tests for the key map; a manual check with a ROM that reacts to buttons |
| F4 [#88](https://github.com/Lorithan/gameboy-emu/issues/88) | Demo ROM chosen, pinned and recorded | Provenance and licence recorded first; the demo is played through the checklist |

## F2 contract (agreed 2026-11-22, [#86](https://github.com/Lorithan/gameboy-emu/issues/86))

**Dependency:** `sdl3 = { version = "=0.22.1", features = ["build-from-source-static"] }`. It pins `sdl3-sys` 0.7.3, which bundles SDL 3.4.20. CMake 4.5.1 is installed; VS Code and terminals need a restart to see it on PATH.

**Loop.** Run by emulated time, not by published frames. While the LCD is off the core publishes nothing, so waiting for a frame would hang.
- **Each due frame runs exactly 17,556 M-cycles.** That is 70,224 dots, one LCD frame.
- **Present** whenever `frames_completed()` has changed.
- **STOP:** a `Stopped` step ends that frame's work early, without spinning.
- **Core error:** print it and exit non-zero.

**Testable interface** (pure, no SDL). The names are the tests' contract; internals are the user's choice.

```rust
// frontends/sdl/src/display.rs
pub const RGBA_LEN: usize = Frame::WIDTH * Frame::HEIGHT * 4;
pub fn shade_rgba(shade: Shade) -> [u8; 4];                  // FF/AA/55/00 grey, alpha FF
pub fn write_rgba32(frame: &Frame, out: &mut [u8; RGBA_LEN]); // row-major R, G, B, A; pitch WIDTH * 4
pub fn integer_viewport(window_w: u32, window_h: u32) -> (i32, i32, u32, u32);

// frontends/sdl/src/pacing.rs
pub const MCYCLES_PER_FRAME: u32 = 17_556;
pub const MAX_CATCH_UP: u32;                                 // user's choice, at least 1 (3 suggested)
pub struct Pacer;
impl Pacer {
    pub fn new(start: Instant) -> Self;
    pub fn frames_due(&mut self, now: Instant) -> u32;
    pub fn next_due(&self) -> Instant;
}
```

- **`integer_viewport`** returns `(x, y, width, height)`. The scale is the largest whole number of at least 1 that fits, `min(w / 160, h / 144)`. The picture is centred by Rust's truncating division, so a window smaller than 1x gets negative offsets.
- **Frame deadlines.** Frame n (n = 0, 1, ...) is due at `start + floor(n * 70,224 * 10^9 / 4,194,304)` ns. Each deadline is computed from the start, never accumulated, because one frame is 16,742,706.298828125 ns. Frame 0 is due at the start.
- **Return values.** `frames_due` returns how many due frames have not been returned yet. `next_due` is the deadline of the next one.
- **Catch-up.** With more than `MAX_CATCH_UP` frames due, `frames_due` returns `MAX_CATCH_UP` and drops the rest. It then counts the schedule again from `now`, so the next frame is due at `now + deadline(1)`.

**Unit tests** (assistant, written 2026-11-22):
- **Files.** `frontends/sdl/src/display/tests.rs` (8) and `pacing/tests.rs` (7), wired by `#[cfg(test)] mod tests;` in each module.
- **Display: shades and layout.** Exact greys; a blank frame overwriting a sentinel buffer; every pixel in row-major order from a frame rendered by a small hand-assembled ROM. The test first checks that frame has all four shades and no simple symmetry.
- **Display: viewport.** Explicit window sizes, windows below 1x, and a sweep over sizes up to 2000×2000 checking maximal scale and centring.
- **Pacing.** The M-cycle constant; the first frame at the start; each of 600 frames due exactly at its deadline and not 1 ns before; an hour polled every 5 ms giving exactly 215,020 frames with no drift; stalls within and beyond the cap, including the resynchronisation.
- **Validation.** In a scratch worktree, against stubs 14 of 15 fail (the buffer-length test passes on the stub's constant). A reference implementation passes all 15, and each of 16 deliberate breaks fails at least one:
  - zero alpha; swapped greys; reversed pixel order; white pixels skipped;
  - no minimum scale; `max` instead of `min`; not centred; floor offsets;
  - an accumulated rounded duration; rounded-up deadlines; first frame late; no cap;
  - dropping the backlog without resynchronising; resynchronising with the next frame due immediately;
  - 17,557 M-cycles; a cap of 0.
- **Expected-value fixes.** Against the reference, two of the tests' own expected values were wrong and were corrected: the 1920×1080 and 500×2000 viewports, and the hour's frame count, 215,020 rather than 215,019.

**Manual checklist** (run and recorded by the assistant when the window works):
1. **No ROM argument:** prints usage and exits non-zero.
2. **Missing file, or a file that is not a ROM:** prints the core's error and exits non-zero.
3. **dmg-acid2:** a 640×576 window shows a crisp picture matching `reference-dmg.png`.
4. **Resizing:** the picture keeps a whole-number scale, centred and sharp; smaller than 1x it stays 160×144, centred.
5. **Closing the window:** exits with status 0.
6. **Speed:** on exit, the frontend prints frames run and elapsed time. After about 60 s the rate is 59.73 ± 0.1 frames per second.
7. **CPU:** the process sleeps between frames rather than spinning.
8. **Static linking:** the release `gameboy-sdl.exe` has no `SDL3.dll` dependency (`dumpbin /dependents`). Record whether `SDL3_DYNAMIC_API` can still override SDL.
