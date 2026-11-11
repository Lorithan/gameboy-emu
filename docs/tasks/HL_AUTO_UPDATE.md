# HL auto-update accumulator transfers

Issue: [#32](https://github.com/Lorithan/gameboy-emu/issues/32). Prepared 2026-11-10.

## Goal
Implement 22 LD (HL+),A; 2A LD A,(HL+); 32 LD (HL-),A; 3A LD A,(HL-). Baseline 363b803, 275-test behavior baseline; naming-only follow-up focused tests/formatting/Clippy verified. M2 CPU execution.

## Behavior
Access the memory byte at original HL, then update HL by +1 or -1 with 16-bit wrapping. Stores write original A; loads write the read byte into A. Preserve every flag and BC/DE/SP. Each is one byte/two M-cycles/eight ticks; no separate INC/DEC instruction or extra Idle.
HL+ / HL- are post-update addressing modes, not memory contents plus/minus one. Normal Bus mapping remains in effect, including ignored writes; HL still updates when a write targets ROM/unmapped memory.

## Proposed architecture and timing
Decode transfer direction plus a constrained increment/decrement mode with A and HL fixed.
Opcode completion advances PC once with wrapping and captures original HL, resolved next HL and store data when applicable. It reports incomplete; A/HL/flags remain unchanged.
The data cycle requests Read or Write at original HL. Successful completion loads A or acknowledges the write, assigns captured next HL, restores FetchOpcode and reports complete. Missing read data leaves A/HL/PC/pending state unchanged. Captures survive pauses and repeated requests; never derive final HL from a subsequently modified live register.
Keep ordinary memory transfers unchanged. Two focused pending read/write states carrying the HL update are a straightforward proposal; an explicit constrained completion action on shared states is also reviewable if it simplifies real duplication. Do not add a generic callback/micro-operation system or blanket HL updates to all memory accesses. No Bus/System changes or dependencies expected. The exact representation belongs to the user.
The final data-cycle commit follows our initial M-cycle abstraction, not a physical-edge accuracy claim.

## Acceptance criteria (assistant tests)
Independent four mappings/all-byte classification; all original HL values per opcode including FFFF->0000 and 0000->FFFF; all A/data bytes at representative addresses; every initial flag. Confirm data access uses original HL and no early HL/A change.
Captured address/next HL/store A under private test-only mutation; immutable requests, missing-opcode/load retries, PC wrapping. Exact Read/Read and Read/Write traces, false/true completion, all budgets/resumes with one HL update only, continuation to another instruction.
Bus boundaries, ROM ignored writes still advancing HL, full-memory preservation for loads/only permitted store+echo effects. Forward/backward buffer programs using both transfer directions; ordinary HL and BC/DE transfers remain unchanged. Full regressions, formatting, strict Clippy.

Start 22 store/increment, then 2A load/increment, then decrement forms. Absolute/high-memory transfers, LD SP,HL and interrupts/CB remain separate. User implements; assistant tests/docs/review. No ROM fixtures.

Reference checked 2026-11-10: [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7), HLI/HLD load sections. Local contract: docs/tasks/HL_AUTO_UPDATE.md.
## Completion - 2026-11-10

No correctness findings. All 282 tests pass (197 unit, 85 integration); formatting and strict Clippy pass. User added two focused pending data states and reused captured address/data. Calculating next HL from immutable addr plus the operation is equivalent to separately capturing next HL and is accepted; live HL is not consulted.

Evidence: four independent mappings/all-byte classification; all 262,144 opcode/HL combinations with wrapped update, original-address action, flags/unrelated state and next-opcode checks. All initial flags with opcode/load retries, request immutability and private test-only changes to HL/A prove captured address/update/store data. Exact public Read/Read and Read/Write traces, no early A/HL changes, every zero/partial/exact/excess budget and resume pass. Every byte round-trips through both directions across WRAM, echo and HRAM. A 96-tick program fills a buffer forward and reads it backward. Full-address-space oracle verifies loads preserve memory and stores only alter allowed destinations/echo aliases; ignored ROM/unmapped writes still update HL. All prior regressions pass. No external ROM suite was run. Assistant changed tests/docs only.

Optional future design cleanup: store_fetched both loads the byte and finishes the instruction; the new read handler then assigns HL and repeats FetchOpcode/completion. This is correct and atomic for missing reads, but it splits completion ownership. During the separately proposed transition-return refactor, separate the read/register update from instruction finalization so one place owns the final outcome. Do not introduce a new framework or refactor working behavior solely to remove these duplicate lines.

Complete locally, publication pending.