## Goal
Reduce repeated accumulator-arithmetic decoding and execution dispatch while preserving all existing behavior. Milestone M2; follows #15, committed locally as bee1ed0. This is a refactor with no new supported instructions.

## Proposed design
- Represent supported accumulator operation semantics explicitly: Add, Adc, Sub, Sbc, Compare. Do not combine a general kind with an independent with_carry bool: combinations such as Compare-with-carry are not instructions.
- Represent a byte source explicitly: Register(RegisterValue), IndirectHL, Immediate8. Keep this separate from writable targets and word registers.
- Use one decoded accumulator-arithmetic variant containing operation kind and source. This replaces the current register/memory/immediate variants for ADD/SUB/ADC/SBC/CP.
- The decoder stays pure: no register, flags, Bus, PC, or time access.
- At opcode completion, resolve the source. Register operations finish immediately; indirect/immediate operations queue the existing corresponding accumulator-read state.
- Resolve incoming carry at entry to execution: ADC/SBC capture the current C; ADD/SUB/CP capture false. For pending reads, retain original A, the numeric carry input, operation kind, and any captured HL address.
- Keep semantic operation kind distinct from saved execution input. The existing AddC(bool)/SubC(bool) payloads can be replaced with a captured-carry field in the two pending accumulator states as part of this refactor.
- Use one shared accumulator-operation execution helper from register and read-completion paths. It owns arithmetic flags and A assignment where applicable. CP updates flags only. It does not advance PC, change execution state, access Bus, or clock the system.
- Preserve missing-read validation before mutation, exact PC advancement, original opcode fault address, completion states, timing, traces, budgets, and read-only memory arithmetic.
- Use bit-field decoding for currently supported accumulator groups if desired: 0x80–0x9F cover ADD/ADC/SUB/SBC; 0xB8–0xBF covers CP. Source field order is B,C,D,E,H,L,(HL),A. Keep all unsupported groups unsupported.
- Immediate forms remain explicitly mapped (C6, CE, D6, DE, FE).
- Leave byte-transfer and unary decoded operations unchanged in this task; their bit-field conversion can be separate.
- Retain current From/Unknown classification and cycle response/outcome APIs. Do not combine unrelated interface or ownership changes with this refactor.
- No new dependencies, opcodes, public API, or System/Bus changes.

## Acceptance criteria — assistant-owned verification
- [x] Existing 152 tests remain green after adapting private decoded/state type references.
- [x] Public integration tests remain unchanged.
- [x] Independent opcode matrices still verify every source/operation mapping; all 256 raw bytes retain their previous supported/unsupported classification.
- [x] Exhaustive arithmetic, flags, comparison preservation, and self-source tests pass.
- [x] Pending A/address/carry capture regressions, missing-read retries, and PC wrapping pass.
- [x] CP never assigns A; memory operands never write; INC/DEC still preserve C; ADD/SUB/CP still ignore incoming C.
- [x] Exact timing, instruction boundaries, snapshots, trace contents, and resume budgets remain unchanged.
- [x] Unsupported faults still consume only their opcode cycle, retain the original address, and remain sticky.
- [x] Formatting passes and no new Clippy warnings appear. Existing unused-code warnings are reported separately.

## Suggested sequence
First introduce semantic operation/source types and consolidate only the decoder output and CPU dispatch, keeping the explicit opcode table. Then unify the repeated accumulator execution match. Finally optionally switch the supported accumulator groups to bit-field decoding. The assistant adapts the private tests and runs each checkpoint; the user writes implementation.

## References
[Pan Docs instruction groups](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) and the existing documented execution/carry contracts in docs/tasks/ADC_SBC.md and docs/tasks/OPERAND_ADD_SUB.md.

## Scope and review update — 2026-11-04

The user expanded the refactor to byte loads and unary decoding as well as accumulator arithmetic. The implemented ArithmeticKind::{Add { carry }, Sub { carry }, Compare} represents the five semantic operations without permitting compare-with-carry; this is an accepted alternative to separate Add/Adc/Sub/Sbc variants. Pending AccumulatorOperation::{Add(bool),Sub(bool),Compare} retains the resolved numeric carry input. Decoder policy and captured input remain distinct.

Current evidence: 152 tests pass (111 unit, 41 integration), public integration tests unchanged, formatting passes. The independent all-byte supported/unsupported classification, load matrix, unary/accumulator arithmetic, and captured-state/fault regressions remain green. Clippy reports unused set_af and set_pc; fault opcode/address are now publicly inspectable, and ordinary register setters are scoped internally.

Final cleanup is complete:
- execute_accumulator is called from register, immediate, and memory paths. It receives resolved operation and two bytes, updates flags/A as appropriate, and leaves PC/state/timing alone. Compare changes flags only.
- ArithmeticKind is reused to hold the resolved operation in pending states: the decoder's carry policy is resolved once at opcode execution into actual carry input. Arithmetic helpers consume that captured value directly; there is no second live-C check.
- Opcode 0x36 now decodes as LoadImmediate { destination: Target::IndirectHL }; redundant StoreImmediateHL and its execution arm are removed. The assistant updated that private decoder assertion; public behavior tests were unchanged.

Further selector renaming, bit-field compression for unary/immediate tables, required-read helpers, and response/ownership API redesigns are deferred. No new functionality or dependencies.

## Completion review — 2026-11-04

All 152 tests pass (111 unit, 41 integration), formatting passes, and no correctness findings remain. Existing independent opcode matrices and all-byte rejection checks verify unchanged supported instructions; exhaustive arithmetic/comparison tests and cycle traces/budgets/captures remain green. Clippy reports unused set_af and set_pc only. Committed locally as 32aef0f; not yet pushed. Stop this refactor at the verified checkpoint; relative jumps are specified next in RELATIVE_JUMPS.md and issue #17.
