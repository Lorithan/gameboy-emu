# LCD register proposal

Active M4 control/status contract, updated 2026-11-20. Video-memory foundation #71, the [eight passive data registers in #72](https://github.com/Lorithan/gameboy-emu/issues/72) and [#74, LCDC and an internal dot clock](https://github.com/Lorithan/gameboy-emu/issues/74), are implemented and verified against the provisional model below ([devices](../DEVICES.md#lcdc-and-internal-dot-clock), [testing](../TESTING.md)). **Gate change, agreed 2026-11-20:** live LY and STAT are now mapped on the provisional model and then calibrated against audited ROMs, because the calibration ROMs cannot run without readable LY and STAT. The split is:

- [#75 slice A](https://github.com/Lorithan/gameboy-emu/issues/75): live LY/STAT readback. Implemented and verified 2026-11-20; see [testing](../TESTING.md#live-lystat-75).
- [#76 slice B](https://github.com/Lorithan/gameboy-emu/issues/76): STAT/VBlank interrupts. Agreed, implemented and verified 2026-11-20; see [devices](../DEVICES.md#stat-and-vblank-interrupts) and [testing](../TESTING.md#stat-and-vblank-interrupts-76). **Next: #77.**
- [#77 slice C](https://github.com/Lorithan/gameboy-emu/issues/77): CPU VRAM/OAM access restrictions. [Contract](#slice-c-cpu-vram-and-oam-access-restrictions-77) drafted 2026-11-20, awaiting agreement.
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
| FF41 | STAT | 85 as the documented DMG readable profile | Bit 7 reads 1; bits 3-6 writable; mode and clocked comparison status | Readback #75; interrupts #76 (done) |
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

**Status:** implemented and verified 2026-11-20 (520 tests, all 18 fixtures). One clarification found in review: STAT bit 7 is an unused bit that always reads 1, independent of LCDC bit 7. Measured reporter costs exceeded the estimate below (about 179,000 added M-cycles per fixture, not 53,000), with every LY wait completing.

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

## Slice B: STAT and VBlank interrupts (#76)

Agreed 2026-11-20, with the three provisional decisions in points 4-5 and the line-144 pulse as drafted; implemented and verified the same day. It adds interrupt requests on top of the slice A registers; register readback changes only through the immediate LYC comparison in point 4. All timing remains the provisional model; #73 calibrates it.

### Sources

| Source | What it establishes |
| --- | --- |
| [Pan Docs interrupt sources](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupt_Sources.md) | VBlank is requested on entering mode 1 at LY 144. STAT sources (modes 0-2, LY=LYC) are ORed into one line, gated by their select bits. A request happens only on the line's rising edge, so overlapping sources block each other (e.g. modes 0 and 1 both selected: no separate mode-1 interrupt). LYC 0 matches early in line 153. |
| [Pan Docs STAT](https://raw.githubusercontent.com/gbdev/pandocs/master/src/STAT.md) | The comparison runs constantly. On DMG, writing STAT during OAM scan, HBlank, VBlank or LY=LYC can request an interrupt: it "behaves as if $FF were written for one M-cycle, and then the written value were written the next M-cycle." |
| [stat_lyc_onoff.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/stat_lyc_onoff.s) | Table below. Every LYC write in it happens with the LCD off, so it does **not** settle LYC writes while the LCD is on. |
| [stat_irq_blocking.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/stat_irq_blocking.s) | Round 1: already in VBlank, writing STAT `$10` must request an interrupt. Round 2: with STAT `$78`, sweeping LYC over 0-143 must never request one, because only mode 3 lowers the line. |
| [vblank_stat_intr-GS.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/vblank_stat_intr-GS.s) | On DMG, the **mode-2** select (bit 5) also triggers at line 144, at the same time as VBlank. It does not test the mode-1 select. |

### Proposed types and ownership

| Item | Proposed shape | Responsibility |
| --- | --- | --- |
| `PpuEffects` | `#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]` struct with `vblank: bool` and `lcd_stat: bool` | What the PPU asks Bus to request. Independent flags, both possible at once; not an enum. |
| `Ppu::tick(&mut self) -> PpuEffects` | changed return type | Advances timing, updates the flag, evaluates the line |
| `Ppu::write_stat` / `write_lyc` `-> PpuEffects` | changed return type | Writes that can raise an edge immediately |
| STAT line history | one stored `bool`, owned with LcdStatus or Ppu (your choice) | Previous combined line level, used for edge detection |
| Bus | applies effects in `advance_mcycle` and in its FF41/FF45 write arms | `request_interrupt(VBlank)` / `request_interrupt(LCDStat)`. Never inspects IE/IME; CPU dispatch is unchanged. |

`PpuTiming` stays free of register knowledge, as in slice A. The line is a function of the selects, the flag and the timing state, so put it on whichever side owns those inputs. One natural shape is an `LcdStatus` method that takes a small timing view (mode, plus whether this is the line-144 mode-2 pulse).

### Behaviour

**1. STAT line (provisional, needs calibration).** It is the OR of four sources:

- **HBlank:** HBlank select and mode HBlank.
- **VBlank:** VBlank select and mode VBlank.
- **OAM:** OAM select and (mode OamScan, or the line-144 pulse).
- **LY=LYC:** LYC select and the flag.

The **line-144 pulse** is dot 0 of physical line 144 (provisional, after vblank_stat_intr-GS). The first line after enable reports HBlank, not OamScan, so it has no mode-2 source.

**2. Evaluation points.** The line is re-evaluated after every enabled tick, after the flag update. LCDStat is requested when the new level is high and the stored level is low; the new level is then stored.

**3. VBlank request.** Requested on the enabled tick that enters physical line 144 / dot 0. It ignores the STAT selects and the line, and happens once per frame.

**4. LYC writes while the LCD is on (provisional).** The flag and line are re-evaluated immediately, and the write may return `lcd_stat`. Pan Docs says the comparison runs constantly, and no source contradicts an immediate update. **This changes slice A, which compared only on ticks.** CPU reads cannot tell the difference, so slice A's tests still hold.

**5. STAT writes while the LCD is on (DMG quirk).** The line is first evaluated with all four selects as if `$FF` were written, which may request; then the real selects are stored and the line is evaluated again, without requesting a second time on the same write. In the provisional model this collapses the quirk's "one M-cycle" into the write. That satisfies stat_irq_blocking round 1 with or without the quirk. Its exact duration is a #73 target.

**6. LCD off.**

- There are no ticks, so there are no effects and no evaluations.
- STAT/LYC writes while off store their values, but do not evaluate or request; the quirk does not apply while off.
- The stored line level and the flag are both held. This is what gives stat_lyc_onoff its results:
  - Round 2: true to true after enable, so no request.
  - Round 4: false to true at the first enabled tick, so a request.
- Neither LCDC edge requests anything at the moment of the write.

**7. Startup.** The stored line level starts at the level the seed would produce, which is low with STAT selects 0. Nothing is requested at construction, and the seed is mid-VBlank, so the first VBlank request is at the next line 144.

### stat_lyc_onoff cases (also pinned by the slice A flag behaviour)

| Retained comparison | Comparison after enable | Expected comparison interrupt in that source |
| --- | --- | --- |
| True | False | None |
| True | True | None |
| False | False | None |
| False | True | Requested |

Disabling must not recompute the flag just because visible LY becomes 0, and resetting the line history on enable would create a spurious true-to-true interrupt.

### Interrupt sampling in our model

- Effects from ticks set IF during `advance_mcycle`, before that cycle's bus access, so the CPU's post-advance interrupt check sees them in the same step.
- Effects from a write set IF after the access, so they are seen at the next step's first check.
- Whether the write-time choice in points 4-5 is observably different from waiting for the next tick will be checked by tests. Hardware phase is #73.

### Out of scope

- VRAM/OAM access restrictions (#77).
- Calibrating any constant (#73).
- Running stat_irq_blocking, vblank_stat_intr or stat_lyc_onoff as fixtures; they need the #73 audit first. Their conditions are reproduced as local tests instead.
- DMA, rendering.

### Slice B acceptance

**Line and effects model**
- For all 16 select combinations × LYC values {0, 1, 143, 144, 153, FF}, over two frames from the seed and from enable, `Ppu::tick` effects match an independent line/edge model at every tick.
- Spot counts per frame:
  - HBlank select only: 144.
  - OAM only: 145, including the line-144 pulse.
  - VBlank only: 1.
  - LYC only: 1 (0 for LYC FF).
  - HBlank and VBlank: 144, because the VBlank source is blocked behind HBlank.
- VBlank effect exactly once per frame at line 144 / dot 0, with any selects.

**Writes**
- LYC writes while on update the flag immediately and request only on a rising edge.
- STAT-write quirk: `$00` written in HBlank / VBlank / OAM scan or with the flag set, while the line is low, requests once. Written in Drawing with the flag clear, it does not.
- stat_irq_blocking round 1 and round 2 conditions reproduced through Bus.

**LCD off/on**
- No effects while off, for any writes.
- The four stat_lyc_onoff rounds reproduced through Bus IF bit 1.
- No request at construction or in the first M-cycles.

**CPU and System level**
- With IE set, the VBlank and STAT interrupts wake HALT and dispatch to 0040 / 0048 at the expected M-cycle.
- With IME off, only IF changes.
- Interrupt requests from writes are seen one step later than those from ticks, as described above.

**Gate**
- All 18 existing fixtures pass, with per-fixture M-cycle counts recorded. PPU interrupts now set IF bits 0-1 during fixtures, so if_ie_registers and similar tests are the ones to watch.
- Full suite, fmt and strict Clippy pass.

### History

**Correction to the earlier proposal:** coincidence is real retained state, not always a pure expression of current LY/LYC. Pinned [stat_lyc_onoff.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/stat_lyc_onoff.s), blob 6d8a34fd0978bd6a79370a07dc830c870bfcf152, holds that latch while LCD is off, even across LYC writes. Enable resumes comparison with visible LY0. Its four cases are tabulated in the slice B contract above. CPU memory-access restrictions are #77; first blank-frame policy belongs to rendering.

## Slice C: CPU VRAM and OAM access restrictions (#77)

Proposed contract, drafted 2026-11-20 and awaiting agreement. It restricts ordinary CPU accesses through Bus by PPU mode. All timing remains the provisional model; #73 calibrates it.

### Sources

| Source | What it establishes |
| --- | --- |
| [Pan Docs: accessing VRAM and OAM](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Accessing_VRAM_and_OAM.md) | VRAM (8000-9FFF) is accessible in modes 0-2, so blocked in mode 3. OAM (FE00-FE9F) is accessible in modes 0-1, so blocked in modes 2-3. Blocked reads return "undefined data (typically $FF)"; blocked writes "are ignored (data remains unchanged)". "While the display is disabled, both VRAM and OAM are accessible." |
| [Pan Docs: memory map](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Memory_Map.md) | FEA0-FEFF reads FF while OAM is blocked; on DMG, reads otherwise return 00, and reads while blocked trigger OAM corruption. Writes are not described. |
| [Pan Docs: OAM corruption bug](https://raw.githubusercontent.com/gbdev/pandocs/master/src/OAM_Corruption_Bug.md) | On DMG, accesses (and 16-bit inc/dec of a register) in FE00-FEFF during mode 2 corrupt OAM. |
| [lcdon_timing-GS.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/lcdon_timing-GS.s) | DMG-verified. After enable, "line 0 starts with mode 0 and goes straight to mode 3", and OAM and VRAM are both readable early in line 0, then blocked together when mode 3 starts. Line 0 is "late by 2 T-cycles"; lines 1-2 are normal. |
| [lcdon_write_timing-GS.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/ppu/lcdon_write_timing-GS.s) | DMG-verified write samples after enable. Its table suggests OAM writes become blocked a little earlier than OAM reads around the mode 0 to 2 edge, while VRAM writes follow mode 3. Calibration only. |

### Proposed behaviour

1. **Rule (provisional).** While the LCD is on, a CPU access is blocked when the mode reported by `PpuTiming::mode()` at the moment of the access is:
   - VRAM: Drawing.
   - OAM: OamScan or Drawing.
   The first line after enable reports HBlank for dots 0-79, so OAM is accessible there, which matches lcdon_timing-GS qualitatively.
2. **Blocked read:** returns FF and has no effect. **Blocked write:** ignored; storage is unchanged.
3. **LCD off:** never blocked. Disabling the LCD mid-Drawing makes both accessible from the next access.
4. **Sampling:** CPU accesses happen after the M-cycle's four ticks (existing schedule), so the access sees the mode after those ticks. Hardware read/write edge differences (lcdon_write_timing-GS) are #73 targets.
5. **FEA0-FEFF (proposal: unchanged).** Keep the existing FF read / ignored write fallback in every mode. The DMG 00 read and corruption behaviour are recorded deviations, deferred to a separate issue with the OAM corruption bug. Implementing only the 00 read without corruption would be a half-model.
6. **No other effects:** blocked accesses do not change LY, STAT, IF, timing or other memory.

### Proposed types and ownership

- **Ppu owns the rule.** The VRAM/OAM accessors Bus already calls become the CPU-facing, mode-checked path. Bus keeps mapping addresses and does not inspect PPU mode.
- **Derive, don't store.** Accessibility is a function of the LCDC enable bit and the current mode; no separate lock booleans that could drift from timing.
- **Unrestricted access stays internal.** Future rendering reads the arrays inside Ppu; future OAM DMA gets its own path when DMA is specified. Don't add those paths now.
- Naming (keep `read_vram`/`write_vram`, or rename to make "CPU access" explicit) and whether the checks live in two small query methods are your choice.

### Out of scope

- OAM corruption bug; FEA0-FEFF DMG behaviour (point 5).
- OAM DMA and its OAM-blocking (later DMA contract).
- Rendering's own VRAM/OAM reads; CGB VRAM banking/HDMA.
- Calibrating any edge (#73), including running lcdon_timing-GS or lcdon_write_timing-GS as fixtures.

### Slice C acceptance (proposed)

**Bus level, against an independent mode model**
- From the seed and from enable, for every M-cycle of two frames: reads at 8000, 9FFF, FE00, FE9F and interior addresses return the stored byte when accessible and FF when blocked; writes land exactly when accessible. Blocked writes are confirmed unchanged by reading later with the LCD off.
- Both blocked and allowed sets are non-empty for each region (non-vacuity), and the first line after enable allows OAM in dots 0-79.
- LCD off: every byte accessible at every M-cycle; disable mid-Drawing unblocks the next access.
- FEA0-FEFF reads FF and ignores writes in every mode.
- Blocked accesses leave LY, STAT, IF, timing probe and all other memory unchanged.

**CPU and System level**
- CPU loads/stores to VRAM and OAM (e.g. `LD (HL),A`, `LD A,(HL)`, `LD (a16),A`, `PUSH` into OAM) observed against a reference Bus advanced in lockstep: trace read values, ignored stores and final memory match the mode after each access cycle's ticks.
- Instruction-budget pauses do not change which accesses are blocked.

**Migrations**
- Existing tests that access VRAM/OAM with the LCD on after the seed's VBlank tail (113 M-cycles) may now hit blocked phases. Each is migrated explicitly, by disabling the LCD in setup or choosing an accessible phase, without weakening its own oracle.

**Gate**
- All 18 hash-verified fixtures pass with per-fixture M-cycle counts recorded. The reporter writes VRAM only with the LCD off, so counts should not change; any change must be explained.
- Full suite, fmt and strict Clippy pass.

### Decisions to agree

1. Restrictions follow the reported mode exactly (point 1), with read/write edge differences left to #73.
2. FEA0-FEFF stays on its current fallback (point 5).
3. Ppu, not Bus, owns the check, with no stored lock state.

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
