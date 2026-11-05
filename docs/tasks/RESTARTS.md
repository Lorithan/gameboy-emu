# RST fixed-vector calls

Issue: [#23](https://github.com/Lorithan/gameboy-emu/issues/23), M2 CPU execution. Baseline: conditional returns committed locally as 40e728a, with 211 passing tests.

## Contract and architecture

C7/CF/D7/DF/E7/EF/F7/FF call 0000/0008/0010/0018/0020/0028/0030/0038 respectively. The destination is encoded in opcode bits 5–3, equivalent to opcode & 0x38. Each form is one byte, unconditional and preserves flags/general registers. It pushes PC immediately after opcode fetch (original address+1, wrapping), not a three-byte CALL return address. There are no operand reads or machine-reset side effects.

At successful opcode completion capture the incremented PC, its full high/low bytes and SP-1. Enter existing Call execution state directly, bypassing the immediate operand reads. Existing Idle and caller high/low writes provide the remainder. Final low write assigns SP and target and completes. Required order: opcode read, Idle, high-byte write at S-1, low-byte write at S-2; four M-cycles/16 ticks total. Requests stay immutable, captured payloads survive budget pauses, and stack accesses retain ordinary Bus semantics. RET returns normally. No new execution states or dependencies are needed.

Sources: [RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7) and [Pan Docs encoding](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md). Architecture was explained before implementation; this issue records that contract and the completed review.

## Completion evidence — 2026-11-04

All 217 tests pass (151 unit, 66 integration); formatting and Clippy pass without warnings. No correctness findings remain. User implemented RST and corrected the initial low-byte mask from 0F to FF during review; assistant changed tests/docs only.

- Independent eight-vector mapping and updated all-byte supported classification.
- Every opcode location for every RST vector (524,288 cases), verifying both full return bytes, wrapping PC, snapshots and final target.
- All flags at representative PC/SP boundaries, missing opcode retry, repeated requests and test-only live PC/SP changes to verify capture.
- Exact public read/Idle/write/write traces and RST/RET round trips in HRAM and WRAM, with returns to a sentinel immediately after the opcode. Four-cycle timing and no extra fetch verified.
- Every instruction budget pause/resume point with no duplicate writes or SP updates.
- Full-address-space memory oracle across all vectors, stack boundaries and pauses: only ordinary mapped stack destinations/echo aliases change. Ignored writes and wrapping follow Bus behavior.
- Existing CALL/RET, PUSH/POP, arithmetic, jump, memory and sticky-fault regressions pass. The old conditional-return test's RST rejection was removed; RETI remains unsupported.

Complete locally, pending commit/push. Interrupt entry and RETI are separate work; this is not completion of the full CPU milestone.
