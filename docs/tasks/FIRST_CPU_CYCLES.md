# First CPU execution slice

## Goal
Implement the first machine-cycle-stepped CPU slice: **NOP (0x00)** and **LD A, immediate (0x3E)**. Milestone: M2. Builds on the completed register, startup, snapshot, and memory work.

## Accuracy boundary and ordering
Use a deterministic initial **M-cycle model**, not a pin/edge-accurate SM83 simulation. One CPU cycle spans four base ticks. System owns the clock; elapsed_ticks starts at zero at cartridge entry (it is not time since physical power-on).

For every successful cycle:
1. Obtain one CPU action without advancing CPU state: Read(address), Write(address, value), or Idle.
2. Advance four base ticks through one centralized clock path.
3. Service the bus action once (Idle does no access).
4. Complete that CPU transition once, supplying a read byte only for Read.
5. Return a trace, including whether an instruction completed.

This bus-at-end rule is a **project convention for the first slice**, not a verified hardware sampling edge. Physical fetch/execute overlap and sub-cycle peripheral effects are not modeled yet. Keep phase handling centralized so device timing can refine it later. This deliberately replaces the earlier requirement to fully settle physical bus-edge ordering before any CPU instruction can be implemented.

## Interface contract
Names below are suggested; preserve the behavior:
- Internal CPU cycle-action query and cycle-completion methods. A query is repeatable and does not increment PC or modify registers. No CPU ownership of Bus and no shared ownership.
- Public System::step_machine_cycle returns a Result with one cycle trace on success.
- Public System::elapsed_ticks returns u64.
- A trace exposes tick count before/after, the read/write/idle action, read data when applicable, and an instruction_complete flag. It is an owned diagnostic value.
- Public System::step_instruction(max_mcycles) repeatedly uses the same cycle path, consuming at least one cycle and stopping at the next completed instruction. Its result reports cycles/ticks consumed, or budget exhaustion. Budget zero changes nothing; a partial instruction remains resumable.
- Direct Bus reads/writes and snapshot inspection never advance the clock.
- No device tick stubs or empty generic event framework are required now: clock advancement initially updates the counter; add device advancement there when devices are implemented.

CPU needs an explicit initial fetch state, in-progress instruction state, and a faulted state. Do not execute a multi-cycle instruction atomically or silently prefetch the following instruction.

## Supported behavior
- NOP: fetch at PC, advance PC by one with u16 wrapping, complete after one cycle (four ticks); preserve all other registers/flags.
- LD A, immediate: opcode fetch advances PC but leaves A/flags unchanged and reports incomplete; next cycle reads the immediate, advances PC, sets A, preserves flags/other registers, and reports complete. Total two cycles (eight ticks).
- Treat every other opcode as **not implemented**, not as a claim that it is illegal hardware. Return a recoverable diagnostic containing opcode and fetch address, plus the consumed-cycle trace. The unsupported fetch consumes four ticks and advances PC once; record a terminal fault so later steps return that fault without consuming additional cycles. Reconstruct System to reset it for now.
- Interrupts, HALT/STOP, CB instructions, DMA, devices, and additional opcodes are outside this task.

## Test acceptance (assistant owns test code)
- [ ] Existing 45 tests still pass.
- [ ] Synthetic program at 0100: 00 3E 42 00. Cycle-end PC values: 0101, 0102, 0103, 0104; elapsed ticks: 4, 8, 12, 16; completion flags: true, false, true, true. A changes to 42 only on cycle three; F/BC/DE/HL/SP stay unchanged.
- [ ] Immediate values 00, 80, FF load unchanged and do not alter flags.
- [ ] Repeated action queries do not mutate CPU; read completion advances exactly one state.
- [ ] CPU unit tests check PC wrapping without requiring executable memory at FFFF in the current Bus.
- [ ] Traces report exact addresses, data, and tick spans.
- [ ] Zero and insufficient budgets terminate without hanging; partial LD resumes successfully without refetching its opcode.
- [ ] Unsupported opcode diagnostics and sticky-fault semantics match the contract.
- [ ] Formatting passes; report strict Clippy results and any remaining unused planned instruction helpers without widening visibility solely for lint cleanup.

## References and interpretation
[Gekkio, GB Complete Technical Reference, revision 192](https://gekkio.fi/files/gb-docs/gbctr.pdf), chapters 2, 5, and NOP/LD immediate entries; [gbdev opcode data](https://github.com/gbdev/gb-opcodes/blob/master/Opcodes.json); [SM83 signal research](https://iceboy.a-singer.de/doc/dmg_cpu_connections.html). Consulted 2026-11-02.

The references establish cycle counts, access sequences, and the limits of a simple model. The bus-at-end convention, trace schema, fault handling, and budget API are project design choices. Test success on this task demonstrates that model's behavior, not full hardware timing accuracy.

The user writes emulator implementation. The assistant maintains tests and architecture. Begin with NOP and one-cycle stepping; add LD A, immediate and bounded instruction stepping next.

## First cycle-request review

ExecutionState and CycleAction are in place. Three new CPU unit tests currently fail: the first request reads 0101 instead of 0100, querying changes PC, and querying at FFFF panics on overflow. Move PC advancement to cycle completion and make request_cycle an immutable query. Formatting passes. These are targeted request tests; the full suite was not rerun during this review. Next implement NOP completion and recoverable unsupported-opcode handling before System stepping.

## Cycle-completion review

The immutable Result-returning request API now passes its tests. Updated those tests and added NOP/LD completion, unsupported opcode, sticky fault, missing read data, and PC boundary coverage. Targeted CPU run: five passed, seven failed (12 total); formatting passes. No full-suite rerun during this review.

Outstanding implementation bugs: opcode 3E reports completion too early; operand completion does not increment PC or restore FetchOpcode; opcode PC increments can overflow; unsupported opcodes neither consume the byte nor set Faulted. Missing read data returns a fabricated CpuFault for opcode zero, so the diagnostic category still needs separation from unsupported opcodes. A recoverable internal protocol error must leave state unchanged. Emulator code was left unchanged; failures are preserved for the user to address.

## Updated cycle-completion review

CpuError now separates unsupported opcodes and missing read values. NOP/LD transitions, supported PC wrapping, and sticky fault behavior pass. Tests were migrated to the error enum and extended for missing-operand retry and unsupported fetch at FFFF. Latest targeted CPU run: 12 passed, two failed. Both failures are the same remaining contract gap: unsupported opcode fetch does not advance PC once, including wrapping at FFFF. Save the original fault address and consume the byte before returning the stored fault. Formatting passes. Full suite not rerun during this review; emulator code remains unchanged.

## CPU cycle transitions verified

The user fixed unsupported-fetch PC consumption and removed unused imports. Full suite: 59 tests pass (55 unit, four integration), including all 14 CPU cycle tests. Formatting passes. Strict Clippy is still blocked by unread CpuFault fields, unused future register variants/pair setters, and System memory. CPU request/completion behavior is ready for System integration. Clock advancement, cycle traces, and bounded instruction stepping are not yet implemented, so issue #5 remains open.

## Initial System stepping review

System now starts elapsed_ticks at zero, requests the CPU action before advancing four ticks, services reads/writes through Bus, and completes supported CPU transitions. Migrated tests to CycleAction's renamed addr field. Added five tests for initial/inspection clock behavior, supported immediate values, independent systems, exact mixed-program cycle progression, and fault timing.

All 64 tests (58 unit, six integration) and formatting pass. Strict Clippy remains blocked by unread CpuFault fields, unused register variants, and pair setters. Unsupported fetch consumes four ticks; subsequent faulted steps consume none, as verified through the public API.

The dormant Idle branch incorrectly returns true without calling CPU completion; it must complete an idle transition with None rather than assume an instruction finished. Current CPU states never request Idle, so that path and write cycles are not exercised end-to-end yet. The public step still returns only bool; required owned cycle traces, error traces, and bounded instruction stepping remain pending. No emulator logic changed by the assistant.

## Initial trace review

Idle now calls CPU completion and System returns CycleTrace. Tests were adapted to the new return type. Two new trace tests expose reuse across an instruction: an immediate operand trace keeps the opcode address and accumulates eight ticks instead of describing its own four-tick cycle. Library tests: 58 passed, two failed. Public API tests: six passed separately. Formatting passes.

Recommendation: construct a fresh local trace for every cycle, with absolute ticks_before/ticks_after, current action, current read value, and current completion status. Do not retain a dummy previous trace in System or reset it only on instruction completion. Current trace fields have no public accessors, so public completion assertions are temporarily covered by internal unit tests; restore them to public tests when the interface is exposed. Completion errors still discard consumed-cycle trace data through early propagation; attach the trace to a System execution error. Bounded instruction stepping remains pending.

## Revised trace/error review

The user introduced public fresh per-cycle traces and SystemError variants separating request failures from completion failures with a consumed-cycle trace. Tests now check absolute spans and public trace contents, including unsupported-read error traces.

Current run: 56 unit tests pass and five fail; four public API tests pass and two fail. The read branch calls complete_cycle(None) despite having read a value, causing MissingReadValue on every fetch. It also assigns trace.read_value after completion, losing the byte in an error trace. Record Some(value) in the trace before calling completion and pass Some(value) to CPU. The trace/error architecture is otherwise consistent with the contract. Formatting passes. No emulator logic changed by the assistant.

## Single-cycle stepping verified

The user fixed read-value propagation and trace ordering. Full suite: 67 tests pass (61 unit, six integration); formatting passes. Read traces now retain the actual byte even when CPU completion faults, with correct absolute tick spans. Existing sticky faults consume no extra time. Strict Clippy still reports private unread fault fields and unused future register variants/pair setters.

Single-cycle NOP/LD execution and cycle/error traces are locally verified. Bounded step_instruction(max_mcycles) is the remaining behavior in issue #5. Idle/write paths remain structurally reviewed, not exercised end-to-end by the current read-only instruction slice. The model remains the documented M-cycle approximation.

## Bounded stepping completion

The user implemented InstructionStep::{Complete,BudgetExhausted} and step_instruction, with a corrected one-based completed-cycle count. Added six public integration tests covering zero budgets on fresh/partial states, exact and excessive budgets, no fetch beyond completion, load resumption, per-call counts after earlier execution, and completion/request fault behavior including consumed-cycle traces.

All 73 tests pass (61 unit, 12 integration), and formatting passes. Strict Clippy still reports unread fault fields, unused register variants/pair setters, and a needless final return in step_instruction. These lint failures are explicitly retained; no visibility changes or warning suppression were made by the assistant.

Issue #5's supported behavior and acceptance tests are complete locally. The new wrapper/tests need committing and publication. Device-edge accuracy and end-to-end Idle/write instructions remain outside this slice. The assistant changed only tests/documentation; emulator logic is the user's work.

## Bounded stepping checkpoint

The user removed the needless final return. The checkpoint contains bounded instruction stepping, its six integration tests, and completion documentation. The previous 73-test verification applies; formatting and staged whitespace checks are checked before commit. Remaining strict Clippy failures concern existing unread fault fields and unused register scaffolding. Publication is pending.
