# M2 completion review

Review task [#51](https://github.com/Lorithan/gameboy-emu/issues/51), 2026-11-18. Reviewed published checkpoint f0283fa. This is a source/evidence/planning review, not a new emulator implementation or test run. The last actual full run passed 430 tests (312 unit, 118 integration), explicitly including five external fixtures; formatting and strict Clippy passed.

## Decision

Complete M2 for its documented functional DMG CPU scope and move implementation planning to M3. The original gate asks for instruction implementation, focused edge-case tests, deterministic stepping/traces and selected CPU ROM results. All are supported below. It does not require every external suite, a PPU/mapper or physical-edge certification. Requiring those now would silently enlarge the milestone.

GitHub milestone [M2 - CPU execution](https://github.com/Lorithan/gameboy-emu/milestone/3) was set to closed after this review; the update response confirmed the closed state. #52 remains a separate confidence follow-up, and #53 belongs to M3.

This decision means a functioning, tested CPU execution core. It does not certify all hardware behavior, every instruction's physical timing, unknown startup state or every possible game. Preserve accuracy follow-ups and reopen a milestone when a demonstrated defect invalidates its declared scope. There is no known failing CPU acceptance case at this checkpoint.

## Gate-to-evidence mapping

| Documented M2 requirement | Owning evidence reviewed | Conclusion |
| --- | --- | --- |
| Registers, decoding and instruction families | src/cpu/registers.rs, src/cpu/opcode.rs, CPU tests and [coverage record](OPCODE_COVERAGE.md): 244 legal non-prefix base encodings plus all 256 CB | Implemented for declared scope; illegal-opcode hardware lockup is not claimed by the project's sticky diagnostic fault policy. |
| Flags and arithmetic edge cases | src/cpu/tests/register_add_sub.rs uses widened/nibble arithmetic independent of production calculations; ADC/SBC and operand variants have corresponding tests. [DAA evidence](tasks/DECIMAL_ADJUST.md) covers all A/flag inputs and valid packed-decimal combinations. tests/support/cb.rs and CB tests provide result/flag oracles. | Focused edge-case requirement met. Broad external arithmetic/CB certification is still absent and remains additional confidence work. |
| Loads, stack and control flow | Family unit/public tests cover mapped accesses, little-endian words, PC/SP wrapping, taken/untaken branches, captured operands and following instructions. [CB System integration](tasks/CB_INTEGRATION.md) checks all 256 encodings, memory variants and budgets. | Functional behavior and coarse cycle accounting covered. This is not a claim about future DMA/PPU access restrictions. |
| Deterministic stepping and traces | CPU request/completion protocol tests, instruction-step budgets/resume, sticky faults, System traces and test-only bounded runner | Repeatable requests, typed input rejection, completion boundaries and bounded execution have evidence. |
| Interrupt/control instructions | DI/EI/RETI/HALT/STOP tests plus [selected external ROMs](tasks/MOONEYE_CPU_SUBSET.md); CPU/System/Bus ownership remains explicit | Declared entry, delayed EI, HALT and conditional STOP scope covered; physical collision/oscillator limits remain explicit. |
| Selected CPU ROM checks with documented outcomes | [reg_f](tasks/MOONEYE_REG_F.md), if_ie_registers, interrupts/ie_push, ei_sequence and div_timing: pinned artifacts, hashes, result signatures, counts and explicit reproduction | Original selected-ROM gate met. Ignored/default fixture tests were not counted as passes; each fixture actually ran. |

## Independent instruction-result candidates

| Candidate and inspected revision | Dependencies / result protocol | Selection |
| --- | --- | --- |
| [Mooneye acceptance/instr/daa.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/instr/daa.s), same 31510e1 pin | Two tables of 2,048 entries cover all 4,096 A/flag-nibble combinations via POP AF, DAA and PUSH AF, comparing both result and flags. Source marks DMG passing. Uses ROM/WRAM/HRAM and the already-audited common quit path. Suite MIT and separate font provenance are already recorded; actual fixture hash/header/reporting still need verification. | Selected as the smallest next confidence check, [#52](https://github.com/Lorithan/gameboy-emu/issues/52). Non-blocking after M2; no new production interface or dependency is needed. No external DAA run is claimed here. |
| [SingleStepTests/sm83 README](https://github.com/SingleStepTests/sm83/blob/f9c30210245dd691661db39f5ace022c465ecc2f/README.MD), f9c30210245dd691661db39f5ace022c465ecc2f; repository [MIT notice](https://github.com/SingleStepTests/sm83/blob/f9c30210245dd691661db39f5ace022c465ecc2f/LICENSE) | 1,000 JSON cases per legal opcode file, explicit initial/final registers and sparse RAM, plus bus samples. Assumes flat 64 KiB RAM rather than Game Boy mapping. README cautions against using IME fields and describes generation from an Ares-derived implementation, not independent hardware measurement. | Useful broader future result comparison, but needs a test-only CPU vector adapter and ingestion design. Do not feed flat-RAM fixtures through production Bus or equate between-state pin samples with one CycleAction. Not a new M2 blocker. |
| [Blargg cpu_instrs README](https://github.com/retrio/gb-test-roms/blob/c240dd7d700e5c0b00a7bbba52b53e4ee67b5f15/cpu_instrs/readme.txt), c240dd7d700e5c0b00a7bbba52b53e4ee67b5f15 | Broad result/CRC-based instruction groups and ASCII output, excluding STOP/illegal opcodes. The inspected [individual-ROM builder](https://github.com/retrio/gb-test-roms/blob/c240dd7d700e5c0b00a7bbba52b53e4ee67b5f15/cpu_instrs/source/common/build_rom.s) declares MBC1 type 01 even for a 32 KiB image, which our loader intentionally rejects. Needs separate reporting and complete notice/prerequisite audit. | Defer until mapper/reporting prerequisites are deliberately supported, normally M5. Do not patch headers or reuse the six-byte Mooneye observer for ASCII output. This is a dependency, not a CPU failure. |

The Mooneye DAA tables remain an independent regression source even though local exhaustive DAA tests already pass. It is a small useful check, not a reason to hold the whole CPU milestone open indefinitely. Do not implement the larger vector framework merely because data is available.

## Selected follow-up contract

#52 covers one DAA ROM only. Verify its pinned archive/ROM/symbol identity, inherited component notices, unmodified header and common reporting path before execution. Reuse the public-System test runner and checksum-gated script, with explicit missing/malformed/fault/STOP/timeout failure paths. Existing one-million-M-cycle/10-second limits are candidates to review before running, then record actual duration/counts; do not silently raise them. No new fixture, adapter, application API or dependency was introduced during this review. A genuine failure becomes a small user-owned fix with assistant regression acceptance.

If SingleStepTests is adopted later, use an isolated CPU unit-test adapter: initialize registers, supply a test-only flat 64 KiB memory image, perform requested reads/writes with typed responses and compare final registers/listed memory. Keep production Bus unchanged. Initial scope should be ordinary result/flag instructions; settle parsing, invalid flag inputs, bounds and trace normalization before implementation. Do not assert interrupt or pin accuracy from that dataset's cautioned fields.

## Work that belongs to later milestones

- M3: programmable TIMA/TMA/TAC, input selection, overflow/reload/write interactions and generated timer interrupts. Divider and interrupt-service foundations already exist; [#53](https://github.com/Lorithan/gameboy-emu/issues/53) specifies the timer contract before user implementation.
- M4: PPU/VRAM/OAM/DMA and their CPU access interactions. For example, the inspected [Mooneye jp_timing source](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/jp_timing.s) deliberately waits for vblank, copies to VRAM/OAM and times reads against OAM DMA completion. It is not currently runnable as a pure CPU check.
- M5: mapper-dependent validation and MBC1 software.
- [#44](https://github.com/Lorithan/gameboy-emu/issues/44): physical STOP startup/edge/glitch accuracy. Authentic boot execution remains deferred under the declared startup scope. Neither is silently included in M2 completion.
- M0/#1 remains open for staged architecture/device startup review; it does not undo the established CPU boundary or require an emulator rewrite.

M2's original functional gate is met. Additional validation should be tied to specific uncertainty, a new device or an observed defect, rather than keeping the milestone open until an unspecified number of ROMs passes.
