# Implement LD SP,HL with captured source and an internal cycle

Issue: [#35](https://github.com/Lorithan/gameboy-emu/issues/35). Prepared 2026-11-11.

## Goal
Implement F9 LD SP,HL after the LDH task. One focused load; baseline reviewed instructions through 3140521.

## Contract and architecture proposal
One byte/two M-cycles: opcode Read then Idle. Successful opcode completion advances PC once with wrapping, captures original HL and reports incomplete. Idle completion assigns the captured value to SP, preserves HL/all flags/other registers/memory, restores FetchOpcode and reports true. SP must stay unchanged after opcode fetch.
Use a dedicated decoded operation and a small pending captured word-copy state, or reuse an explicitly named word-write completion abstraction if it already exists. Do not represent this as arithmetic or invent an immediate operand read. Exact types remain user-owned. No Bus/System changes, dependencies or broader refactor.

## Acceptance
Independent mapping/classification; every HL value, all flags at boundaries, captured HL under test-only mutation, missing-opcode retry/PC wrap/request immutability. Exact Read/Idle traces, all budgets/resumes, no early SP update, next instruction continuation. Memory preservation, updated SP consumed by PUSH/POP and ordinary loads unchanged. Full regressions, formatting, strict Clippy.
Exclude 08, signed-SP arithmetic changes, interrupts and CB. Local contract: docs/tasks/LOAD_SP_FROM_HL.md.

Reference checked 2026-11-11: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7). User owns implementation; assistant owns tests/docs/review. No dependencies or ROM fixtures.