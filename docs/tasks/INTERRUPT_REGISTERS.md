# Interrupt controller and IF/IE registers

GitHub task: [#38](https://github.com/Lorithan/gameboy-emu/issues/38), M2 - CPU execution.
Status: implemented and verified locally 2026-11-17. The design section records the original proposal; completion details below describe the final interface.

## Goal
Add Bus-owned interrupt-register storage and FF0F/FFFF mapping as a small M2 prerequisite to interrupt entry. User owns implementation; assistant owns tests/review/docs. Local contract: docs/tasks/INTERRUPT_REGISTERS.md. Baseline 6a634f8: 334 passing tests (236 unit + 98 integration), fmt and strict Clippy pass, verified 2026-11-16. Live main matches.

## Required register behavior
Ordinary DMG post-boot profile: IF reads E1 (stored request bits 01), IE reads 00; CPU IME remains disabled.
FF0F IF: writes replace bits 0-4; ignore writes to 5-7, which always read as ones.
FFFF IE: retain/read back all eight written bits. Bits 5-7 have no interrupt meaning.
Eligible mask is IF & IE & 1F, independent of IME.
Sources bits 0..4: VBlank, LCD STAT, Timer, Serial, Joypad.
Request sets only its IF bit regardless of IE/IME; repeated requests coalesce. Acknowledge clears only the specified source, leaves IE/other requests intact, and is harmless if absent. Register reads and pending queries have no side effects. Software IF writes replace requests; they are not write-one-to-clear.

## Agreed boundary and proposed interfaces
Agreed: Bus owns controller, CPU owns IME, System connects them; CPU never borrows Bus.
Proposal: private memory::interrupts child module, crate-visible re-export of InterruptController/Interrupt, private fields; Interrupt::{VBlank,LcdStat,Timer,Serial,Joypad}. Named post-boot constructor, read_if/write_if/read_ie/write_ie, request(Interrupt), pending_mask() -> u8, acknowledge(Interrupt). Bus retains private controller, routes existing public read/write, and offers narrow crate-visible request_interrupt/pending_interrupts/acknowledge_interrupt forwarding methods. Names are proposals; semantics are the contract. No public mutable controller access or CPU changes. Do not cfg(test)-gate production operations merely because dispatch/peripheral callers arrive later; use a narrowly documented temporary dead_code allowance if needed.

## Acceptance (assistant tests after implementation)
- All 256 writes per register and all 65,536 IF/IE pairs: readback, eligibility, high-bit masking and repeated read/query preservation.
- Every source and each of 32 request sets: request preservation/idempotence, acknowledge preservation/idempotence, requests while disabled, later IE enable, unchanged IE including high bits.
- Exact mapping, separate storage and adjacent FF0E/FF10/FF7F fallback; FFFE remains independent HRAM. Full address-space before/after comparisons, ROM protection, WRAM/echo/HRAM regression and independent Bus instances.
- System synthetic programs exercise absolute/LDH reads/writes to both registers with existing exact cycle counts, no early writes, every budget pause/resume, unchanged CPU IME. IE-enabled pending requests still do not dispatch in this task.
- Update stale fallback oracles across loads, stack, word-store and CB tests. Preserve wrap-at-FFFF coverage by explicitly writing intended instruction data to IE; keep independent expected hardware rules.
- Full cargo test --all-targets --quiet, cargo fmt --check, cargo clippy --all-targets -- -D warnings.

## Sources checked 2026-11-16
- [Pan Docs interrupts](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupts.md): source bits, software IF writes, IME separation and acknowledgment.
- [Pan Docs power-up sequence](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md): DMG IF E1 and IE 00 at cartridge entry.
- [Mooneye unused_hwio-GS source](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/bits/unused_hwio-GS.s): hardware-verified IF high bits read one, IE high bits retain written values. Consulted only; no ROM imported/run.

No interrupt sampling/priority API, interrupt entry, vectors, RETI/HALT/STOP, peripherals, dependencies, frontend or broad refactors. Simultaneous hardware request/software-write ordering will be specified with device scheduling. Startup is a boot-skip convention, not a complete hardware power-on model.

## Suggested implementation order

1. Implement controller storage and register semantics, then request/pending/acknowledge operations. Keep the five-source type small; bit-mask conversion may remain private.
2. Add the private Bus field, post-boot initialization, exact address mapping and narrow forwarding operations.
3. Assistant adds controller/Bus/System acceptance tests and updates existing fallback assumptions; review before committing.

Concrete readback examples: writing 00 to IF reads E0; writing FF reads FF. Writing 00, E0 or FF to IE reads exactly 00, E0 or FF. With IF=FF and IE=E0 the eligible mask is zero; with IE=FF it is 1F. Reading a pending request does not consume it. Software can clear all requests by writing zero to IF.

Known migration points include src/memory/tests.rs (FFFF currently ignored), src/system/tests/cb_integration.rs (FFFF target policy), tests/cb_instructions.rs (CB prefix at FFFE currently assumes FF at FFFF), and independent full-memory oracles in transfer/stack/word-store tests. Audit all fallback assumptions, rather than replacing expected values with reads from the implementation under test.

## Verification checkpoint - 2026-11-16

All four handoff commits exist locally, through 6a634f8. Clean working tree before documentation edits. Read-only live branch lookup confirms GitHub main is 6a634f8a71589d57c7c929c57991a3ed5a731407; cached origin/main matches. Issue #37 is closed. Fresh baseline: 334 tests pass (236 unit, 98 integration); formatting and strict Clippy pass. These checks validate the existing implementation, not the proposed controller. No external ROM suite was run. No application code, test code, build configuration or dependencies changed during specification.

## Completion evidence - 2026-11-17

The user implemented the controller, named post_boot constructor, five-source bit-index helper, IF/IE mapping and crate-visible Bus request/pending/acknowledge methods. InterruptController is visible only within memory; Interrupt is re-exported crate-wide. CPU never borrows Bus. Assistant changes cover tests, minimal test wiring, formatting, documentation, and explicitly approved temporary dead-code allowances for staged production interfaces. No behavior fixes were required.

Five new tests cover:
- All 65,536 IF/IE byte pairs, exact register readback, high-bit masking, repeated pure reads and eligibility queries.
- All 32 request sets, all 256 IE values and all five sources: repeated requests/acknowledgments, preservation of other sources and IE, and disabled requests becoming eligible after enabling IE.
- Every byte written to each register with all 65,536 addresses checked after every write, independent instances and neighboring fallback addresses. Existing ROM/WRAM/echo/HRAM coverage remains.
- Six CPU access forms (EA/FA/E0/F0/E2/F2), both registers, six data patterns and every budget boundary: exact resumed actions/ticks/completion, no early writes, zero-budget preservation, snapshots, unchanged IME and following-instruction continuation.
- Enabled IME with eligible requests still executes instructions without dispatch in this scoped checkpoint. This temporary test must evolve with interrupt-entry support.

Existing independent stack, CALL/RST, memory-operation, transfer, LDH, word-store and CB expectations now recognize IE storage. The LDH every-offset oracle also accounts for IF masking. The CB PC-wrap test explicitly initializes IE to the intended second opcode instead of depending on unmapped fallback.

Final validation: 339 tests pass (241 unit, 98 integration), cargo fmt --check, cargo clippy --all-targets -- -D warnings, and git diff --check pass. No external ROM suite was imported or run. No dependencies/build configuration changes. Temporary dead-code allowances should be removed when interrupt entry/peripherals use these APIs.

Next: specify interrupt sampling boundaries, source priority, acknowledgment ordering, the five-M-cycle entry sequence and explicit interrupt-entry versus instruction-completion outcomes before user implementation. RETI/HALT/STOP and peripherals remain separate work.
