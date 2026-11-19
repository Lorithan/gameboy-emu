# Project plan

Updated 2026-11-19. [Documentation index](README.md).

## Current position

M1 cartridge/memory and M2 functional CPU execution/cleanup are complete. M3 register #65, tick/reload/write-phase #66 and DIV/TAC write-edge #67 slices are locally verified for the declared coarse schedule. Latest executable verification, 2026-11-19: **461 passing tests** (336 unit, 125 integration), including five hash-verified existing external fixtures; formatting and strict Clippy pass. STOP pending-phase interactions and external timer conformance remain incomplete. See [testing](TESTING.md) and [M2 decision](M2_COMPLETION_REVIEW.md).

**Next: settle STOP reset-edge/pending-phase/pause/resume scope under [timer design #53](https://github.com/Lorithan/gameboy-emu/issues/53).** [DIV/TAC write-edge #67](tasks/TIMER.md#div-and-tac-write-edge-contract) preserves Counting-only increments, existing overflow/reload state and delayed IF. STOP remains separate and unchanged until its contract is settled. Pinned hardware timer ROM audits/runs and phase calibration remain required; #53 stays open.

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
| M3: Timers/interrupts | Programmable timer and clock/interrupt integration | Active design #53; DIV/service/standby foundations exist. Timing/interrupt tests and audited relevant ROM results required |
| M4: Video/controls | PPU, DMA, framebuffer, desktop input | Headless joypad exists; require visual fixtures, PPU/DMA/input checks and an agreed ROM-only interactive demo |
| M5: Cartridge compatibility | MBC1, applicable persistent RAM | Bank/mode/boundary tests, save/load checks, selected supported software |
| M6: Audio/release | DMG channels, playback/pacing, usability | Repeatable audio checks, listening review, regression and honest compatibility report |

Milestones express dependency order, not deadlines. Split work into small issues; demonstrated defects can reopen gates. Raw opcode counts and timeouts do not establish hardware accuracy.

## Documentation and validation

Keep current contracts/limits in guides, active proposals in `docs/tasks`, fixture provenance/results in `docs/testing`. Completed specs and superseded checkpoint narratives live in Git history and closed [issues](https://github.com/Lorithan/gameboy-emu/issues). Update current sections in place instead of prepending progress logs.

Preserve independent result/flag/memory/trace/retry/fault/budget oracles. Before fixtures, record source/revision, artifact hashes, component licenses, prerequisites, reporting, finite limits and expected outcomes. Keep ROM binaries ignored. Report exactly what ran; ignored/empty runs are not passes. See [TESTING.md](TESTING.md).
