# LCD register proposal

Active M4 control/status proposal, 2026-11-19. Video-memory foundation #71 and the [eight passive data registers in #72](https://github.com/Lorithan/gameboy-emu/issues/72) are implemented and verified. The control/status design below sets the direction for subsequent work; live LCDC/STAT/LY mapping still waits for its timing and interrupt contract. User owns application implementation; assistant owns acceptance tests, source review and oracle updates. The implemented passive contract is also recorded in [devices](../DEVICES.md#passive-lcd-data-registers).

## Ownership and representation

Keep LCD state private to Ppu, with parent-visible accessors used by Bus. The CPU continues to issue ordinary reads/writes; System does not inspect PPU fields. No dependency or shared mutable ownership is needed.

- Passive registers are u8 fields. Use scy, scx, lyc, bgp, obp0, obp1, wy and wx, with read_NAME and write_NAME methods, like Timer's register methods.
- LCDC is eight configuration bits. For the later control slice, use an LcdControl byte wrapper with named masks/accessors, full-byte conversion/readback and no invalid byte patterns. Bit helpers should describe hardware meaning; a flags crate is unnecessary.
- The mode portion of STAT uses PpuMode. STAT itself combines writable interrupt enables with computed status, so it is not a single enum or a freely writable stored byte.

## Register map and startup

This table specifies the DMG skipped-boot profile and implementation order. Values are hexadecimal.

| Address | Register | Startup read | Read and write contract | Slice |
| --- | --- | --- | --- | --- |
| FF40 | LCDC | 91 | Read all bits; writes replace configuration and apply enable/disable transitions | Later control/timing |
| FF41 | STAT | 85 as the documented DMG readable profile | Bit 7 reads 1; bits 3-6 writable; mode/comparison computed | Later live status |
| FF42 | SCY | 00 | Full-byte read/write | Implemented #72 |
| FF43 | SCX | 00 | Full-byte read/write | Implemented #72 |
| FF44 | LY | 00 as the documented DMG readable profile | PPU-generated; CPU writes ignored | Later live status |
| FF45 | LYC | 00 | Full-byte comparison configuration; later writes reevaluate STAT | Implement storage now |
| FF46 | DMA | Existing FF fallback for now | Separate Bus DMA contract; not a passive PPU byte | Later DMA |
| FF47 | BGP | FC | Full-byte read/write | Implemented #72 |
| FF48 | OBP0 | FF by project policy | Full-byte read/write | Implemented #72 |
| FF49 | OBP1 | FF by project policy | Full-byte read/write | Implemented #72 |
| FF4A | WY | 00 | Full-byte read/write | Implemented #72 |
| FF4B | WX | 00 | Full-byte read/write | Implemented #72 |

The [Pan Docs handoff table](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md) provides the documented DMG values, while identifying OBP0/OBP1 as uninitialized. Choosing FF for those two is a deterministic project policy, not a hardware guarantee. Keep the existing zeroed VRAM/OAM policy; no boot-ROM logo data is added.

LY00 and STAT85 describe a readable profile, not a complete hidden timing state. STAT85 reports mode1 and coincidence, so initializing OamScan solely because LY is zero would disagree. Before live status mapping, settle the internal line/dot phase and visible-LY derivation, including the final VBlank line, with an explicitly sourced or labeled model seed. This first slice does not choose or freeze that phase.

## Implemented passive register storage

The eight private fields are grouped in LcdRegisters under Ppu and initialized in dmg_post_boot using the table. Parent-visible Ppu read_NAME(&self) -> u8 and write_NAME(&mut self, value:u8) methods expose them to Bus. Each current writer stores its byte and returns unit; each reader returns it without effects. Bus maps exactly FF42, FF43, FF45 and FF47-FF4B. The gaps FF44 and FF46 retain fallback.

Keep every bit of all eight bytes. SCX/SCY, WX/WY and LYC accept all values without clamping. WX's display offset belongs to rendering, not register storage. Object color index0 is transparent during rendering; its unused shade selection does not justify erasing low bits from the stored OBP byte. These byte/readback rules are the proposed storage contract. [Scrolling](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Scrolling.md), [window coordinates](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Window.md), [palettes](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Palettes.md).

LYC is passive only for this checkpoint because comparison and STAT signaling do not exist yet. When live status is introduced, write_lyc must also reevaluate its comparison/interrupt effect. Do not add unused control/mode types, placeholder ticks or temporary dead-code allowances for the later slices now. FF40, FF41, FF44 and FF46 retain current fallback; video memory remains unrestricted until its mode/DMA contract is implemented.

## LCDC control design for the next slice

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

LCDC is configuration; do not duplicate its enable bit in an independently mutable boolean. Distinguish a real bit7 transition from rewriting the same enable state. The later contract must reset/cancel the relevant timing/rendering state on disable, report LY0/mode0 while off, grant CPU video-memory access, and restart the display sequence on enable without clearing VRAM/OAM or the other configuration registers. Timer and global elapsed time continue when the LCD is off. The exact restart phase, STAT-line effects and first blank-frame behavior must be specified/tested before implementing that transition.

## STAT live status and interrupts

Use the four PpuMode values HBlank=0, VBlank=1, OamScan=2 and Drawing=3. Store only writable STAT interrupt-enable bits, masked with 0x78. Readback combines 0x80, those enables, bit2 from visible LY==LYC and the current reported mode. CPU writes cannot set mode/coincidence or bit7. LY is owned/generated by PPU timing; avoid separate independently mutable copies of LY, coincidence and STAT. [STAT layout](https://raw.githubusercontent.com/gbdev/pandocs/master/src/STAT.md); [unused bit7 hardware test](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/unused_hwio-GS.s).

STAT sources share one logical line: enabled HBlank, VBlank, OamScan and coincidence conditions are ORed. Request LCDStat only on that combined line's low-to-high transition, not on every tick or separately for each overlapping source. Remembering the previous line level is real edge-detector state, unlike caching the comparison result. VBlank is a separate interrupt request. Tick/write effects must represent both requests together; Bus sets IF independently of IE/IME. [Interrupt sources and STAT blocking](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupt_Sources.md).

Changes to mode/LY, LYC and STAT enables can change the line; control writes can also have effects. Therefore a later PPU write interface must report effects to Bus rather than waiting for the next tick to notice them. Before that slice, define precise sample/update order, LCD-off gating, startup line initialization and DMG's spurious STAT-write behavior. These are explicit follow-up decisions, not silently implemented as independent interrupt pulses or omitted under a full-accuracy claim.

## Timing and ROM integration gate

PPU will advance once per base tick alongside Timer in Bus's four-tick loop, retaining System's advance-before-access schedule. HALT and interrupt entry clock it; existing Stopped polls consume no ticks. Dot/mode progression, mode-dependent memory access and display restart need their own acceptance before mapping live status. A permanently fixed Drawing duration cannot be called full rendering accuracy. [Rendering and dots](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Rendering.md).

Pinned Mooneye [is_ppu_broken.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s), blob 03facecd5d67ebf4902e6eef122d9e95165b5cf1, treats LY below 160 as a usable PPU. [quit.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s) then uses video setup and waits. Publishing a constant LY0 before progression changes the audited reporter path and can prevent completion; a timeout would not by itself prove a CPU regression. Preserve the original ROMs, reporting expectations and finite caps. Re-audit and run the real helper path when timing is introduced; do not add fixture-specific LY overrides or weaker success criteria. Keeping LY/STAT unmapped is only this explicit intermediate checkpoint, not a final PPU design.

## Acceptance and checkpoints

For #72 four new tests cover all-byte read/write round trips and startup, read purity, exact address/gap decoding, per-register/per-instance/video-memory/Timer/IF/IE isolation, and CPU LDH actions/ticks/snapshots. Expanded existing LDH checks cover both addressing forms and every pause budget; fallback/high-I/O expectations change explicitly for only these eight addresses, preserving unrelated memory/flag/trace/fault/budget oracles. Full regression and fixture evidence are recorded in [testing](../TESTING.md).

Next settle control/timing/startup and live status, then edge-based interrupts/access restrictions with a fresh reporter audit. Rendering, frame publication and DMA stay separate. The remaining design sections are a blueprint for review before dependent implementation; completing passive storage does not authorize mapping frozen LY/STAT.
