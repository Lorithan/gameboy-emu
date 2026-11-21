# Project plan

Updated 2026-11-20. [Documentation index](README.md).

## Current position

M1 cartridge/memory, M2 functional CPU execution/cleanup and **M3 timers/interrupts are complete for their declared DMG scope**. The [M3 gate review](M3_COMPLETION_REVIEW.md) retains its f0d38ef evidence. M4 now has [video-memory storage #71](DEVICES.md#video-memory-foundation), [eight passive LCD data registers #72](DEVICES.md#passive-lcd-data-registers), [LCDC with an internal dot clock #74](DEVICES.md#lcdc-and-internal-dot-clock), [live LY/STAT readback #75](DEVICES.md#live-ly-and-stat-readback) and [STAT/VBlank interrupts #76](DEVICES.md#stat-and-vblank-interrupts) and [VRAM/OAM access restrictions #77](DEVICES.md#vram-and-oam-access-restrictions). Latest no-fail-fast fixture-inclusive regression, 2026-11-20: **539 passed, zero failed** (401 unit, 138 integration), including all eighteen external fixtures, which now take Mooneye's on-screen reporter path with every LY wait completing; formatting and strict Clippy pass. Timing, status, interrupts and access restrictions follow the agreed provisional model and are not hardware-calibrated. Rendering does not exist yet. See [testing](TESTING.md).

**Next: user fix for [#79](https://github.com/Lorithan/gameboy-emu/issues/79)**, HALT with IME=1 accepting device-raised interrupts one M-cycle late, found by [#73 D2](tasks/PPU_CALIBRATION.md#d2-finding-halt-wake-with-ime1-is-one-m-cycle-late-79). The acceptance test is written. D1 is complete: three PPU regression fixtures plus a calibration probe. Predicted: all five `intr_*` PPU ROMs pass after the fix with no PPU constant changed. Then D3 (LCD-enable timing). [#78](https://github.com/Lorithan/gameboy-emu/issues/78) tracks the deferred OAM corruption bug and FEA0-FEFF reads.

The 2026-11-20 gate change splits the former #73 gate into four slices:

- [#75 slice A](https://github.com/Lorithan/gameboy-emu/issues/75): live LY/STAT readback, complete.
- [#76 slice B](https://github.com/Lorithan/gameboy-emu/issues/76): STAT/VBlank interrupts, complete.
- [#77 slice C](https://github.com/Lorithan/gameboy-emu/issues/77): VRAM/OAM access restrictions, complete.
- [#73 slice D](https://github.com/Lorithan/gameboy-emu/issues/73): calibration against audited Mooneye PPU ROMs, required before any hardware-accuracy claim and the M4 gate.

Mooneye's LY-wait timeouts are silent, so every later PPU checkpoint must keep recording per-fixture M-cycle counts. div_write now has about 100,000 M-cycles of margin under the 1,000,000 cap. Rendering, frame publication and DMA remain separate. Agree demo/frontend scope before SDL2 work. #64 physical STOP and #52 DAA remain separate.

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
