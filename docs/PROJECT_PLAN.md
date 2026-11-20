# Project plan

Updated 2026-11-19. [Documentation index](README.md).

## Current position

M1 cartridge/memory, M2 functional CPU execution/cleanup and **M3 timers/interrupts are complete for their declared DMG scope**. The [M3 gate review](M3_COMPLETION_REVIEW.md) retains its f0d38ef evidence. M4's first slice [#71](https://github.com/Lorithan/gameboy-emu/issues/71) now adds [PPU-owned VRAM/OAM storage and Bus mapping](DEVICES.md#video-memory-foundation). Latest no-fail-fast fixture-inclusive regression, 2026-11-19: **489 passed, zero failed** (351 unit, 138 integration), including all eighteen external fixtures; formatting and strict Clippy pass. This is a storage foundation, not functioning PPU timing/rendering. See [testing](TESTING.md).

**Next: implement [#72, eight passive LCD data registers](tasks/LCD_REGISTERS.md).** The proposal specifies SCY/SCX/LYC, BGP/OBP0/OBP1 and WY/WX storage/readback and startup, with FF object-palette startup explicitly a project policy. LCDC/STAT/LY mapping waits for the next timing/startup/interrupt contract: static LY would change the audited Mooneye reporter path. The proposal includes that control/status blueprint and remaining gates. Current video-memory access is unrestricted only for the foundation; PPU clocking, rendering, frame publication and DMA remain unimplemented. Agree the ROM-only interactive demo/frontend scope before SDL2 work. Current coarse timing, skipped boot and physical STOP limits remain explicit; #64 physical behavior and #52 DAA remain separate.

Separate nonblocking follow-ups:

- [#52 external DAA ROM](https://github.com/Lorithan/gameboy-emu/issues/52) is unrun; local DAA tests do not satisfy it.
- [#64 physical STOP assessment](https://github.com/Lorithan/gameboy-emu/issues/64) retains revision/startup/edge/glitch limits. Deterministic STOP integration #44 is complete.
- [#1 core contract](https://github.com/Lorithan/gameboy-emu/issues/1) retains staged device timing/startup decisions. [Architecture](CORE_ARCHITECTURE.md) records implemented boundaries.

## Scope and working agreement

Build an explainable DMG emulator in Rust, initially headless. Planned first release: desktop SDL2 frontend, keyboard input, audio, ROM-only and MBC1 cartridges. SDL2/native dependencies wait until M4; audio follows in M6. Host resources/pacing remain outside the core. CGB, GBA, link cable, additional mappers, save states and speculative multi-system abstractions are outside current release scope. Authentic boot-ROM execution is deferred; current startup explicitly skips boot.

The user writes application code. The assistant leads architecture/planning, issues/docs, review and automated tests/minimal test wiring. Define interfaces, invariants and acceptance evidence before implementation; prefer concrete types/simple ownership. Commit reviewed checkpoints, push and verify remote HEAD; never force-push divergent work without authorization. [AGENTS.md](../AGENTS.md) is the working agreement.

## Milestones

| Milestone | Scope | Gate / position |
| --- | --- | --- |
| M0: Foundation | Architecture, scope, timing, tests, repository | Broad decisions agreed; staged device timing/startup remains under #1 |
| M1: Cartridge/memory | ROM-only loading, headers, initial mapping | Complete within documented limits |
| M2: CPU | Registers, instructions, flags, control flow, stepping | Complete: 500 legal encodings, focused tests and five selected real ROM passes |
| M3: Timers/interrupts | Programmable timer and clock/interrupt integration | Complete within [reviewed scope](M3_COMPLETION_REVIEW.md): local timing/interrupt checks and thirteen audited timer ROM passes |
| M4: Video/controls | PPU, DMA, framebuffer, desktop input | Headless joypad exists; require visual fixtures, PPU/DMA/input checks and an agreed ROM-only interactive demo |
| M5: Cartridge compatibility | MBC1, applicable persistent RAM | Bank/mode/boundary tests, save/load checks, selected supported software |
| M6: Audio/release | DMG channels, playback/pacing, usability | Repeatable audio checks, listening review, regression and honest compatibility report |

Milestones express dependency order, not deadlines. Split work into small issues; demonstrated defects can reopen gates. Raw opcode counts and timeouts do not establish hardware accuracy.

## Documentation and validation

Keep current contracts/limits in guides, active proposals in `docs/tasks`, fixture provenance/results in `docs/testing`. Completed specs and superseded checkpoint narratives live in Git history and closed [issues](https://github.com/Lorithan/gameboy-emu/issues). Update current sections in place instead of prepending progress logs.

Preserve independent result/flag/memory/trace/retry/fault/budget oracles. Before fixtures, record source/revision, artifact hashes, component licenses, prerequisites, reporting, finite limits and expected outcomes. Keep ROM binaries ignored. Report exactly what ran; ignored/empty runs are not passes. See [TESTING.md](TESTING.md).
