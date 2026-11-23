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
