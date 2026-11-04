# High RAM before stack instructions

Issue: [#19](https://github.com/Lorithan/gameboy-emu/issues/19), M2 prerequisite for stack operations. Baseline: absolute jumps committed locally as 4ef1bf1, with 169 passing tests (121 unit, 48 integration).

## Required behavior

Map 0xFF80–0xFFFE inclusive to 127 independent writable bytes. HRAM belongs to each Bus and does not alias WRAM, echo RAM, cartridge ROM, or another instance. Initialize it to zero for deterministic tests, matching the project's current WRAM convention. Zero initialization is a project approximation, not a claim about authentic post-boot contents.

The endpoints matter: 0xFF7F is outside HRAM, and 0xFFFF is the interrupt-enable register. Both retain the existing temporary read-FF/ignore-write behavior until their separate device tasks. See the [Pan Docs memory map](https://gbdev.io/pandocs/Memory_Map.html).

The existing post-boot SP is 0xFFFE. Adding this backing storage lets subsequent stack instructions use the normal startup stack region. No CPU-specific stack storage or dedicated stack service is needed.

## Architecture proposal

Add private fixed-size HRAM storage to Bus, with 0x7F bytes. Route reads/writes through the existing methods, indexing relative to 0xFF80. System already owns Bus, so no ownership or public API changes are needed. CPU requests remain ordinary reads/writes; no timing changes are expected.

Names and organization within this boundary are the user's choice. Keep this issue small: stack instructions, interrupt-enable behavior, other I/O registers, devices, dependencies, and ROM fixtures are separate work.

## Acceptance evidence

The assistant owns tests and updates existing tests that currently classify HRAM as unimplemented.

- Every HRAM address stores every byte value correctly; writes preserve all other locations.
- Both endpoints work; accesses to 0xFF7F/0xFFFF do not spill into HRAM.
- HRAM, WRAM/echo, ROM, and distinct Bus instances remain independent. New instances contain zeroed HRAM.
- Full-address-space expectations and ignored-write tests classify the new range correctly and preserve other regions.
- CPU HL-based transfers can write/read HRAM with unchanged instruction timing and captured addresses; budget pauses still delay writes.
- A synthetic program writes an opcode into HRAM and jumps there, proving that instruction fetch and data access share the bus mapping. Verify traces, flags/register preservation, and completion without unintended read-ahead.
- Existing bus, arithmetic, and jump regressions pass; formatting passes and Clippy warnings are reported separately.

## Next dependency

After HRAM passes review, specify PUSH/POP with byte ordering, SP updates, flags masking for POP AF, and cycle payloads. CALL/RET/RST follow in separate tasks. The user implements emulator behavior; the assistant supplies architecture contracts and tests.

## Completion review — 2026-11-04

HRAM maps to independent Bus-owned storage with both endpoints correct. The user corrected the initial WRAM-routing typo during review. No correctness findings remain. Zero initialization remains an approximation, and neighboring I/O/IE placeholders are unchanged.

All 175 tests pass (125 unit, 50 integration). New tests cover all 127 addresses and all 256 byte values, checking all other HRAM locations after each write; WRAM/echo/ROM non-aliasing; independent zeroed Bus instances; and both neighboring addresses. Full-address-space and unimplemented-region expectations are updated, as are CPU memory-operation boundary expectations.

Public CPU tests cover each HRAM address with three representative values and exact store/read traces. Internal System tests verify budget pauses delay writes until the final cycle and preserve WRAM. A program writes INC B into HRAM, jumps there with JP HL, and executes it at the first, middle, and last locations with the expected snapshot and timing, stopping before another fetch. Existing register/flag, arithmetic, bus, and jump regressions pass. Formatting passes; Clippy reports only existing unused set_af. The assistant changed tests/documentation only. Complete locally, not yet committed or pushed.
