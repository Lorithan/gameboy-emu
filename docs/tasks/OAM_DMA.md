# OAM DMA (FF46) contract

Proposed 2026-11-21 for the next M4 slice ([#80](https://github.com/Lorithan/gameboy-emu/issues/80)). Not yet agreed. The user implements; the assistant owns the tests, fixture wiring and review.

OAM DMA copies 160 bytes from `XX00-XX9F` into OAM (`FE00-FE9F`) when the CPU writes `XX` to FF46. Games start it from an HRAM routine every frame to update sprites, so sprite rendering will depend on it.

## Sources

All at the pinned Mooneye revision `31510e12`, verified on DMG. The ROMs are already in the shared archive ([mooneye-reg-f.json](../../tests/fixtures/mooneye-reg-f.json)).

| ROM | Cartridge | Observes |
| --- | --- | --- |
| oam_dma/basic | ROM only | all 160 bytes copied from a ROM source |
| oam_dma/reg_read | ROM only | FF46 reads return the last written value, during and after a transfer, and after two quick writes |
| oam_dma_timing | ROM only | OAM still reads FF one M-cycle before the end and reads the data exactly at the end |
| oam_dma_restart | ROM only | a second write mid-transfer restarts it; the same end alignment applies to the second transfer |
| oam_dma_start | ROM only | the first M-cycles of a fresh and a restarted transfer, observed by fetching code from OAM |
| oam_dma/sources-GS | MBC5 | source areas, including `E0-FF`; **not runnable** (MBC5 is outside planned scope), so its table is reproduced as local tests |

`oam_dma_start` states the timeline directly. For a fresh transfer: M=0 the FF46 write, M=1 nothing (OAM still accessible), M=2 the transfer starts and OAM reads return FF. For a restart: the previous transfer keeps running, and blocking OAM, during M=0 and M=1; the new one starts at M=2.

## Proposed rules

M-cycle numbers count from the M-cycle whose CPU access is the FF46 write (M0). Each later M-cycle's DMA progress happens in Bus advancement, before that M-cycle's CPU access, like the PPU and timer.

1. **FF46 register.** Reads return the last written byte at all times. It starts at FF (Pan Docs DMG power-up table; no pinned ROM observes the startup value, so it is provisional). A write stores the value and schedules a transfer from source `value << 8`.
2. **Timeline.** M1 does nothing. The transfer copies byte 0 during M2's advancement, byte *i* during M(2+*i*) and the last byte, 159, during M161. It is **active** for the CPU accesses of M2 through M161 (160 M-cycles) and inactive from M162. That is exactly what `oam_dma_timing` measures.
3. **Restart.** A write while a transfer is active schedules a new one with the same M1/M2 timing. The old transfer keeps copying during M0 and M1 and is replaced at M2, with the new source starting at byte 0. The register always holds the latest value (reg_read rounds 5-6 write twice, four M-cycles apart). A second write before the first transfer has started replaces the pending start (provisional; no pinned ROM writes twice within two M-cycles).
4. **Source mapping.** Byte *i* is read from `(value << 8) + i`. Sources at E000 and above read WRAM at the address minus 0x2000, so `E0` reads C000, `FE` reads DE00 and `FF` reads DF00 (sources-GS). Reads go through a DMA-only path: no read side effects, and the CPU's PPU-mode VRAM lock does not apply to them (provisional; hardware conflicts with the PPU are a rendering-era question). With this ROM-only cartridge, A000-BFFF reads FF as it does for the CPU.
5. **Destination.** Byte *i* goes to OAM index *i* directly, ignoring the PPU-mode OAM locks. The CPU path stays separate.
6. **CPU OAM access while active.** CPU reads of FE00-FE9F return FF and writes are ignored, regardless of LCDC and PPU mode. This is in addition to the #77/D3 locks, not instead of them. FEA0-FEFF keeps its fallback.
7. **Other CPU accesses: no bus conflicts (proposed simplification).** All other addresses behave normally during a transfer. On hardware, CPU accesses to the bus the DMA is using (external: ROM/SRAM/WRAM; video: VRAM) conflict. No pinned ROM checks conflict values, and `oam_dma_start` runs its wait loop from ROM during a VRAM-source transfer, which this rule allows. Software that follows the documented HRAM-only rule cannot tell the difference. Bus conflicts would be a separate, sourced issue.
8. **Independence.** The transfer runs with the LCD on or off and while the CPU is halted, because Bus advancement continues. It does not advance while stopped, which matches the current STOP model, where no M-cycles advance. It requests no interrupts and does not change PPU, timer or interrupt state.

## Proposed ownership and types

- **Bus owns the DMA state**, because the source can be anywhere in the memory map. A small `Copy`-able `OamDma` value (your design) holds the register byte, the active transfer (source page and next index) and the pending start (source page and remaining delay).
- **Advancement:** once per M-cycle in `advance_mcycle`. Handle the pending start first, then copy one byte if a transfer is active. Whether it runs before or after the four PPU/timer ticks makes no observable difference yet.
- **Ppu** gains a DMA-path OAM write that bypasses the CPU locks. Its CPU OAM accessors, or Bus, also consult "DMA active". Decide which owns that check; Bus is the simpler choice because the PPU never needs to know about DMA for CPU locking.
- **Rejected:** copying all 160 bytes at the write. It cannot express the timeline, restart or blocking that the ROMs check.

## Decisions for the user

1. Rule 7: no bus conflicts now (proposed), or model conflicts in this slice.
2. Rule 1: FF46 starts at FF (proposed).
3. Where the "DMA active blocks CPU OAM" check lives: Bus (proposed) or Ppu.

## Acceptance (assistant; written after agreement)

- **Local Bus tests:**
  - the timeline at every M-cycle from M0 to M163: OAM readable at M1, FF from M2 to M161, data at M162;
  - all 160 bytes copied from each runnable source region (ROM, VRAM, WRAM C0 and DF) and the echo mappings E0, FE and FF;
  - restart at several offsets, including during M1 of a previous start, and two consecutive writes;
  - FF46 readback at every phase;
  - CPU OAM writes ignored while active and allowed afterwards;
  - DMA writes landing during OamScan and Drawing with the LCD on;
  - LCD off;
  - nothing else in the memory map, PPU, timer or IF changed against a reference Bus.
- **System tests:** a transfer continues across HALT; instruction-budget pauses do not change the result; an HRAM routine like Mooneye's `dma_proc` completes.
- **Fixtures:** the five ROM-only Mooneye DMA ROMs pinned in a manifest with a checksum-gated script, all passing, with M-cycle counts recorded. The 37 existing fixtures keep their counts.
- Full suite, fmt and strict Clippy pass.
