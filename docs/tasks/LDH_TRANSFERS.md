# Implement LDH accumulator transfers through C and immediate offsets

Issue: [#34](https://github.com/Lorithan/gameboy-emu/issues/34). Prepared 2026-11-11.

## Goal
Implement E2 LDH (C),A; F2 LDH A,(C); E0 LDH (a8),A; F0 LDH A,(a8). Baseline 3140521, 289 passing tests. Begin C forms, then immediate forms.

## Behavior and proposed cycle contract
Address is FF00 plus unsigned C or the immediate operand byte, covering FF00-FFFF. The H in LDH means high-memory addressing; it is not the H flag. Preserve every flag and unrelated registers/SP. A changes only for loads, C remains unchanged.
C forms: one byte/two M-cycles (opcode Read then data Read/Write). Capture C-derived address and original A for stores at opcode completion.
Immediate forms: two bytes/three M-cycles (opcode Read, offset Read, data Read/Write). Capture store A at opcode completion; successful offset read advances PC once and resolves the address.
All instruction-byte PC advances wrap. No idle cycle or extra fetch; final data completion restores FetchOpcode and reports true. Missing reads are retryable without mutation; address/store data remain captured across pauses.

## Architecture proposal
Explicit high-memory load/store operations with a constrained C/immediate address-source selector, or four dedicated decoded variants. Keep high-memory addressing out of IndirectPair. Reuse final ReadMemory/WriteMemory states. Only immediate forms need an offset-read stage; do not force C forms through it.
Use current Bus semantics. HRAM FF80-FFFE works; unimplemented I/O and IE still use FF-read/ignored-write fallback. This task adds CPU addressing, not device behavior or interrupt registers.

## Acceptance
Independent mappings and all 256 offsets for every form; all byte values/flags at mapped addresses; captures under test-only mutation, missing reads, PC wrap and no early effects. Exact traces and every budget pause/resume; next opcode continuation.
Cross-check C/immediate forms and equivalent absolute addressing. HRAM round trips, FF00/FF7F/FF80/FFFE/FFFF boundaries and full-memory/permitted-store oracle. Preserve HL/BC-DE/absolute transfer regressions. Full tests, formatting, strict Clippy.

Exclude device registers, interrupts, CB, F9 and 08. Local contract: docs/tasks/LDH_TRANSFERS.md.

Reference checked 2026-11-11: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7). User owns implementation; assistant owns tests/docs/review. No dependencies or ROM fixtures.
## Completion - 2026-11-11

No correctness findings. All 295 tests pass (205 unit, 90 integration); formatting and strict Clippy pass. The user clarified HighMemoryTarget to HighMemoryAddressSource and both decoded fields to address_source during review and removed an unused Values import. New test references were adapted to the live rename before the successful full run. Assistant changed tests/docs only.

Implementation review: C-derived addresses and store A are captured at opcode completion, C forms enter final data states directly, immediate forms use one additional byte-read state, successful offset reads advance PC once, and all final data accesses reuse ReadMemory/WriteMemory. OR with FF00 is correct for the constrained byte offset. Option<u8> cleanly represents load/no store byte versus a captured store payload; a dedicated read/write enum would be optional clarity work rather than a correctness requirement. No general address framework is warranted.

Evidence: independent four mappings/all-byte classification; all 262,144 opcode/offset/data combinations with high-page, flags and unrelated-state assertions; every initial flag with captured C/store A under private test-only mutation, repeated requests, opcode/offset/data retries and PC wrapping. Exact public two/three-cycle traces, no early effects and all zero/partial/exact/excess budgets/resumes pass. Every offset is cross-checked against absolute transfers using three representative data values. Full-address-space memory oracle verifies only mapped HRAM changes at final store completion; FF00/FF7F/FF80/FFFE/FFFF fallback boundaries are covered. All prior regressions pass. This validates CPU addressing with current Bus behavior, not hardware I/O or interrupt register implementation. No external ROM suite was run.

Complete locally; publication pending. Next: #35 LD SP,HL, then #36 LD (a16),SP.