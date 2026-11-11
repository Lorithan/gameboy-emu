# Accumulator memory transfers through BC and DE

Issue: [#31](https://github.com/Lorithan/gameboy-emu/issues/31). Prepared 2026-11-10.

## Goal
Implement A transfers through BC/DE: 02 LD (BC),A; 0A LD A,(BC); 12 LD (DE),A; 1A LD A,(DE). Baseline 84c67d1, 269 passing tests. M2 CPU execution.

## Behavior
Parentheses mean the memory byte at the address held in the pair. Stores copy A to that address; loads copy that byte into A. Do not transfer the pair's numeric value into A or modify the address pair.
All four occupy one byte, take two M-cycles/eight ticks, and preserve all flags and unrelated registers/SP. Ordinary Bus mapping applies: writes to ROM/unmapped regions are ignored under current policy; loads read mapped data/fallback normally.

## Proposed architecture/cycle contract
Decode direction and BC/DE only: four explicit variants or a constrained two-case indirect-pair selector with load/store operations are sufficient.
Do not broaden Target to arbitrary indirect pairs inside the register-transfer matrix: that would permit unsupported combinations and complicate existing decoding. Do not use AF/SP as byte-addressing choices.
Opcode completion advances PC once with wrapping and captures the selected address. For stores also capture A. Reuse existing ReadMemory { addr, destination: A } and WriteMemory { addr, value } states. Second-cycle completion loads A or acknowledges the single write, restores FetchOpcode and reports true. Opcode completion reports false. No operand-byte reads, idle cycle, pair increment/decrement, new execution states, Bus/System changes or dependencies are expected.
Preserve request immutability and no-mutation missing-read retries; numeric addresses/data survive budget pauses.

## Acceptance criteria (assistant tests)
Independent four mappings and all-byte classification; exhaustive address selection per opcode and all-byte data transfers at representative mapped addresses; every initial flag, unrelated register preservation.
Captured BC/DE address and stored A under test-only mutations; immutable requests, opcode/load read retries, PC wrapping, no early load/write. Exact Read/Read and Read/Write traces, false/true completion, every zero/partial/exact/excess budget and resume, continuation to another instruction.
ROM/WRAM/echo/HRAM/unmapped boundary behavior, round-trip programs using both BC and DE, full-address-space preservation for loads and only permitted store/echo effects. Existing HL loads unchanged. Full suite, formatting and strict Clippy.

Start LD (BC),A, then LD A,(BC), then DE. HL auto-increment/decrement, absolute/high-memory loads and LD SP,HL remain separate issues. User implements; assistant tests/docs/review. No ROM fixtures.

Reference checked 2026-11-10: [gbdev opcode data](https://raw.githubusercontent.com/gbdev/gb-opcodes/master/Opcodes.json). Local contract: docs/tasks/BC_DE_MEMORY_LOADS.md.
## Completion - 2026-11-10

All four mappings and their captured-address execution paths are correct; no correctness findings. All 275 tests pass (193 unit, 82 integration), formatting and strict Clippy pass. Assistant changed tests/docs only; no execution states or Bus/System behavior added by the user.

Evidence: independent four mappings and supported-byte classification; all 262,144 opcode/address selections with snapshot, direction, data and flag assertions; every initial flag with opcode/load retries, request immutability, captured address/stored A under test-only mutation and PC wrap; successful continuation to a following NOP. Exact public Read/Read and Read/Write traces, zero/partial/exact/excess budgets and resume pass. Every byte round-trips through BC/DE across WRAM, echo and HRAM. Full-address-space memory oracle at bus boundaries verifies loads preserve memory and stores change only permitted mapped destinations/echo aliases, including before/after pauses. Existing regressions pass. No external ROM suite was run.

Optional readability suggestion: LoadAFromIndirect would name the implicit A destination and mirror StoreAToIndirect. Current LoadFromIndirect is functional and not a completion blocker. Complete locally; publication pending.