# First PPU foundation

Active M4 implementation [#71](https://github.com/Lorithan/gameboy-emu/issues/71). User writes application code; assistant owns architecture, review and tests/minimal test wiring. [M3](../M3_COMPLETION_REVIEW.md) is complete. This first slice adds only video-memory ownership/storage and Bus routing; it does not yet implement a functioning PPU.

## Agreed architecture

Bus owns PPU. PPU will own VRAM, OAM, registers, rendering state and frame output. Bus coordinates accesses and device effects; System will expose read-only display output/events to the frontend. PPU advances one dot per base tick, alongside Timer inside Bus's four-tick DMG M-cycle loop. System stepping remains in M-cycles, and host pacing remains outside the core. Independent simultaneous PPU effects must be representable; exact event/frame interfaces are still to be specified before dependent code.

## Implement now

1. Add src/memory/ppu.rs and declare it from memory.rs. Use Ppu with private vram:[u8;0x2000] and oam:[u8;0xA0], visible only as needed to its parent.
2. Add dmg_post_boot with both arrays zeroed. This is deterministic project policy, not authentic boot-ROM video memory; no logo data is introduced.
3. Provide parent-visible read_vram(offset:usize), write_vram(offset:usize,value:u8), read_oam(offset:usize) and write_oam(offset:usize,value:u8). Reads return u8; writes return unit. Offsets must already be within the relevant array; Bus performs address decoding and subtracts the range base. No modulo/masking aliases, mutable array exposure or public frontend API.
4. Give each Bus its own Ppu initialized by the constructor, and route both reads and writes:

| CPU address | Storage | Offset |
| --- | --- | --- |
| 8000-9FFF inclusive | 8192 bytes VRAM | addr -8000 |
| FE00-FE9F inclusive | 160 bytes OAM | addr -FE00 |
| FEA0-FEFF | Existing fallback, not OAM | Read FF; ignore writes, as current project policy |

Every mapped byte retains all eight bits independently. Reads do not mutate state or time. Other memory/device/instance state is isolated. CPU access remains unrestricted at this storage-only stage; future mode/DMA access rules will deliberately extend it.

Leave LCD registers FF40-FF4B (including DMA FF46), timing, interrupts, rendering, framebuffers and frontend interfaces for their specified slices. No placeholder tick function, new dependency or build change is needed. Existing fallback LY/SC behavior used by audited ROM reporters is unchanged.

## Acceptance owned by the assistant

- Zeroed arrays, pure reads and independent per-instance state.
- All offsets using address-dependent patterns, all-byte writes at endpoints/interior, exact mapped boundaries and no aliases into each other or surrounding memory.
- WRAM/HRAM/ROM, timer/joypad/IF/IE and instance isolation.
- CPU load/store integration retaining exact cycle actions, timing, snapshots and bounded stepping.
- Explicit migration of existing fallback/full-memory/stack oracles where the two newly mapped regions now retain writes. Preserve every unrelated expectation.
- Full regression, formatting and strict Clippy, including all eighteen hash-verified existing ROM fixtures. The last verified application baseline f0d38ef passed 484 tests; this planning step runs no new tests and claims no PPU hardware result.

When implementation is ready, the assistant adds/tests these oracles. The user does not need to change automated tests.

## Next contracts

Specify LCD register masks/permissions and skipped-boot values together with initial LY/mode and LCD enable/disable behavior. Then settle dot/mode progression, CPU access restrictions and VBlank/STAT effects before rendering, frame publication and separate DMA coordination. Rendering approximation versus pixel-fetcher scope, exact event/output types and startup timing remain decisions before their dependent work; no invented constant LY or fixed Mode3 accuracy is implied by this foundation.

Primary references: [memory map](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Memory_Map.md), [mode-dependent access](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Accessing_VRAM_and_OAM.md), [boot sequence](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md), [rendering/dots](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Rendering.md). FEA0-FEFF's hardware behavior varies; retained FF fallback is an explicit limit. Zero startup arrays intentionally omit boot-ROM logo/tile initialization.
