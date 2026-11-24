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
2. **The demo ROM:** chosen 2026-11-23. MaxPirate v1.0 (MIT) is the demo, and GB Snake (WTFPL) covers Select for the F3 checklist. Both are pinned in [demo-roms.json](../../tests/fixtures/demo-roms.json).

## Slices

Each is a separate issue with acceptance criteria written first.

| Slice | Scope | Evidence |
| --- | --- | --- |
| F1 [#85](https://github.com/Lorithan/gameboy-emu/issues/85), complete 2026-11-22 | Workspace split, core library-only, empty `gameboy-sdl` binary | `cargo build --workspace`; full suite, fmt, strict Clippy across the workspace; all seven fixture scripts pass with identical counts |
| F2 [#86](https://github.com/Lorithan/gameboy-emu/issues/86), complete 2026-11-22 | SDL3 window shows frames from a ROM given on the command line, with pacing | Unit tests for RGBA conversion and pacing; dmg-acid2 shown matching its reference; the checklist is run and recorded |
| F3 [#87](https://github.com/Lorithan/gameboy-emu/issues/87), complete 2026-11-23 | Keyboard input | Unit tests for the key map; a manual check with a ROM that reacts to buttons |
| F4 [#88](https://github.com/Lorithan/gameboy-emu/issues/88), complete 2026-11-23 | Demo ROM chosen, pinned and recorded | Provenance and licence recorded first; the demo is played through the checklist |
| F5 [#89](https://github.com/Lorithan/gameboy-emu/issues/89) | Open a ROM from the window: picker screen, native file dialog, drag-and-drop, Ctrl+O, errors in the window, no console in release | Unit tests for pure layout and hit-test functions; the F5 checklist run and recorded |

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

## F2 result (complete 2026-11-22)

The user implemented `display`, `pacing` and the SDL loop in `main.rs` against the contract above. All 15 unit tests pass; the workspace has 626 tests (611 core, 15 frontend) passing with `--include-ignored`, and fmt and strict Clippy pass. The checklist:

| Item | Result | Checked by |
| --- | --- | --- |
| 1. No argument | Usage on stderr, exit code 2 | assistant |
| 2. Missing file / not a ROM | `could not read cartridge at does-not-exist.gb` / `expected header minimum: 336 bytes, actual size: 132 bytes`, exit code 1 | assistant |
| 3. dmg-acid2 | Sharp picture matching the reference at 640×576 | user |
| 4. Resizing | Whole-number scale, centred and sharp, with black borders, including below 1x | user |
| 5. Closing the window | Exit code 0, with the frame count printed | assistant |
| 6. Speed | `frames 3559 in 59.59 s (59.73/s)` with dmg-acid2 in release | assistant |
| 7. CPU | 12.0 s of CPU in 60.0 s wall, 20% of one core: sleeps, does not spin | assistant |
| 8. Static linking | `dumpbin /dependents` lists no `SDL3.dll`, only Windows system libraries and the MSVC C runtime (`VCRUNTIME140.dll`); the `SDL3_DYNAMIC_API` hook is compiled in, but an override was not exercised | assistant |

**Notes:**
- **CPU use is higher than emulation needs.** The core runs about 30 times real time in release, so emulation accounts for about 3% of a core; the rest is probably rendering and presenting, or per-cycle overhead. It is not a failure; profiling would be a separate issue if needed.
- **Build time.** SDL3 builds from source in about 41 s (debug) or 50 s (release) the first time, and is cached afterwards.

## F3 contract (agreed 2026-11-22, [#87](https://github.com/Lorithan/gameboy-emu/issues/87))

**Agreed design.**
- **Two key sets, both active.** WASD and the arrow keys for the D-pad; J/K and Z/X for B/A (B left of A, as on the Game Boy); Enter for Start; Right Shift or Backspace for Select. Esc stays free for quitting.
- **Physical keys.** Bindings use SDL scancodes, so WASD keeps its physical position on any keyboard layout.
- **A binding table, not a `match`.** Rebinding can later replace the table without touching the event loop. Rebinding itself is out of scope.
- **Last press wins on each D-pad axis.** With both Left and Right (or Up and Down) held, the one pressed most recently is active; releasing it hands the axis back to the other if that is still held. Remembering the last direction pressed on each axis is enough.

**Later: controllers** (not in scope; noted 2026-11-22).
- **One input for all devices.** Gamepads reuse `Input` through an enum key type, such as `Source::Key(Scancode)` or `Source::Pad(GamepadButton)`, with **one** `Input` for every device. Then holds and last-press-wins work across devices, and one device's release cannot cancel another's hold. Separate `Input`s per device would have exactly that bug.
- **Analog input.** Sticks and triggers need a small converter with a dead zone and hysteresis that produces virtual key-downs and key-ups.
- **Device handling.** Several pads need the device ID in the source. Unplugging a pad needs a per-device release, like `release_all`.

**Core change (user):** `Button` derives `PartialEq, Eq` (and `Hash` if wanted), so bindings and tests can compare buttons.

**Testable interface** (pure, no SDL calls; the logic is generic over the key type so a web frontend can reuse it):

```rust
// frontends/sdl/src/input.rs
pub const DEFAULT_BINDINGS: [(Scancode, Button); 15];       // the 15 bindings above
pub struct InputEvent { pressed: Option<Button>, released: Option<Button> } // Debug, PartialEq, Eq
impl InputEvent { pub fn apply(self, system: &mut System); } // the release, then the press
pub struct Input<K>;
impl<K: Copy + Eq + Hash> Input<K> {
    pub fn new(bindings: impl IntoIterator<Item = (K, Button)>) -> Self; // panics if a key is bound twice
    pub fn key_down(&mut self, key: K) -> InputEvent;
    pub fn key_up(&mut self, key: K) -> InputEvent;
    pub fn release_all(&mut self) -> Vec<Button>;            // focus loss: forget every held key
}
```

**Rules.**
- **Held buttons.** A button is held while any key bound to it is held. Unbound keys, a key-down for a key already held (SDL key repeat), and a key-up for a key not held change nothing.
- **Recency.** For the last-press-wins rule it follows the button, not the key: it updates only when a direction goes from no keys held to at least one.
- **Reported changes.** Each call reports exactly the buttons whose effective state changed. One key event changes at most one release and one press, so `InputEvent` holds one of each; `InputEvent::apply` passes `released` and then `pressed` to `System::set_button`. The fields are private, so nothing else can apply them in the wrong order. `release_all` returns every button it released. (Revised 2026-11-23 from a `Vec<(Button, bool)>` to the user's `InputEvent`.)
- **Focus loss.** On `WindowFocusLost`, the loop calls `release_all`, because SDL may never deliver the key-ups.

**Unit tests** (assistant, 2026-11-22): `frontends/sdl/src/input/tests.rs` (10), wired by `#[cfg(test)] mod tests;`.
- **Default table:** the full table, every button reachable, Esc unbound.
- **Basics:** duplicate keys rejected; press and release; unbound keys, repeats and stray releases.
- **Rules:** buttons held by either of two keys; last press wins in both directions, including releasing the loser; axes and action buttons independent; recency per button; `release_all`.
- **Random sequences:** 20 seeds of 2,000 events each against an independent model, checking order, uniqueness, no-op changes and the reported state, with `release_all` every 500 events.
- **Validation.** In a scratch worktree, all 10 fail against stubs. A reference passes all 10, and each of 12 deliberate breaks fails at least one:
  - recency per key press; repeats updating recency; duplicate keys allowed;
  - first press wins; no rule; both-held-means-neither;
  - presses before releases; one key releasing a two-key button; `release_all` keeping keys;
  - Backspace missing; Z bound to A; axes crossed.

**Manual checklist** (run with a ROM that reacts to buttons; the F4 demo, or a test ROM):
1. **Each binding:** WASD, the arrows, J/K, Z/X, Enter, Right Shift and Backspace each act as their button.
2. **Opposite directions:** holding Left then pressing Right moves right; releasing Right moves left again.
3. **Repeat:** holding a key does not re-trigger it.
4. **Focus loss:** switching windows while holding a key leaves nothing stuck.
5. **Quitting:** Esc still quits.

## F3 and F4 result (complete 2026-11-23)

The user played MaxPirate v1.0 and GB Snake in the release frontend and reported no problems. Both games played and controlled correctly. The F3 checklist passed:

| Item | Result |
| --- | --- |
| 1. Each binding | WASD, arrows, J/K, Z/X, Enter, Right Shift and Backspace act as their buttons (Select through Snake) |
| 2. Opposite directions | Last press wins; releasing it hands the axis back |
| 3. Repeat | Holding a key does not re-trigger it |
| 4. Focus loss | Nothing stuck after switching windows |
| 5. Quitting | Esc quits |

Automated evidence: 10 input unit tests, plus 2 headless demo smoke tests that fail with the button presses disabled. MaxPirate is the agreed ROM-only interactive demo for the M4 gate.

## F5 contract: open a ROM from the window (agreed 2026-11-23, [#89](https://github.com/Lorithan/gameboy-emu/issues/89))

Proposed by the user so the emulator can be launched from Explorer without a terminal. It is in M4 because it completes the demo as a launchable program. No new dependencies are needed.

**Design (agreed).**
- **`App`** owns everything SDL: context, canvas, texture, event pump, input and the RGBA buffer. It also holds a `Screen`: either `Picker { error: Option<String> }` or `Running(Session)`.
- **`Session`** is one loaded game: the `System`, a fresh `Pacer`, the last published frame count, the frame statistics and the ROM name. Loading a ROM replaces it.
- **Starting.** `App::new()` sets up SDL; `app.run(rom: Option<PathBuf>)` starts the game if a path was given, otherwise shows the picker. The command-line argument stays, because Explorer passes the file path that way for "Open with", file associations and dropping a file onto the `.exe`. Automated checks also use it.
- **One load path.** `App::load(path)` serves the argument, the dialog and drag-and-drop:
  - **On success:** it builds a new `Session` and releases every held key (`Input::release_all`), so nothing carries over between games. The window title shows the ROM name.
  - **On failure:** it returns to the picker and shows the error in the window. Nothing is printed to the console and nothing exits.
- **The dialog.** `sdl3::dialog::show_open_file_dialog` uses a `.gb`/`.gbc` filter. Its callback may run on another thread and must be `Send`, so it sends the chosen path through a `std::sync::mpsc` channel. The main loop drains that channel each iteration. Cancelling the dialog changes nothing.
- **Picker screen.** A centred "Open ROM" button and a hint to drop a file, drawn with `draw_debug_text` (SDL's built-in 8×8 font), plus the last error if there is one.
- **Controls:**
  - in the picker: click the button, or press Enter or O, to open the dialog;
  - anywhere: drop a file onto the window to load it, or press Ctrl+O to open the dialog;
  - Esc in a running game returns to the picker, ending that session and releasing held keys. Esc in the picker does nothing (agreed 2026-11-23);
  - closing the window or Alt+F4 quits from either screen. An exit button on the picker is optional.
- **Coordinate spaces.** Mouse positions arrive in window coordinates, while `output_size()` is in pixels. On a high-DPI display the two differ, so the button's hit test and its drawing must use the same space. Either convert the mouse position, or lay out in window coordinates with `window.size()`.
- **Release builds have no console.** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` removes the console window when launched from Explorer. Errors therefore go to the screen. The frame-rate line stays on stderr and is visible only in debug builds or a terminal.
- **Future: a config file.** Settings such as bindings, scale, palette and the last folder are gathered in one `Settings` value with defaults. Reading them from a file is later work and needs a parser dependency decision then.

**Testable parts.** If the user keeps them as small pure functions in the frontend:
- the button's rectangle for a given window size: centred, and fitting small windows;
- the hit test, including its edges.

The names are agreed when the module exists.

**Manual checklist:**
1. **Launching from Explorer:** double-clicking the release `gameboy-sdl.exe` opens the picker with no console window.
2. **The button:** clicking it, or pressing Enter or O, opens the file dialog filtered to `.gb`/`.gbc`. Choosing MaxPirate starts it; cancelling leaves the picker unchanged.
3. **Errors:** choosing a non-ROM file shows the error in the window, and the picker still works afterwards.
4. **Drag and drop:** dropping a ROM onto the picker starts it; dropping one onto a running game switches to it.
5. **Switching games:** Ctrl+O while playing opens the dialog and switches games, at full speed from the first frame, with no stuck keys.
6. **The argument:** `gameboy-sdl <rom>` and dropping a ROM onto the `.exe` both start that game directly.
7. **Esc:** Esc while playing returns to the picker with no stuck keys. Loading a game afterwards starts it fresh. Esc in the picker does nothing.
8. **Earlier checks:** the F2 and F3 checklist items still pass: speed, scaling, keys and closing the window. Esc no longer quits.
