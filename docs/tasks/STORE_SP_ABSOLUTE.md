# Implement LD (a16),SP with two little-endian memory writes

Issue: [#36](https://github.com/Lorithan/gameboy-emu/issues/36). Prepared 2026-11-11.

## Goal
Implement 08 LD (a16),SP after the other remaining loads. Baseline reviewed instructions through 3140521.

## Behavior and proposed cycle contract
Three bytes/five M-cycles: opcode Read, address-low Read, address-high Read, SP-low Write at target, SP-high Write at target+1 (wrapping).
Capture original SP at successful opcode completion; retain both bytes across address fetches and pauses. Read target low byte first; advance PC once per successful instruction-byte read, wrapping. No PC advance on either write. Neither SP nor flags/other registers change. Only final write completion restores FetchOpcode and reports true.
Ordinary Bus policy applies independently to each write, including ignored ROM/unmapped writes and WRAM echo aliases. This is not PUSH: it does not decrement SP or write high first.

## Architecture proposal
Reuse address-fetch concepts with an explicit captured word-store continuation where practical. The first write must retain the high byte and next address; final write can reuse existing WriteMemory. Avoid a general arbitrary-width transfer framework or merging instruction families purely to reduce enum count. Exact representation belongs to user. Preserve no-mutation missing-data retries, request immutability and sticky faults. No Bus/System changes/dependencies.

## Acceptance
Independent mapping/classification; exhaustive SP values and exhaustive target addresses in separate sweeps, distinct bytes and every initial flag. Captured SP/low target and missing-read retries; PC wraps at each fetch; exact five-action traces; no early writes; every budget pause/resume and only one write per stage; next opcode continuation.
Full-memory oracle at bus boundaries/echo aliases and FFFF->0000, including one mapped/one ignored destination. Round trips verified by existing A transfers and mixed stack behavior with SP unchanged. Full suite, formatting, strict Clippy.
Exclude interrupts, CB and refactor implementation. Local contract: docs/tasks/STORE_SP_ABSOLUTE.md.

Reference checked 2026-11-11: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7). User owns implementation; assistant owns tests/docs/review. No dependencies or ROM fixtures.
## Completion - 2026-11-11

No correctness findings. All 307 tests pass (213 unit, 94 integration); formatting and strict Clippy pass. Original SP is captured during opcode completion, address bytes are read little-endian, the dedicated low-write state retains the high byte, and the final high write reuses WriteMemory at wrapped target+1. SP and flags remain unchanged. Assistant changed tests/docs only.

Evidence: independent 08 mapping/classification; all SP values and all target addresses in separate sweeps (131,072 cases, not all source/target pairs), distinct low/high data, exact snapshots and following-opcode continuation. All flags with captured SP under test-only mutation, preserved buffered low address, missing-read atomic retries, immutable requests and all fetch-wrap positions pass. Exact five-action public traces, all zero/partial/exact/excess budgets/resumes and no duplicate write stages pass. Full-address-space oracle covers every pause, independent low/high mapping, WRAM echo aliases and FFFF->0000. A 116-tick program stores the active stack pointer in HRAM, reads both bytes using A loads, and later pops the original stack data without SP corruption. All prior regressions pass. No external ROM suite was run.

This completes all legal non-CB base load/store instruction encodings in the current CPU scope. Bus devices/mappers remain staged; load/store completion does not imply full memory hardware accuracy. Remaining instruction work is the 256 CB entries plus five base interrupt/control instructions and CB dispatch. Specify the next CB slice before implementation. Complete locally, publication pending.