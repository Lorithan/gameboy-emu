# LCD register proposal

Active M4 control/status contract, updated 2026-11-20. Video-memory foundation #71, the [eight passive data registers in #72](https://github.com/Lorithan/gameboy-emu/issues/72) and [#74, LCDC and an internal dot clock](https://github.com/Lorithan/gameboy-emu/issues/74), are implemented and verified against the provisional model below ([devices](../DEVICES.md#lcdc-and-internal-dot-clock), [testing](../TESTING.md)). **Gate change, agreed 2026-11-20:** live LY and STAT are now mapped on the provisional model and then calibrated against audited ROMs, because the calibration ROMs cannot run without readable LY and STAT. The split is:

- [#75 slice A](https://github.com/Lorithan/gameboy-emu/issues/75): live LY/STAT readback. **Next, specified below.**
- [#76 slice B](https://github.com/Lorithan/gameboy-emu/issues/76): STAT/VBlank interrupts.
- [#77 slice C](https://github.com/Lorithan/gameboy-emu/issues/77): CPU VRAM/OAM access restrictions.
- [#73 slice D](https://github.com/Lorithan/gameboy-emu/issues/73): calibration, required before any hardware-accuracy claim and before the M4 gate. User owns application implementation; assistant owns acceptance tests and review. The implemented passive contract remains in [devices](../DEVICES.md#passive-lcd-data-registers).

## Ownership and representation

Keep LCD state private to Ppu, with parent-visible accessors used by Bus. The CPU continues to issue ordinary reads/writes; System does not inspect PPU fields. No dependency or shared mutable ownership is needed.

- Passive registers are u8 fields. Use scy, scx, lyc, bgp, obp0, obp1, wy and wx, with read_NAME and write_NAME methods, like Timer's register methods.
- LCDC uses an LcdControl byte wrapper in lcd.rs, with infallible byte conversion, bits() and is_enabled() for current use. Store it in LcdRegisters, initially 0x91. Add other named bit helpers when a production consumer needs them; no flags dependency or unused hooks.
- Ppu owns private PpuTiming state, suggested in ppu/timing.rs. PpuMode represents mode, not the complete STAT byte. STAT later combines writable enables, mode and a clocked coincidence latch. Do not store another independently mutable LCD-enabled boolean.

## Register map and startup

This table specifies the DMG skipped-boot profile and implementation order. Register bytes are hexadecimal; clock counters and tick durations below are decimal.

| Address | Register | Startup read | Read and write contract | Slice |
| --- | --- | --- | --- | --- |
| FF40 | LCDC | 91 | Read all bits; writes replace configuration and apply enable/disable transitions below | Implement #74 |
| FF41 | STAT | 85 as the documented DMG readable profile | Bit 7 reads 1; bits 3-6 writable; mode and clocked comparison status | Readback #75; interrupts #76 |
| FF42 | SCY | 00 | Full-byte read/write | Implemented #72 |
| FF43 | SCX | 00 | Full-byte read/write | Implemented #72 |
| FF44 | LY | 00 as the documented DMG readable profile | PPU-generated; CPU writes ignored | Implement #75 |
| FF45 | LYC | 00 | Full-byte comparison configuration; later active writes reevaluate comparison | Implemented #72 |
| FF46 | DMA | Existing FF fallback for now | Separate Bus DMA contract; not a passive PPU byte | Later DMA |
| FF47 | BGP | FC | Full-byte read/write | Implemented #72 |
| FF48 | OBP0 | FF by project policy | Full-byte read/write | Implemented #72 |
| FF49 | OBP1 | FF by project policy | Full-byte read/write | Implemented #72 |
| FF4A | WY | 00 | Full-byte read/write | Implemented #72 |
| FF4B | WX | 00 | Full-byte read/write | Implemented #72 |

The [Pan Docs handoff table](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md) provides the documented DMG values, while identifying OBP0/OBP1 as uninitialized. Choosing FF for those two is a deterministic project policy, not a hardware guarantee. Keep the existing zeroed VRAM/OAM policy; no boot-ROM logo data is added.

LY00 and STAT85 describe a readable profile, not a measured hidden timing phase. STAT85 reports mode1 and coincidence. #74 chooses physical line153/dot4/VBlank as a representative internal seed, with no first-enable-line state pending. This is a project seed consistent with the intended profile, not an authentic boot handoff measurement. Its first normal line begins after452 ticks. Slice A reads it through a provisional line-153 visible-LY rule; #73 calibrates hidden phase and that rule. Do not equate physical line153 with a permanently visible LY153.

## Implemented passive register storage

The eight private fields are grouped in LcdRegisters under Ppu and initialized in dmg_post_boot using the table. Parent-visible Ppu read_NAME(&self) -> u8 and write_NAME(&mut self, value:u8) methods expose them to Bus. Each current writer stores its byte and returns unit; each reader returns it without effects. Bus maps exactly FF42, FF43, FF45 and FF47-FF4B. The gaps FF44 and FF46 retain fallback.

Keep every bit of all eight bytes. SCX/SCY, WX/WY and LYC accept all values without clamping. WX's display offset belongs to rendering, not register storage. Object color index0 is transparent during rendering; its unused shade selection does not justify erasing low bits from the stored OBP byte. These byte/readback rules are the proposed storage contract. [Scrolling](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Scrolling.md), [window coordinates](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Window.md), [palettes](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Palettes.md).

LYC remains passive because comparison and STAT signaling do not exist yet. Once that clock is implemented, active LYC writes must reevaluate comparison/interrupt effects; LCD-off writes retain the comparison latch. #74 maps only LCDC in addition to the existing passive bytes. FF41, FF44 and FF46 remain fallback, and video memory stays unrestricted until the mode/DMA contract is implemented. No temporary dead-code allowances or unused future accessors are required.

## Implement LCDC and internal progression

All LCDC bits are readable/writable. The proposed helper meanings follow the [DMG LCDC layout](https://raw.githubusercontent.com/gbdev/pandocs/master/src/LCDC.md):

| Bit | Meaning |
| --- | --- |
| 7 | LCD and PPU enabled |
| 6 | Window tile-map selection |
| 5 | Window enabled |
| 4 | Background/window tile-data addressing selection |
| 3 | Background tile-map selection |
| 2 | Object size selection |
| 1 | Objects enabled |
| 0 | Background/window enabled on DMG |

Add Ppu read_lcdc and write_lcdc, with exact Bus FF40 routing. Store all bits, but distinguish old/new bit7:

| Old enable | New enable | Clock effect after storing the byte |
| --- | --- | --- |
| 1 | 0 | Reset line/dot to0/0, mode HBlank, and discard first-enable-line state |
| 0 | 1 | Start line0/dot0/HBlank, marked as first line after enable |
| Same | Same | Preserve current line/dot/mode, including on repeated disabled writes |

Neither transition clears VRAM/OAM, passive registers, IF/IE, Timer or global elapsed time. #74 emits no PPU interrupt/frame effects. Timer continues when LCD is off. Ppu::tick returns unit and advances the private clock only when LCDC bit7 is set; disabled ticks retain the reset state. Do not add a framebuffer/blank-frame marker until there is a rendering consumer.

## Explicit provisional dot schedule

Use physical line0..153 and dot0..455. The state at dot0 is the line's initial state; each tick advances one dot and applies boundaries. At456, reset dot to0 and advance line, wrapping153 to0. Ordinary visible lines enter OamScan at dot0; the model uses:

| Ordinary visible line dots | Mode |
| --- | --- |
| 0-79 | OamScan |
| 80-251 | Drawing |
| 252-455 | HBlank |
| All dots of lines144-153 | VBlank |

The first line after enable initially reports HBlank instead of OamScan; in this provisional model it enters Drawing at80 and HBlank at252, ending at456. Retain its identity until the next line so calibration can change its timings independently. Later frame line0 is an ordinary OamScan line. There is no extra enabled flag in timing state; LCDC controls whether Ppu invokes its clock.

The fixed172-dot Drawing interval is a minimum-transfer scaffold without SCX/window/object stalls. The first-line80/172/204 schedule is also deliberately provisional: pinned [lcdon_timing-GS.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/lcdon_timing-GS.s), blob 3bcf2b2cb58eaa89b59ef75a46dd24dd84844966, explicitly observes distinct first-line timing and a two-T-cycle difference. These local constants are not a claimed hardware match. [Rendering reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Rendering.md). #73 must calibrate the CPU access projection and startup/enable behavior before live status; variable Drawing duration remains required for final rendering accuracy.

Bus calls Ppu::tick once inside each existing base-tick iteration, after handling that tick's Timer event. Keep the four device ticks before the CPU bus access: an LCDC write takes effect after its M-cycle's four ticks, without an extra tick. HALT and interrupt entry advance this clock; Stopped polls and input consume no ticks. Current StopEntered resets Timer's counter only, preserving/freeze-resuming PPU phase under our existing coarse STOP policy. No PPU IRQ, access lock, rendering, DMA or public debug API is part of #74. Child test modules can inspect private state; add only production-used interfaces.

## Slice A: live LY and STAT readback

[#75](https://github.com/Lorithan/gameboy-emu/issues/75) makes FF44 and FF41 readable from the #74 dot clock. It requests no interrupts and adds no access restrictions; reads and writes change no CPU timing. All constants remain the provisional #74 model.

### Proposed types and ownership

Interface names are fixed so tests can be written in advance; method bodies and LcdStatus internals are user choices.

| Item | Proposed shape | Responsibility |
| --- | --- | --- |
| `PpuMode::stat_bits(self) -> u8` | in timing.rs | HBlank 0, VBlank 1, OamScan 2, Drawing 3 |
| `PpuTiming::mode(&self) -> PpuMode` | read-only getter | Lets Ppu compose STAT without exposing fields |
| `PpuTiming::visible_ly(&self) -> u8` | derived, not stored | The LY the CPU sees; the single home of the line-153 rule |
| `LcdStatus` | in lcd.rs, beside LcdControl, held by LcdRegisters | Register state only: the STAT select bits and the stored LY=LYC flag; no timing knowledge |
| `Ppu::read_ly` / `read_stat` / `write_stat` | parent-visible, like the other register methods | Bus routes exactly FF41 and FF44; FF44 writes are ignored by Bus or a no-op, the user's choice |

Ppu coordinates, as in write_lcdc. `Ppu::tick` advances timing only while LCDC bit 7 is set, then updates the LY=LYC flag from `visible_ly()` and LYC. While the LCD is off nothing updates the flag.

### Behaviour

- **Visible LY:** on lines 0-152 it equals the physical line. On line 153 it reads 153 at dots 0-3 and 0 at dots 4-455. This window is provisional; Pan Docs says only that LY "starts reporting 00 only a few dots into" line 153, and the [GBEDG PPU guide](https://hacktix.github.io/GBEDG/ppu) gives roughly four T-cycles while warning that it may be inaccurate. #73 calibrates it. With the LCD off, timing sits at line 0, so LY reads 0 without a special case.
- **FF44 writes** are ignored: no stored value, timing or flag changes.
- **STAT read:** `0x80 | select | flag << 2 | mode.stat_bits()`. With the LCD off, mode is HBlank, so bits 0-1 read 0, as Pan Docs requires.
- **STAT write:** stores `value & 0x78`. Bits 0-2 and 7 cannot be written. No interrupt or DMG write quirk in this slice.
- **LY=LYC flag:** stored, not computed on read. It is updated after every enabled tick, so an LYC write is reflected by the next enabled tick, which precedes any later CPU access. It holds while the LCD is off, across LYC writes and across disabling, as [stat_lyc_onoff](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/stat_lyc_onoff.s) requires. On enable it holds until the first enabled tick compares LY 0. Whether active LYC writes update it immediately is settled in slice B, where it becomes observable through interrupts.
- **Startup:** the flag starts true. The seed (line 153, dot 4, VBlank) gives visible LY 0, LYC 0, so FF44 reads 00 and FF41 reads 85, matching the documented DMG values without hard-coding the bytes. The first enabled tick recomputes the same true value.

### Sampling note

CPU accesses happen after the M-cycle's four ticks. From the seed, samples fall on dots that are multiples of 4, so dot 0 of line 153 is sampled and LY 153 is visible to exactly one sample per frame. After an enable write, sampling also falls on multiples of 4 from line 0 / dot 0. Phase relative to hardware is a #73 target.

### Reporter impact

The pinned helpers below were re-audited for slice A:

- **[is_ppu_broken](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s):** with LY below 160, the PPU is treated as working, so every existing fixture switches to the on-screen result path in [quit](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s).
- **quit sequence:** disable_ppu_safe waits for LY 144 and then turns the LCD off. Then reset_screen, the font load and the callback's printing write VRAM, all with the LCD off, so slice C restrictions should not affect them. Finally it re-enables the LCD and waits for LY 143, 144, 143 and 144, before the register signature and serial bytes.
- **Silent timeouts:** each [wait_ly_with_timeout](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/wait_ly_with_timeout.s) iteration costs 13 M-cycles, so a timeout costs 851,968 M-cycles. A timeout returns normally, and quit still reports the fixture's own result. **A pass therefore does not prove LY works.**
- **Expected counts:** before slice A, 16 fixtures took 3,943-4,677 M-cycles, div_write 721,229 and ie_push 852,459, the last because its LY wait timed out. After slice A, each fixture should add roughly three frames of waits (about 53,000 M-cycles) plus screen setup. ie_push should drop sharply because its wait now completes.
- **Timeout bound:** any fixture other than div_write that reaches 851,968 M-cycles signals a timeout. div_write with a timeout would exceed the existing 1,000,000 cap. Caps are unchanged.

## STAT interrupts (slice B, #76)

Use the four PpuMode values HBlank=0, VBlank=1, OamScan=2 and Drawing=3. Later store only writable STAT interrupt-enable bits, masked with 0x78. Readback combines 0x80, those enables, the clocked coincidence latch at bit2 and the reported mode. CPU writes cannot set mode/coincidence or bit7. LY is generated from timing, with physical line and visible LY distinguished. [STAT layout](https://raw.githubusercontent.com/gbdev/pandocs/master/src/STAT.md); [unused bit7 hardware test](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/unused_hwio-GS.s).

**Correction to the earlier proposal:** coincidence is real retained state, not always a pure expression of current LY/LYC. Pinned [stat_lyc_onoff.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/stat_lyc_onoff.s), blob 6d8a34fd0978bd6a79370a07dc830c870bfcf152, holds that latch while LCD is off, even across LYC writes. Enable resumes comparison with visible LY0. Its four cases require:

| Retained comparison | Comparison after enable | Expected comparison interrupt in that source |
| --- | --- | --- |
| True | False | None |
| True | True | None |
| False | False | None |
| False | True | Requested |

Thus disabling must not recompute coincidence just because visible LY becomes0, and resetting edge history indiscriminately would create a spurious true-to-true interrupt. Slice B (#76) settles interrupt sample order and combined-source history against these cases before implementation; slice A adds the held flag only.

STAT sources share one logical line: enabled HBlank, VBlank, OamScan and coincidence conditions are ORed. Request LCDStat on that combined line's low-to-high transition, not on every tick or separately for overlapping sources. Both its edge history and the clocked comparison latch are meaningful state. VBlank is a separate request. Proposed later PpuEffects has independently named vblank and lcd_stat flags, allowing both together; Bus applies them independently of IE/IME. This is not a mutually exclusive event enum. [Interrupt sources and STAT blocking](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupt_Sources.md).

Active changes to mode/LY, LYC and STAT enables can change the line; control writes can also have effects. Later affected writes and ticks return effects to Bus. #76 must settle precise interrupt sample order, LCD-off gating, startup/no-synthetic-event initialization and DMG's spurious STAT-write behavior. CPU memory-access restrictions are #77; first blank-frame policy belongs to rendering. These are required, not silently omitted accuracy claims.

## Timing and ROM integration gate

#74 kept STAT/LY unmapped while implementing internal progression and LCDC effects (verified at 49b8dd4, 508 tests). Under the revised gate, slices A-C publish live behaviour on the provisional model; #73 (slice D) then calibrates phases, status/interrupt/write ordering and access rules against audited PPU ROMs. The live checkpoint must actually run the real reporter path, with timeouts ruled out by M-cycle counts.

Pinned Mooneye [is_ppu_broken.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s), blob 03facecd5d67ebf4902e6eef122d9e95165b5cf1, treats LY below 160 as usable. [quit.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s) then invokes video setup and waits. [disable_ppu_safe.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/disable_ppu_safe.s), blob 153c2b8ab503f6e8dcc4c911753a706437b1580b, checks LCDC bit7 and waits for LY144 before disabling; its [wait helper](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/wait_ly_with_timeout.s), blob ae26cb6c8e879d7a7a7fc08f0a16bc06f3603e18, bounds a wait by a wrapping 16-bit loop. LCDC mapping alone can change helper execution/counts even while LY remains FF. #74 must preserve existing finite caps and report any real helper timeout/failure; do not assume unchanged counts or add fixture-specific overrides. Mapping live LY requires renewed font/screen-helper and reporting audit. Keeping LY/STAT unmapped is only this named intermediate checkpoint, not the final design.

## Acceptance and checkpoints

For #72 four new tests cover all-byte read/write round trips and startup, read purity, exact address/gap decoding, per-register/per-instance/video-memory/Timer/IF/IE isolation, and CPU LDH actions/ticks/snapshots. Expanded existing LDH checks cover both addressing forms and every pause budget; fallback/high-I/O expectations change explicitly for only these eight addresses, preserving unrelated memory/flag/trace/fault/budget oracles. Full regression and fixture evidence are recorded in [testing](../TESTING.md).

For #74 the assistant will add all-byte LCDC read/write and old/new-enable tests; independent model dot/mode/line/frame boundary and wrap oracles; the 452-tick constructor tail; first-enabled-line distinction; disabled freeze/restart/idempotence; full-memory/device/instance isolation; CPU LDH traces/post-advance write timing/budget pauses; and HALT/entry/STOP progression checks. Explicitly migrate FF40 expectations and retain the other status/DMA gaps. Require the full suite and 18 existing hash-verified ROMs, formatting and strict Clippy. No application changes or tests were run for this contract update.

#74 verification is recorded in [testing](../TESTING.md).

### Slice A acceptance (#75)

The assistant writes these tests against the interface above.

**Timing level**
- `visible_ly()` for every dot of two frames from enable and from the seed, against an independent model.
- `mode()` agrees with the #74 model.
- `stat_bits()` for all four modes.

**Bus level**
- Startup reads LY 00 / STAT 85, and reads have no effects.
- Every FF41 byte write reads back `0x80 | value & 0x78 | flag | mode`.
- Every FF44 byte write is ignored, with full-memory, timing and flag isolation.
- At every M-cycle over two frames, LY and STAT match a model for LYC values 0, 1, 143, 144, 153 and FF.
- The single LY 153 sample per frame.

**LCD off/on**
- While off, LY reads 0 and mode reads 0.
- The flag holds across LYC writes and across disabling, in all four stat_lyc_onoff states.
- After enable, the first enabled tick compares LY 0.

**CPU and System level**
- LDH reads of FF41/FF44 return the values for the exact M-cycle.
- Budget pauses do not change results.
- No IF change over several frames with every STAT select bit set (this slice has no interrupts).

**Migration and gate**
- FF41/FF44 move out of the fallback lists; FF46 stays fallback.
- All 18 existing fixtures pass, with per-fixture M-cycle counts recorded and checked against the timeout bound above.
- Full suite, fmt and strict Clippy pass.

Rendering, frame publication and DMA remain separate. #73 is required before the M4 gate and any hardware-accuracy claim; it must revisit the documented scaffold instead of presenting these constants as hardware certification.
