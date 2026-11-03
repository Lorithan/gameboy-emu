# CPU execution-state refactor

Status: implemented and reviewed within #8 on 2026-11-03. The implementation calls the byte destination type `Destination` and its memory variant `Memory(u16)`. The CPU/System boundary and supported opcode set are preserved. No additional dependencies.

## Problem

The existing states mix the source of data, its destination, and the operation's history. `StoreImmediate` and `WriteMemory` both request a memory write, while `FetchImmediateStore` and `FetchImmediate` both read an instruction operand. `store_fetched` and `store_immediate` duplicate register assignment but differ in PC behavior. These differences caused the recent transition and PC bugs.

## Proposed types and responsibilities

Use execution-state names for the next pending cycle. Use named fields for destination/address/value rather than ambiguous tuple fields. Retain separate byte and word register selectors.

| State | Retained data | Next action and completion |
| --- | --- | --- |
| FetchOpcode | none | Read PC, advance PC, decode and choose continuation |
| ReadImmediate8 | ByteDestination | Read PC, advance PC, assign register and finish OR queue memory write |
| ReadImmediate16Low | WordValue destination | Read PC, advance PC, retain low byte |
| ReadImmediate16High | WordValue destination, low byte | Read PC, advance PC, assign pair/SP and finish |
| ReadMemory | address, RegisterValue destination | Read captured address, assign byte register and finish without advancing PC |
| WriteMemory | address, value | Write captured data, finish without advancing PC |
| Faulted | opcode, original address | Return stored fault, no action or time |

`ByteDestination` has only two forms: `Register(RegisterValue)` and `Memory { address: u16 }`. The latter holds an already resolved address, not an HL selector. It describes where an immediate byte will go; it is not a universal operand or addressing-mode type. This keeps immediate-to-register and immediate-to-memory reading in one state while retaining their different completion behavior. Word destinations continue to exclude memory and AF.

## State transitions for current memory instructions

- LD A,(HL): FetchOpcode -> ReadMemory(address=HL, destination=A) -> FetchOpcode.
- LD (HL),A: FetchOpcode -> WriteMemory(address=HL, value=A) -> FetchOpcode.
- LD (HL),d8: FetchOpcode -> ReadImmediate8(destination=Memory(address=HL)) -> WriteMemory(address, value=operand) -> FetchOpcode.

Capture HL and, for register stores, the source byte when decoding the opcode. This is the initial project model, not a claim about physical register sampling edges. Once an address is resolved, later data cycles do not need to know its source.

## Helper boundaries and invariants

- Register byte read/write helpers only select/read/assign a register. They do not move PC, change execution state, or mark completion. These are proposed internal helpers, not a request for public accessors.
- Cycle handlers own instruction progress. Keep wrapping PC advancement explicit for opcode/operand reads and absent for data-memory cycles.
- `request_cycle` projects state into one action immutably. It performs neither register mutation nor Bus access.
- Validate read data before mutation. Missing data reports the requested address and preserves registers, pending destination, buffered bytes, and state.
- Completing either memory write returns to FetchOpcode. System alone performs the write; completion does not write it again.
- Continue fetching raw operand bytes, not decoded instructions, in operand states.
- Preserve sticky unsupported-opcode behavior and all existing timing, traces, instruction budgets, and public interfaces.

## Extending later

This is a small state machine for the supported load instructions, not the final universal instruction engine. Future arithmetic, read-modify-write, stack operations, and internal cycles will require additional continuations/states. Add those with their tasks; do not prebuild a generic micro-operation queue or add unused enum variants. Future BC/DE or immediate addresses can reuse resolved data-memory actions, but different operand-fetch sequences and side effects still need explicit handling.

## Evidence required

The original 95 tests remain green after adapting private-state references. Captured address/data tests and the completed #8 acceptance coverage bring the suite to 102 passing tests (77 unit, 25 integration). Coverage includes all-byte round-trips, traces, budgets, mapping boundaries, flags, and retry/wrap behavior. Formatting passes. Strict Clippy remains blocked by unread CpuFault fields and unused set_af; no new suppressions were added.
