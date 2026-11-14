# CB shifts and nibble swap

## Scope

Implement the 32 encodings CB 20-3F: SLA, SRA, SWAP and SRL for B, C, D, E, H, L, (HL), A. Begin with SLA and extend after review. The user owns implementation; the assistant owns tests. BIT, RES, SET and interrupt/control instructions remain separate work.

## Proposed design

Reuse Target and the existing CB register and captured-address memory paths. No new execution states should be necessary. Extend CbType with operation selectors for the three shifts and swap; keep the shift selector distinct from RotationDirection so it cannot express nonexistent combinations. Arithmetic helpers return a result and outgoing carry; callers own flags, target writes and instruction completion. Keep TryFrom rejection for CB 40-FF until those families are implemented.

## Behavior

- CB 20-27 SLA: shift left, insert zero into bit 0, original bit 7 becomes C.
- CB 28-2F SRA: shift right, preserve original bit 7, original bit 0 becomes C.
- CB 30-37 SWAP: exchange the high and low four-bit nibbles; C is cleared.
- CB 38-3F SRL: shift right, insert zero into bit 7, original bit 0 becomes C.
- All four operations set Z iff the result is zero and clear N/H. Incoming C does not affect the result.
- Instructions consume two bytes. Register targets complete in two M-cycles; (HL) targets complete in four: prefix read, second opcode read, captured-address data read, write to that same address.
- Required read failures leave pending work and architectural state unchanged; PC advances only after successful instruction-byte reads and wraps at FFFF.
- Unsupported CB instructions retain the sticky typed fault and prefix address contract.

## Acceptance evidence required

Independent mapping checks for all 32 bytes; all 256 input values and all 16 initial flag combinations for each target; preservation of other registers and SP; unchanged A for non-A targets; SRA/SRL distinction at 80 and 81; nibble swap examples F0->0F and A5->5A; exact cycles, retry and PC wrapping; captured HL write address; System traces and budget resumes with no early or duplicate memory writes. Existing base and CB rotates must remain green. Run full tests, formatting and strict Clippy after implementation.

The existing CB rotate tests that expect bytes 20-3F to be unsupported must be updated as support is added, preserving negative coverage for 40-FF.

## Reference

RGBDS CPU reference: https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7 (SLA, SRA, SRL, SWAP sections).

## Local checkpoint - 2026-11-13

All 32 encodings are implemented. All 319 tests pass (225 unit, 94 integration); formatting and strict Clippy pass. New shift checks exhaust all values and initial flags for A and (HL), including result/flag oracles and memory cycle completion. SWAP checks cover all eight targets, all values and initial flags, preservation of other registers/SP, and two/four-cycle completion. Existing CB rotates, retries, captured-address and typed sticky fault tests pass. No application code was changed by the assistant.

This is an implementation checkpoint, not completion of every acceptance item above: dedicated System traces/budget/memory checks, exhaustive shifts across every register target, and operation-specific wrap/retry/capture extensions remain unverified. No external ROM suite was run. Publication is not verified.
