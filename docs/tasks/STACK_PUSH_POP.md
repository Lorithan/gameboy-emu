# Stack PUSH and POP

Issue: [#20](https://github.com/Lorithan/gameboy-emu/issues/20), M2 CPU execution. Dependency #19 is complete in local commit 6d04444. Baseline: 175 tests pass (125 unit, 50 integration).

## Required behavior

| Pair | PUSH | POP |
| --- | --- | --- |
| BC | C5 | C1 |
| DE | D5 | D1 |
| HL | E5 | E1 |
| AF | F5 | F1 |

Each is a one-byte instruction. PUSH takes four M-cycles (16 ticks); POP takes three (12 ticks). Both advance PC only once, for the opcode. SP is a 16-bit wrapping address, with no host panic or stack overflow/underflow error. Memory accesses use the normal Bus mapping, even outside HRAM.

For initial SP=S and word 0x1234, PUSH writes 0x12 at S-1 and 0x34 at S-2, ending with SP=S-2. Thus the finished stack word is stored in little-endian order. POP reads the low byte at S and the high byte at S+1, ending at S+2. PUSH preserves registers/flags; POP preserves everything except SP and its destination. POP AF restores Z/N/H/C from bits 7–4 of the low byte, discarding bits 3–0 through existing set_af.

For example, PUSH BC with BC=0x1234 and SP=0xFFFE writes 0x12 to 0xFFFD, then 0x34 to 0xFFFC; final SP=0xFFFC. POP BC from there restores 0x1234 and SP=0xFFFE.

Sources: [RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7) for behavior/timing/flags and [Pan Docs encoding](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) for pair selectors/opcodes.

## Architecture proposal

Introduce a stack-register selector containing BC/DE/HL/AF. Keep it distinct from Register16, which represents BC/DE/HL/SP immediate-load destinations. This prevents invalid PUSH SP or LD AF,a16 combinations. Decode PUSH and POP with that selector. Names are implementation choices.

Keep ordinary CPU-owned cycle payloads and existing System/Bus routing. Dedicated stack states are appropriate: existing WriteMemory completes the instruction after one byte and cannot express another pending write. Do not build a general subroutine or microcode framework in anticipation of later tasks.

### PUSH cycle contract

1. Opcode completion captures the source word and initial SP and queues an Idle state. No SP or memory change yet.
2. Idle completion queues the high-byte write at captured S-1. SP remains S.
3. High-byte write completion sets SP to S-1 and queues the low-byte write at S-2. The instruction remains incomplete.
4. Low-byte write completion sets SP to S-2, returns FetchOpcode, and completes.

Carry captured addresses/data through these states. No subsequent cycle rereads the source register pair or depends on live SP. Requests compute/return actions without mutation; repeated requests cannot decrement SP or write memory. Write/Idle completions need no read byte. Even ignored bus writes advance the CPU normally.

### POP cycle contract

1. Opcode completion captures destination and initial stack address S and queues the low-byte read. SP and destination are unchanged.
2. Successful low-byte read retains the byte and destination, sets SP=S+1, and queues the high-byte read at captured S+1. Keep the destination pair unchanged.
3. Successful high-byte read assembles the word, assigns the destination (using set_af for AF), sets SP=S+2, returns FetchOpcode, and completes.

Read states retain their requested addresses. Missing data returns MissingReadValue at that address with no mutation of SP, destination, flags, PC, or pending payload, permitting retry. Buffer the low byte across pauses/retries; no partial destination-pair assignment.

SP updates at bus-cycle completion and atomic POP destination assignment are our initial M-cycle observation conventions, not claims about physical edge timing. Tests will use these boundaries consistently. OAM corruption, finer device interactions, and physical internal register timing remain deferred.

## Assistant-owned acceptance tests

- Independent eight-opcode mapping and updated all-byte classification; no additional instructions accepted.
- Every 16-bit source/POP input word, with masked AF expectations; all initial flag combinations. PUSH never alters flags; POP BC/DE/HL preserves them; POP AF restores the masked input.
- Exact opcode/Idle/Write/Write versus opcode/Read/Read traces, high-first push and low-first pop, per-cycle SP and destination snapshots, ticks/completion markers, and no extra fetch.
- PC wrap at opcode fetch; SP wrap at 0000/0001/FFFF; memory boundaries and ordinary ignored-write/read-FF bus behavior outside mapped storage.
- Repeated requests, missing opcode/low/high data, retry preserving the first byte and destination, and captured PUSH source/address across test-only live-register changes.
- Zero/partial/exact/excess budgets, pauses after every phase, per-call counts, and resume without duplicate writes or decrements.
- Nested LIFO round trips in default HRAM and WRAM, distinct pairs, AF masking, and no corruption of other memory. No dedicated stack bounds or hidden storage.
- Full regression suite, formatting and reported Clippy warnings. Synthetic programs only; no new dependencies or ROM fixtures.

## First implementation slice

Implement PUSH BC first, then POP BC, and review those complete cycle paths before adding DE/HL/AF mappings. User implements; assistant writes tests. CALL, RET, RST and interrupts remain separate tasks.

## PUSH BC first review — 2026-11-04

Source capture, separate pending states, the Idle cycle, and completion markers follow the intended structure. Two findings remain: both writes currently use initial SP instead of S-1 then S-2, and write completion decrements live SP rather than assigning the captured address. Keep addr in each write state equal to the actual write destination, advance that payload with wrapping between writes, and commit SP from it.

Assistant added independent C5 mapping, exhaustive-word byte-order/state checks, SP/PC wrap and test-only capture perturbation checks, public cycle traces and instruction budget resumes. At this review 126 of 128 unit tests pass; the address and captured-SP regressions fail. One of two new public tests passes (budget counts), while the write-address trace test fails. Formatting passes. No implementation edits by assistant; issue #20 remains open and POP/other pairs are not yet implemented or accepted.

## BC POP review — 2026-11-04

PUSH address/SP findings and wrapping subtraction are corrected. POP's initially repeated low-byte address was corrected during review: high reads now use S+1. New private-state tests verify all four destination handlers for every 16-bit assembled word (including AF masking), flags/wrap/retry/captured-address behavior, and atomic destination assignment. These pass. At this checkpoint 130/131 unit tests pass; the remaining failure is missing C1 decoder mapping. Both new public POP BC tests stop at that unsupported opcode, so end-to-end POP acceptance remains pending. Only PUSH BC is mapped; the other six stack opcodes are later work. Assistant changed tests/docs only.

## BC checkpoint accepted — 2026-11-04

C1 now decodes correctly. All 186 tests pass (132 unit, 54 integration), including every POP BC word with all 16 initial flag combinations through the decoder (1,048,576 cases), exhaustive PUSH BC words, captured stack payloads, missing-read retries, PC/SP wrap, public read/write traces and per-cycle snapshots, and all budget pause/resume points. Public BC round trips restore data and SP in HRAM and WRAM. Formatting passes; Clippy reports only the unused DE/HL/AF selector variants. No correctness findings remain for BC.

Next add PUSH D5/E5/F5 and POP D1/E1/F1 for DE/HL/AF. Existing private tests exercise their POP destination handlers, but opcode-based source selection, AF end-to-end masking and nested distinct-pair LIFO behavior still need review/tests after mappings exist. Issue #20 stays open. Changes are uncommitted; assistant changed tests/docs only.

## Completion review — 2026-11-04

All eight opcode mappings are correct and all cycle paths follow the contract. No correctness findings remain. All 192 tests pass (135 unit, 57 integration); formatting and Clippy pass without warnings. The production use of set_af and all stack selectors removes the previous unused-code warnings. No warnings were suppressed.

New independent mappings, exhaustive source/popped-word selection through all eight opcodes (262,144 pair/word cases), captured PUSH sources, AF masking and the earlier all-flags/wrap/retry tests pass. Public tests cover every budget pause for all pairs, nested distinct-pair LIFO round trips with overwritten live registers in HRAM and WRAM, and every raw popped flag byte with representative A values followed by masked PUSH AF writes.

A full-address-space memory oracle checks all pairs across stack/bus boundaries at every PUSH pause, including wrapping SP, ROM/unimplemented ignored writes, WRAM/echo aliasing and HRAM. POP uses normal bus data and preserves every memory location. Existing arithmetic, jump, fault and memory regressions remain green. User wrote implementation; assistant changed tests/docs only. Issue #20 is complete locally, pending commit/push. CALL/RET/RST remain separate work.
