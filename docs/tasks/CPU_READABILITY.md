# CPU structure and readability cleanup

## Shared immediate-word checkpoint verified - 2026-11-19

User added Imm16Use and shared ReadImm16Low/High, removed the six old Low/High pairs and wired LoadImmediate16, AbsoluteJump, Call, LoadAFromAbsolute, StoreAToAbsolute and StoreStackPointer. Each validated operand read increments PC once; the high handler assembles from_le_bytes and calls finish_imm16. CALL captures the current PC after that increment and splits it with to_le_bytes. Existing branch/stack/store captures and downstream cycles are preserved; no wrapper completion or interrupt changes. Computing return-address bytes for untaken CALL is harmless and has no side effect.

Assistant migrated the two register-word state setups to Imm16Use::LoadRegister and retained all independent result/flag/trace/capture/retry expectations. Full suite passed 442 tests (317 unit, 125 integration), explicitly including five external fixtures with pinned hashes checked. User then removed the unused store_immediate_word helper; this was the only remaining application change and resolves the strict Clippy failure. Final formatting, strict Clippy and whitespace checks pass. No full suite repetition was needed for deleting an uncalled private function. No assistant application edits or new dependencies. [#59](https://github.com/Lorithan/gameboy-emu/issues/59) records this checkpoint; #54 ALU extraction remains queued, and stack push consolidation requires its own contract.

## Fetch-state suppression checkpoint verified - 2026-11-19

User completed #58 by replacing the CPU latch with FetchOpcode { skip_increment }. Ordinary construction/completion/wake uses false; HALT suppression uses true. Request matching ignores the payload, while validated completion passes it to read_opcode. The original opcode address is still captured before increment/suppression and decode handling; returned transitions consume the fetch state through the wrapper, with repeated HALT allowed to rearm. Interrupt acceptance reads the same payload and preserves the wrapping return-PC rewind before capturing entry PC/SP. Rejected arbitration and wrong-kind responses preserve state.

Assistant migrated the five affected test files to typed fetch setup/assertions while preserving independent PC/register/bus/fault/cycle expectations. STOP tests no longer manufacture a suppression flag alongside Stopped, a combination the new type cannot represent; stopped wake still checks ordinary fetch, register/IME preservation and idempotence. A new arbitration matrix checks all 256 pending bytes, three IME modes, both skip values and PC wrap boundaries: rejected calls preserve the whole CPU; accepted calls capture the correct PC/SP and cannot rewind a second time. Existing HALT, STOP, CB/immediate, EI;HALT/cancellation, fault and retry tests remain intact.

All 442 tests pass (317 unit, 125 integration), explicitly including five external fixtures whose hashes match pinned manifests. Formatting, strict Clippy and whitespace checks pass. No assistant production edits, dependencies or public API changes. Next: review typed continuations for shared immediate-word read states before user implementation, preserving each operand's capture timing and taken/untaken cycles. #54 ALU extraction remains a separate queued option; no larger state consolidation is implemented here.

## 16-bit register ownership checkpoint verified - 2026-11-19

User completed #57 with Registers::get16/set16 for BC/DE/HL/SP, delegating to existing pair methods and SP storage. The four CPU matches in Unary16 decode/completion, AddHL decode and store_immediate_word were replaced without moving captured operands, arithmetic, flags, PC increments or completion transitions. All 441 unchanged tests pass (316 unit, 125 integration), explicitly including five external fixtures with pinned hashes rechecked. Formatting, strict Clippy and whitespace checks pass. No additional tests or assistant application edits were needed for this mechanical mapping move.

Next proposed slice is [#58](https://github.com/Lorithan/gameboy-emu/issues/58): replace CPU's separate suppression bool with FetchOpcode { skip_increment: bool }. Ordinary fetch construction/completion/wake uses false; HALT suppression uses true. Request matching ignores the payload; validated fetch completion passes it to read_opcode and preserves original-address capture followed by increment/suppression before decode handling. Wrapper transitions naturally consume the old fetch state; rejected responses preserve it. Repeated HALT may arm suppression again. Enabled eligible interrupt acceptance at that fetch boundary must preserve the existing wrapping return-PC rewind when skip_increment is true, then capture entry PC/SP; rejected arbitration must preserve PC and state. Delayed EI, STOP, faults, bus actions and cycle counts remain unchanged. Assistant owns migration of private-state tests and keeps independent behavioral oracles intact; run the full fixture-inclusive suite before publication. State consolidation and ALU extraction remain separate.

## Byte-register ownership checkpoint verified - 2026-11-19

User moved all seven byte-register selector mappings from CPU into Registers::get/set and updated CPU callers without moving arithmetic or completion effects. JumpCondition::is_met now takes a Registers reference and queries get_flag; the four conditional JR/JP/CALL/RET sites retain their evaluation timing. Registers::f is private again. Existing instruction-result, pair aliasing, flag, branch and protocol tests remain unchanged; all 441 tests pass (316 unit, 125 integration), including all five pinned external fixtures with hashes rechecked. Formatting, strict Clippy and whitespace checks pass. No assistant application edits or new tests.

The remaining register-ownership slice is [#57](https://github.com/Lorithan/gameboy-emu/issues/57): add get16/set16 for BC/DE/HL/SP and replace the four selector matches in Unary16 completion, Unary16 decode, AddHL decode and store_immediate_word. Reuse current pair methods, preserve operand capture/write timing, flags, arithmetic wrapping and CPU-owned PC/state effects. AF/stack selectors remain separate. Verify with existing 16-bit instruction/pair/retry tests and the full fixture-inclusive suite; do not combine with subsequent state or module consolidation.

## Infallible helper checkpoint verified - 2026-11-19

For #56, user changed store_immediate, store_fetched and store_immediate_word to return CycleTransition directly. Callers wrap returned transitions in Ok; response.read_value()? still validates input before mutation. Immediate reads increment PC once, ReadMemoryAndUpdateHL retains captured-address wrapping, and only the shared wrapper applies completion/state effects. That HL branch still discards store_fetched's InstructionFinished value; replacing the call with set_register remains optional readability polish, not a behavioral defect.

All 441 existing tests pass (316 unit, 125 integration), explicitly including the five external fixtures whose hashes match the pinned manifests. Formatting, strict Clippy and whitespace checks pass. Existing instruction, retry/protocol, interrupt and trace oracles cover this mechanical change; no helper-mirroring tests or assistant application edits were added.

Next proposed slice: move typed get/set/get16/set16 register selection onto Registers, using its existing pair accessors/setters to preserve F masking and byte order. Update CPU callers without moving PC increments or changing when values are captured. JumpCondition can query get_flag through a Registers reference, allowing f to become private again. StackRegister renaming, fetch suppression/state consolidation and ALU extraction can remain independent. User owns implementation; assistant reviews the diff and runs existing register/condition/instruction/protocol tests and the full fixture-inclusive suite before publication.

## Decoder validity checkpoint verified - 2026-11-19

User completed CB From and base TryFrom in a single reviewed checkpoint for #55. All CB inputs remain valid; UnsupportedCbOpcode and its impossible runtime fault branch are removed. Base decoding returns IllegalOpcode containing the original byte for exactly D3, DB, DD, E3, E4, EB, EC, ED, F4, FC and FD. The CB prefix remains a valid base dispatch byte; legal instruction encoding coverage remains 500/500. IllegalOpcode has Display/Error as a decoder leaf. The preexisting inner CB selector unreachable arm is mathematically bounded by its three-bit mask; replacing that match with a selector table is optional follow-up work, not an executable failure found in this review.

read_opcode saves the original address, performs the established successful-fetch PC increment or suppression consumption, then matches the decode Result. An illegal byte returns CycleTransition::Faulted; the shared wrapper installs the sticky state and reports CpuError without maturing delayed EI. This resolves the ordering regression noted below. No public runtime error shape or CPU instruction timing changed.

Assistant migrated known-valid decoder assertions to TryFrom(...).unwrap(), replaced the old skip-heavy Unknown classifier with an independent exact eleven-byte rejection list and changed EI coverage to is_err classification. Existing CB mapping checks now use From. Two new tests cover IllegalOpcode's standard leaf reporting and invalid fetches across all illegal bytes, PC wrapping, IME modes, suppression, rejected responses and sticky subsequent faults. All 441 tests pass (316 unit, 125 integration), explicitly including five external fixtures; formatting, strict Clippy and whitespace checks pass. No assistant application edits or dependencies. Earlier failure notes are historical.

Next #56 removes Result from private helpers that cannot fail. Proposed boundaries: store_immediate/store_immediate_word can return CycleTransition, wrapped in Ok at fallible callers; store_fetched can be replaced by existing set_register plus explicit caller completion to avoid an ignored transition. Keep response.read_value()? validation before effects and preserve HL/address capture/PC behavior. User chooses the exact private helper representation; assistant owns test maintenance and verification. Register ownership and fetch-state suppression remain separate checkpoints.

## Decoder-conversion review - 2026-11-19

User completed CB From/base TryFrom together before the CB-only checkpoint. IllegalOpcode has standard leaf reporting; Unknown and UnsupportedCbOpcode are removed, and the outer CB family fallback uses Set. Assistant migrated test decode calls (unwrapping only known legal cases), replaced the historical Unknown classifier with an independent exact eleven-byte illegal list, retained EI's valid/invalid classification and migrated the all-CB mapping test to From. All 17 CB-focused tests and six decoder tests pass; strict Clippy passes.

One behavioral regression remains: base decode failure returns Faulted before the existing opcode PC increment or HALT suppression consumption. The new cycle_protocol test demonstrates D3 fetched at 0000 leaves PC 0000 instead of 0001. Source inspection also shows an armed suppression flag is not cleared on that path. Preserve original address capture and fetch effects before handling the Result. The fault must still pass through CycleTransition::Faulted so the wrapper latches it; a fault must not mature delayed EI. The acceptance matrix checks all eleven illegal bytes, wrap addresses, IME modes, suppression, wrong-kind retries and sticky subsequent faults once the first failing assertion is resolved. No assistant application fix or full regression run; prior verified 439-test baseline remains valid only for 56f99bd.

## Cartridge error checkpoint verified - 2026-11-19

User implemented Display/Error for CartridgeError. Validation failures are source-less leaves with size/checksum/header-field metadata; FileIO adds path context and exposes the original io::Error without embedding its message twice. Assistant added two cartridge unit tests and a public integration test: standard Error/Box<dyn Error> use, validation metadata, a Unicode path, concrete I/O kind/cause, and actual loader missing-file propagation. Full verification passes 439 tests (314 unit, 125 integration), explicitly including five external fixtures, formatting, strict Clippy and whitespace checks. No assistant application changes, new dependencies or loader/execution/timing change.

This finishes standard error reporting for CPU/System/cartridge. The soon-to-be-removed UnsupportedCbOpcode is intentionally excluded from polishing. Next is [#55](https://github.com/Lorithan/gameboy-emu/issues/55): CbType/DecodedCbOpcode From conversions and dead-error removal only. User implements; assistant preserves the existing all-byte/result/flag/cycle oracles and runs regressions. Base illegal decoding, infallible helpers/register ownership, fetch-state suppression and larger state/module consolidation remain independent follow-ups. Four positional flag booleans remain rejected.

## System error checkpoint verified - 2026-11-19

User completed SystemError Display/Error and CycleAction Display. Request errors describe the next cycle query; completion errors describe the failed completion at tick_after with idle/read/write address/value context. CycleRecord's structured fields remain available; the message does not claim the already performed bus action itself failed. Both variants expose the original CpuError, preserving deeper typed fault/protocol causes. Three new tests/system_errors.rs tests check source chains and essential completion metadata without fixing every word of prose. The earlier tick_before formatting mistake is resolved.

Full verification passes 436 tests (312 unit, 124 integration) explicitly including five external fixtures; formatting, strict Clippy and whitespace checks pass. Assistant changed tests/docs only. Execution/timing and dependencies are unchanged. Next is CartridgeError's separate standard-trait cleanup: FileIO retains path context and its io::Error source; validation variants are leaves. #54 arithmetic extraction and other accepted structural proposals remain queued.

## CPU error checkpoint verified - 2026-11-19

User completed CPU error extraction and standard traits. Root cpu:: imports remain compatible; the new public cpu::error module contains CpuError, CpuFault, OpcodeEncoding and UnexpectedCycleResponse. Display keeps outer context separate from source detail, CycleKind has readable names, and fault bytes/addresses use fixed-width hexadecimal. Leaf Error sources are None; CpuError exposes its concrete wrapped cause. No execution/state behavior or dependencies changed by this cleanup.

Assistant's three tests/cpu_errors.rs tests check public API/standard Error use, byte/address/kind details and downcastable original causes; they avoid freezing whole message sentences. Full verification passes 433 tests (312 unit, 121 integration) explicitly including all five external fixtures; formatting, strict Clippy and whitespace checks pass. Earlier missing-trait/export findings below are resolved.

Next: user implements Display/Error for SystemError, retaining request versus completion distinction and the completion CycleRecord. Both variants expose their wrapped CpuError as source. CartridgeError is a later independent slice, preserving its io::Error source and path context. The accepted cleanup proposals also include infallible CB decoding, reviewed base decode faults, register ownership and a later fetch-state latch refactor; each needs its own checkpoint. Positional set_flags is discarded. #54's arithmetic-helper module extraction remains queued rather than completed by this error work.

Latest CPU error review, 2026-11-18: root exports and CpuError source chains are fixed. All three error acceptance tests and two public cycle-outcome tests pass, strict Clippy and whitespace checks pass. Message-polish guidance: keep outer context separate from source details to avoid repeated causes; existing expected/received wording and fixed-width fault hex are useful. No full regression rerun or assistant application change. The previous compile failures below are historical.

Current error-cleanup review, 2026-11-18: user accepted standard Display/Error work and discarded positional set_flags. User moved CPU error types to cpu/error.rs and added leaf formatting/Error implementations, but private parent imports removed the established public cpu:: paths and CpuError still lacks Error/source. Library check passes with a stale CpuError import warning; both the existing cycle_outcomes target and assistant's three new tests/cpu_errors.rs acceptance tests fail to compile on root visibility. No runtime result or full-suite pass is claimed. Restore explicit public root re-exports, add typed wrapped sources/leaf None and remove the unused cycles import before regression verification. If source() returns wrapped causes, prefer outer Display context without repeating the same cause text. Hex padding is an optional readability improvement. Application fixes remain user-owned; the module extraction proposal below is queued.

User focus, 2026-11-18: clean up CPU structure/readability before moving to M3. First implementation issue: [#54](https://github.com/Lorithan/gameboy-emu/issues/54). These are proposed boundaries; user owns application code and private naming choices, assistant owns review/test maintenance. No application refactor was performed for this specification.

Reviewed baseline: 1da8482, clean checkout. Last executable verification remains f0283fa: 430 passing tests including five external fixtures, formatting and strict Clippy passing. This source/docs review did not rerun emulator tests. M2 remains complete. Timer design #53 and optional DAA confidence work #52 stay queued while the user focuses on cleanup.

## Assessment

The existing state/transition design is suitable. complete_cycle validates response kind before mutation, applies explicit transitions, retains sticky faults and performs shared delayed-EI promotion. CPU owns its state; System supplies bus/device samples. Preserve those boundaries.

The readability problem is concentration: cpu.rs is about 1,600 lines and combines a roughly 575-line pending-state handler, 430-line opcode handler and arithmetic/bit helpers. These are different things to look up while debugging. Splitting coherent method groups makes navigation easier without changing the machine model. The exhaustive matches may remain long; file separation alone does not prove behavior is simpler or more correct.

## First slice: move arithmetic helpers

Add private mod alu in cpu.rs and another impl CPU block in cpu/alu.rs. This is the same CPU type with the same owned fields, not another component instance.

Move these existing methods unchanged:

- increment and decrement
- add, sub and execute_accumulator
- rotate, shift and execute_cb_transform

Current parent-module callers require pub(super) on increment, decrement, execute_accumulator, rotate and execute_cb_transform. add, sub and shift are only called within the moved group and can remain private. Keep CPU fields private. Use direct imports from super::opcode and super::registers, plus super::CPU; move std::ops::{Shl, Shr} with the methods. Parent-owned set_register remains callable from the child module.

Keep method bodies, call sites, inputs/results, flag writes and captured carry behavior unchanged. Leave inline DAA, signed-SP arithmetic, opcode dispatch and cycle transitions in their current locations for this first move. Consolidating additional arithmetic can be considered independently after this checkpoint.

## Later proposed slices

| File | Responsibility after separately reviewed moves |
| --- | --- |
| cpu.rs | CPU storage, public interface, construction/snapshot, shared completion transition/fault/EI handling and interrupt/wake entry points |
| cpu/alu.rs | Existing arithmetic/flag and bit-transform helper implementations |
| cpu/dispatch.rs | read_opcode: consume the opcode fetch, decode and select its initial instruction transition |
| cpu/execution.rs | complete_cycle_inner and closely related completion helpers: consume pending instruction/entry cycles |
| Existing cpu/opcode.rs | Byte-to-opcode decoding and operation types |
| Existing cpu/state.rs and cpu/registers.rs | Pending-state/transition payloads and register representation respectively |

These later file names/boundaries are proposals, not instructions to move everything at once. Keep the pure decoder separate from opcode execution. Review each move's cross-module calls and choose only the necessary CPU-private visibility. Once navigation improves, consider small family-specific helpers where repeated behavior actually obscures intent. Preserve wrapper-owned transitions and validate before effects. State-module imports can then be made direct rather than routed through parent use declarations as a separate small cleanup.

## Acceptance and verification

#54 should be a mechanical move/import/necessary visibility diff with no new public API or behavior. Existing tests exercise instruction interfaces, so no new test that merely mirrors file placement is needed. Assistant owns any minimal test-module migration and retains all result, flag, memory, trace, captured-input, retry, fault and budget assertions.

After the user's move, review the diff, run the existing full suite explicitly including the five verified external fixtures, formatting, strict Clippy and whitespace checks. Checkpoint/commit/push that slice before dependent dispatch/completion work. Do not claim the old 430-test result verifies an untested refactor. A compile/visibility problem should be resolved within CPU privacy, not by making helpers public to the whole crate.
