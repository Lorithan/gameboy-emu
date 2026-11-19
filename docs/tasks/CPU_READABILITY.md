# CPU structure and readability cleanup

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
