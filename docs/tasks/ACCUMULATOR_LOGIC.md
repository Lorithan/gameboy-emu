# Accumulator AND, XOR and OR

Active issue: [#24](https://github.com/Lorithan/gameboy-emu/issues/24), reopened for redo on 2026-11-06. The user reports the later implementation is on an inaccessible crashed PC and has chosen to redo it. Start from published RST checkpoint 8a2c2f5 (217 passing tests); do not wait for recovery. Historical completion results do not validate the replacement implementation.

## Behavior

Support AND A0-A7/E6, XOR A8-AF/EE, and OR B0-B7/F6. Register forms are one byte/one M-cycle; (HL) forms one byte/two M-cycles; immediate forms two bytes/two M-cycles. Write the bitwise result to A. Set Z iff result is zero; clear N/C. Set H for AND and clear H for XOR/OR. Incoming flags do not affect results. Preserve other registers, SP and memory. Self-source forms must honor aliasing.

Reference checked 2026-11-06: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7), AND/OR/XOR entries. No ROM fixtures or dependencies are required.

## Architecture proposal

Extend the existing accumulator operation selector with the three logic operations and retain execute_accumulator ownership of A/flags only. Reuse the register, immediate and captured-HL operand paths. Callers continue owning PC advancement, execution state and completion markers. No new cycle states, Bus/System changes or general ALU framework are expected. Renaming ArithmeticKind is optional.

Begin with register AND, then immediate/(HL), then XOR/OR. Interrupts, complement/carry instructions, DAA and CB decoding remain separate.

## Acceptance criteria and assistant tests

- Independently verify all 27 mappings and the updated supported-byte classification.
- Exhaust all non-A operand pairs for all three operations and source paths using an independent per-bit oracle; cover self-source forms and every initial flag combination.
- Check captured original A and HL address, no early side effects, missing-read retries, immutable requests and PC wrapping.
- Verify exact public cycle actions, ticks, completion markers and zero/partial/exact/excess budgets.
- Check full memory preservation and unrelated registers/SP; exercise logic results in conditional-branch programs.
- Run the complete regression suite, formatting and strict Clippy. Record observed results without declaring full CPU or hardware correctness.

## Redo review - 2026-11-06

Register and (HL) logic execution/flags pass an independent exhaustive oracle (1,376,256 operand cases); all self-source values/initial flags pass (12,288 cases). Missing decoder mappings: E6 AND immediate, EE XOR immediate, F6 OR immediate. Five new unit tests are wired; three currently fail at missing E6 (mapping, exhaustive immediate, retry/capture tests), while register/memory and self-source tests pass. Full unit run: 153 pass, three fail. All 66 existing public integration tests pass separately. Formatting and strict Clippy pass after assistant test lint fixes. Application implementation unchanged by assistant. Public logic trace/budget/memory integration coverage remains to be added before closing #24.

## Redo completion - 2026-11-06

Immediate mappings E6/EE/F6 are now implemented by the user. All 226 tests pass (158 unit, 68 integration); formatting, strict Clippy and diff whitespace checks pass. No correctness findings remain. The assistant changed tests/docs and applied formatting to opcode.rs; emulator logic remains user-authored.

Evidence: independent 27 mappings; 1,572,864 non-self operand cases across all source paths using a per-bit boolean oracle; 12,288 self-source/initial-flag cases; boundary values with every initial flag and wrapping PC; captured A/HL and atomic missing-read retries; exact public read traces, completion markers, timing and all budget pauses; full-address-space preservation before/after pauses; mixed logic/branch execution completes in 76 ticks. The new branch test's expected final sentinel address was corrected to 0113 during test development. All previous regressions pass. No external ROM suite was run.

Next: #25 CPL/SCF/CCF, beginning with CPL. No new CPU states or Bus/System behavior are needed for that separate task.
