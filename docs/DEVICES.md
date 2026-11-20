# Devices and startup

Implemented ordinary-DMG skipped-boot profile. [Architecture](CORE_ARCHITECTURE.md) defines scheduling; [Timer](TIMER.md) records the implemented programmable timer contract.

## Cartridge and memory

Cartridge accepts validated unchanged 32 KiB ROM-only images with no cartridge RAM. Header/checksum/type-size constraints and errors are tested; unsupported configurations are rejected. MBC1/persistence belong to M5. No header patching or fallback mapper is used for fixtures.

| Address/range | Mapping |
| --- | --- |
| 0000-7FFF | Cartridge ROM; writes ignored |
| 8000-9FFF | 8 KiB PPU-owned VRAM; unrestricted storage access in the first foundation slice |
| C000-DFFF | 8 KiB WRAM |
| E000-FDFF | Corresponding WRAM echo |
| FE00-FE9F | 160 bytes PPU-owned OAM; unrestricted storage access in the first foundation slice |
| FF80-FFFE | 127-byte HRAM |
| FF00 | JOYP |
| FF04 | DIV |
| FF05-FF07 | TIMA/TMA/TAC with tick counting, reload/write phases and DIV/TAC write-edge integration |
| FF0F / FFFF | IF / IE |
| FF42-FF43 / FF45 / FF47-FF4B | Passive LCD data registers in Ppu |
| Other | Read FF, ignore writes; unimplemented hardware |

WRAM/HRAM start zero as deterministic project policy, not physical RAM behavior. CPU starts PC=0100, SP=FFFE, A=01, BC=0013, DE=00D8, HL=014D. F=80 for zero header checksum, otherwise B0. Source: [Pan Docs post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md). Authentic boot and other console profiles are deferred.

## Video-memory foundation

[#71](https://github.com/Lorithan/gameboy-emu/issues/71) adds Bus-owned Ppu with private VRAM/OAM arrays. Both start zero as deterministic project policy, intentionally omitting boot-ROM logo/tile initialization. Bus subtracts the mapped range base and converts to usize offsets; Ppu indexes directly without masking/modulo aliases. Every byte retains all eight bits independently; reads are pure and instances/regions are isolated. FEA0-FEFF retains existing FF/ignored-write fallback and is not additional OAM; that fallback is not a physical accuracy claim.

Video memory and the passive LCD registers below are storage/mapping only: no PPU clocking, LCD control/status transitions, modes/access locks, rendering, interrupt requests, frame output or DMA yet. FF40, FF41, FF44 and FF46 remain fallback, including LY and DMA; existing audited ROM reporter behavior is unchanged. Agreed later architecture advances PPU once per base tick alongside Timer and exposes read-only completed frames through System, under contracts still to be specified.

Five assistant acceptance tests cover zero/read purity, all-offset independent address patterns, all-byte endpoint/interior writes, full-memory/Timer-phase/instance isolation, unusable-tail nonaliases and CPU word stores across video-region boundaries with exact traces and every pause budget. Existing load/store/stack/interrupt/RETI/CB oracles now explicitly include the newly mapped ranges. This establishes the foundation contract, not functional PPU accuracy. [Mapping reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Memory_Map.md); [future mode restrictions](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Accessing_VRAM_and_OAM.md).

## Passive LCD data registers

[#72](https://github.com/Lorithan/gameboy-emu/issues/72) adds private LcdRegisters under Ppu, with parent-visible Ppu accessors and exact Bus routing. All eight bytes retain/read every bit; reads are pure. Scrolling/window coordinates and LYC are not clamped; WX's display offset and object color-index transparency belong to future rendering, not byte storage.

| Address | Register | Startup byte |
| --- | --- | --- |
| FF42 / FF43 | SCY / SCX | 00 / 00 |
| FF45 | LYC | 00 |
| FF47 | BGP | FC |
| FF48 / FF49 | OBP0 / OBP1 | FF / FF, deterministic project policy |
| FF4A / FF4B | WY / WX | 00 / 00 |

The [handoff table](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md) gives the documented DMG values but identifies OBP0/OBP1 as uninitialized; their FF startup is a project choice. LYC currently has no comparison/interrupt effects. Live status, LCD enable transitions and shared STAT-line signaling need the [remaining register/timing contract](tasks/LCD_REGISTERS.md) before implementation. LY remains fallback so a frozen scanline cannot select Mooneye's functional-PPU reporter path prematurely.

Four new tests verify startup/read purity, every byte with per-register/full-memory/Timer-phase/IF/IE/instance isolation, unmapped address gaps, and CPU round trips with exact actions/ticks/snapshots. Expanded LDH tests cover both addressing forms, every pause budget, current register readback and all high-I/O offsets; full-memory expectations are changed only for the eight mapped registers. No PPU timing/rendering hardware result is implied.

## LCDC and internal dot clock

[#74](https://github.com/Lorithan/gameboy-emu/issues/74) maps FF40 to an `LcdControl` byte wrapper in LcdRegisters (startup 91, every bit read/written). Ppu owns private `PpuTiming` (`ppu/timing.rs`): physical line 0-153, dot 0-455, `PpuMode` and a first-line-after-enable flag. Bus calls `Ppu::tick` once per base tick after the Timer event, so each M-cycle is four dots; Ppu advances timing only while LCDC bit 7 is set.

- Startup seed: line 153, dot 4, VBlank, no first-line state; line 0 / dot 0 / OamScan follows after 452 dots.
- Ordinary lines 0-143: OamScan 0-79, Drawing 80-251, HBlank 252-455; lines 144-153 VBlank. A frame is 70,224 dots.
- `write_lcdc` acts only on bit-7 edges: on→off sets `PpuTiming::lcd_off()` (line 0/dot 0/HBlank) and timing freezes; off→on sets `lcd_on()`, whose first line is HBlank 0-79 / Drawing 80-251 / HBlank 252-455 before an ordinary OamScan line. Same-state writes preserve timing.
- Neither edge touches VRAM/OAM, passive registers, Timer, IF/IE or elapsed time. STOP entry preserves the PPU phase; HALT and interrupt entry advance it.

`mode` updates at boundary dots, so constructors must produce consistent states; `lcd_off()` is valid only because timing never ticks while disabled. The schedule is the provisional fixed-transfer scaffold from the [contract](tasks/LCD_REGISTERS.md), not a hardware match. #74 itself left STAT, LY and DMA as fallback; #75 below maps LY and STAT.

## Live LY and STAT readback

[#75](https://github.com/Lorithan/gameboy-emu/issues/75) makes FF44 and FF41 readable from the dot clock. It requests no interrupts and adds no access restrictions; DMA (FF46) remains fallback.

- **LY (FF44):** `PpuTiming::visible_ly()` returns the physical line, except on line 153, which shows 153 at dots 0-3 and 0 from dot 4 (provisional, after Pan Docs and GBEDG). With the LCD off, timing sits at line 0, so LY reads 0. Writes are ignored.
- **STAT (FF41):** `LcdStatus`, held in LcdRegisters, owns the byte layout. Reads give `0x80 | select | LY=LYC flag << 2 | mode`. Bit 7 is unused and always reads 1, independent of LCDC. Writes store only the select bits (`& 0x78`). With the LCD off, mode reads 0.
- **LY=LYC flag:** stored, not computed on read. `Ppu::tick` updates it after each enabled tick from `visible_ly()` and LYC. It holds while the LCD is off, across LYC writes and disabling, and after enable until the first enabled tick. It starts true, so the existing seed reads LY 00 / STAT 85 without hard-coded bytes.

Live LY switches Mooneye fixtures to the on-screen reporter; measured costs are in [testing](TESTING.md#live-lystat-75). STAT/VBlank interrupts are #76; VRAM/OAM access restrictions are #77; calibration of these provisional constants is #73.

## Interrupt registers

Bus owns the controller. Internal IF starts 01 (reads E1); IE starts 00. IF upper bits read one; IE upper bits retain writes but eligibility is `IF & IE & 0x1F`. Requests set one bit; acknowledgments clear one bit; reads/pending queries are pure. Priority is VBlank, LCD STAT, Timer, Serial, Joypad, with vectors 0040/0048/0050/0058/0060. [CPU entry/IME](CPU.md#ime-and-interrupt-entry) remains separate.

## Divider phase

Timer owns a wrapping u16 base-tick counter; DIV reads the high byte. Real M-cycles tick four times, including HALT/entry. DIV writes ignore their value and clear the whole counter; actual StopEntered resets it once. Neither resets monotonic elapsed_ticks. Stopped polling/input consumes no ticks. TIMA/TMA start zero; TAC stores low three bits and reads them OR F8. Tick and DIV/TAC write falling edges share Counting-only TIMA increments; overflow waits four future ticks before current-TMA reload and one-shot IF through Bus. Phase-dependent TIMA cancellation/protection and TMA propagation are locally verified. Writes preserve pending/reload phase; TAC never resets the divider. [Contracts](TIMER.md) and [hardware-source results](testing/MOONEYE_TIMER.md): all thirteen timer fixtures pass after the #70 System fetch-boundary correction. Coarse STOP remains separately scoped below; arbitrary T-state accuracy is not claimed.

The seed ABC8 at elapsed_ticks=0 is a **model-phase inference**, not an independently measured hidden register. Visible AB follows the [DMG startup profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md). Phase derives from pinned [Mooneye boot_div-dmgABCmgb](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/boot_div-dmgABCmgb.s) and our advance-before-access schedule:

| M-cycles at read | Counter from ABC8 | Source DIV |
| ---: | --- | --- |
| 14 | AC00 | AC |
| 78 | AD00 | AD |
| 141 | ADFC | AD |
| 205 | AEFC | AE |
| 269 | AFFC | AF |
| 334 | B100 | B1 |

First visible increment occurs at 56 ticks/14 M-cycles. These source-derived tests are not an executed boot_div ROM pass. Revisit phase if scheduling changes. The existing external div_timing explicitly resets DIV and does not certify startup phase. [DIV reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).

## Joypad

Joypad stores an eight-button mask and selection bits 4-5. Zero in bit 4 selects directions; zero in bit 5 selects actions. Lines 0-3 correspond to Right/A, Left/B, Up/Select, Down/Start. Selected pressed buttons drive shared lines low; both selected rows combine presses. Readback is `0xC0 | selection | input_lines`; writes retain bits 4-5. Startup selects both rows with no held buttons, reading CF, a deterministic no-host-input profile.

Selection/button changes compare old/new lines: a selected high-to-low transition requests Joypad IF. Repetition, releases and already-low shared lines create no falling event. Active selected level is separately `input_lines != 0x0F`. [JOYP reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Joypad_Input.md).

## STOP entry and wake

CPU samples post-read selected JOYP level and eligible pending mask. If fetch arbitration accepts an interrupt, STOP does not execute or reset DIV. Otherwise the coarse model consumes one opcode-read M-cycle, no padding bus read, and performs ordinary fetch increment/suppression before this table:

| Selected input active | Eligible pending | Skip padding | Next state | Outcome / DIV reset |
| --- | --- | --- | --- | --- |
| No | No | Yes | Stopped | StopEntered / reset |
| No | Yes | No | Stopped | StopEntered / reset |
| Yes | No | Yes | Halted | InstructionComplete / no reset |
| Yes | Yes | No | FetchOpcode | InstructionComplete / no reset |

Pending masks ignore upper bits, independent of IME; enabled interrupt acceptance still precedes opcode fetch. All branches share completion EI bookkeeping. HALT-like STOP does not invoke HALT's separate suppression decision.

System::set_button updates Bus/IF, then offers the active level to guarded CPU wake. Only active Stopped becomes ordinary FetchOpcode; PC/registers/IME are preserved. Input clocks nothing, resets no DIV, acknowledges no IF and performs no service. A selected press/release between execution calls leaves CPU awake through its state, without another latch. Pre-entry pulses are not queued; held input uses the entry table. Next real step arbitrates normally; IF alone cannot resume STOP.

The user-approved timer policy #68 retains the current entry reset without manufacturing a TIMA falling edge. All TIMA/TMA/TAC/phase state is preserved except the counter; remaining overflow delay and Reloading freeze while stopped. Wake consumes no ticks; real ticks resume the pending phase. Already-issued timer IF survives entry/pause and normal service follows only after selected-input wake. Tests also verify HALT-like/Continue branches keep clocking and avoid reset. This is explicitly a coarse policy, not sourced physical reset-to-TIMA edge fidelity.

Deterministic #44/#46/#47/#48 integration is complete. Immediate host wake, conditional padding and absent padding bus access are project approximations. Oscillator startup, analog edges/glitches/vector corruption, entry reset-to-TIMA detail and zombie power remain physical assessment [#64](https://github.com/Lorithan/gameboy-emu/issues/64). CGB KEY1 is outside scope. Sources: [STOP reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Reducing_Power_Consumption.md), [branch chart](https://raw.githubusercontent.com/gbdev/pandocs/master/src/imgs/stop_diagram.svg), [CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595). Existing tests cover entry/wake/polling, input matrices/pulses, IF/IME, resume PC, clocks and interrupt traces.
