# Interrupt entry: first-stage M-cycle contract

GitHub task: [#39](https://github.com/Lorithan/gameboy-emu/issues/39), M2. Hardware-accuracy follow-up: [#40](https://github.com/Lorithan/gameboy-emu/issues/40).

Status: first-stage implementation verified on 2026-11-17 and ready for the requested commit. This document fixes the first-stage behavior. Names below are suggested; ownership, ordering, observable outcomes and invariants are the contract.

## Scope and baseline

User implements application changes; assistant owns tests, review and documentation. Baseline d6b0bb7 implements IF/IE with 339 passing tests (241 unit, 98 integration), formatting and strict Clippy from the preceding checkpoint. Those checks have not been rerun for this documentation-only task.

Implement ordinary running-CPU interrupt entry only. RETI, HALT/wake-up/HALT bug, STOP, peripherals, ROM fixtures, frontend and broad CPU refactors remain separate tasks. Do not claim M2 complete.

## Ownership and interfaces

- Bus keeps the private InterruptController and IF/IE storage. CPU owns IME and execution state. System owns scheduling, obtains eligible requests, and applies acknowledgment through Bus. CPU never borrows Bus.
- Before request_cycle on every actual System machine-cycle step, call a crate-visible mutating CPU acceptance operation, suggested try_accept_interrupt(pending: u8) -> Option<Interrupt>. CPU masks to the low five bits itself.
- Accept only in FetchOpcode with IME Enabled and at least one eligible bit. Disabled, Delayed, mid-instruction (including between CB prefix/subopcode), interrupt-entry and Faulted states return None without mutation. Calling on a fault must leave the later request_cycle error intact.
- Select the lowest set bit; capture selected source/vector, current PC and initial SP; set IME Disabled and enter the first entry stage. Acceptance changes no general registers, F, PC or SP.
- Reuse crate-visible memory::Interrupt as the small shared source identifier; this is no Bus borrow or memory-access interface. Keep bit/source/vector conversion centralized rather than duplicating matches in System. Pure source helper details belong to the user.
- Return Some(source) only for a newly accepted entry. System immediately acknowledges exactly that source using Bus, before requesting the first cycle. The return value makes this coordination explicit; no callback or new bus-access CycleAction is needed.
- request_cycle remains immutable and repeatable. complete_cycle drives entry stages exactly once per serviced cycle. Acceptance itself consumes no extra cycle or tick.
- Direct CPU callers use the same explicit acceptance operation and consume its result; System is the production coordinator. CPU-only tests need no Bus.
- Avoid exposing mutable CPU/Bus internals or public IME controls. Because Interrupt is crate-visible, the acceptance operation remains crate-visible too.

## Sampling and ordering

The order for one System step is: obtain current pending mask; offer acceptance; acknowledge an accepted source; query CPU action; advance four ticks; service that action once; complete CPU once; publish trace. Existing error/tick behavior is preserved after action query.

This is the project's coarse M-cycle convention, not the physical interrupt sampling edge. Future device events occurring during four-tick advancement become visible to the next acceptance opportunity, rather than retroactively changing an already chosen action. Base-tick peripheral timing needs its own contract.

At constructor startup, IME is Disabled. The first machine-cycle call still offers requests before its first fetch. At later boundaries, acceptance precedes the next opcode read; do not speculatively fetch or advance PC. A pending request that arrives during an instruction waits until its completion. Completion of the instruction returns before acceptance: acceptance happens on the next machine-cycle call. No extra time passes between these boundaries.

System::step_instruction(0) performs no acceptance, acknowledgment, memory access, CPU mutation or tick advancement. A positive budget may accept an interrupt even if insufficient to finish entry. Subsequent calls resume the captured entry; they do not acknowledge again.

## Source and vector table

| Source | Bit | Vector |
| --- | --- | --- |
| VBlank | 0 | 0040 |
| LCD STAT | 1 | 0048 |
| Timer | 2 | 0050 |
| Serial | 3 | 0058 |
| Joypad | 4 | 0060 |

## Five entry cycles

Let P be the captured next-instruction PC and S the initial SP. All subtraction wraps as u16.

| Cycle | request_cycle action | Successful completion |
| --- | --- | --- |
| 1 | Idle | Advance entry stage; PC=P, SP=S |
| 2 | Idle | Advance entry stage; PC=P, SP=S |
| 3 | Write high(P) to S-1 | SP=S-1; PC=P |
| 4 | Write low(P) to S-2 | SP=S-2; PC=P |
| 5 | Idle | Set PC to captured vector, return to FetchOpcode, report interrupt entry finished |

Five M-cycles = 20 ticks. Preserve A/B/C/D/E/H/L and all F bits throughout. Stack writes follow ordinary Bus rules, including echo, register mapping and ignored ROM/unmapped writes; SP decrements even when a write is ignored. No extra opcode read or handler execution is part of entry. The next cycle fetches the vector opcode. Use explicit entry continuations or safely shared captured stack continuations; do not disguise entry as a decoded CALL/RST instruction.

## Completion and bounded stepping

User-approved refinement during implementation: private CycleTransition::{Continue(ExecutionState), InstructionFinished, InterruptEntryFinished { interrupt }, Faulted(CpuFault)} replaces handler-owned execution-state assignments. Only the public completion wrapper applies the next state; successful finish transitions select FetchOpcode centrally. MissingReadValue returns an error without a transition; a newly discovered unsupported opcode returns Faulted, which stores the sticky state before reporting the error. An already faulted CPU retains its state and error. Register changes remain handler-owned and must follow read validation.

The implemented trace interface also refines the original proposal: CycleRecord holds tick_before/tick_after/action/read_value; a successful CycleTrace contains record plus a plain CycleOutcome. CpuCompletionError carries the CpuError and failed CycleRecord. step_machine_cycle returns the outer Result, so step_instruction receives only successful traces after ?. No nested Result or unreachable error arm is needed in its outcome match. InstructionStep::InterruptEntered denotes completed entry, not initial acceptance.

Replace the ambiguous CPU completion bool with a small explicit public cycle outcome, suggested CycleOutcome::{InProgress, InstructionComplete, InterruptEntryComplete { vector: u16 }}. Vector is a public numeric address, avoiding a public API leak of crate-visible Interrupt. Existing instructions retain all cycle counts and return InstructionComplete only at their existing completion points.

The complete_cycle wrapper advances delayed EI only for InstructionComplete. Entry never advances instruction bookkeeping. No change to the semantics of EI;NOP, EI;DI, EI;EI or multi-cycle/CB followers.

CycleTrace carries the outcome instead of the bool. Migrate its consumers/tests directly; do not leave a second independently maintained completion bool. InterruptEntryComplete is emitted only on cycle 5 and includes the captured destination.

Preserve InstructionStep::Complete { mcycles, ticks } for ordinary instructions and BudgetExhausted for incomplete work. Add a distinct InterruptEntered { vector, mcycles, ticks } result, returned as soon as cycle 5 finishes. This keeps the existing method name but explicitly broadens its documented stop events. Counts describe only work done by this invocation: resuming after two entry cycles reports three M-cycles/12 ticks, not five/20. Excess budget never runs the first handler instruction in the same call. No result reports entry acceptance as entry completion.

## Deliberate approximation and required follow-up

This first stage freezes source/vector and clears the selected IF bit at acceptance. This makes entry deterministic and acknowledgment exactly once across pause/resume, but does NOT reproduce hardware's late source selection/cancellation.

Mooneye interrupts/ie_push.s is verified on DMG and other models. It shows an upper return-byte stack write to IE can cancel dispatch (PC=0000, request retained) or switch the chosen source. A lower-byte write to IE is too late to cancel. Freezing at acceptance cannot reproduce those cases. Do not add special-case stack addresses or label this implementation fully interrupt-accurate.

For this stage:
- Changes to IF/IE after acceptance do not redirect/cancel entry.
- A new request for the accepted source after immediate acknowledgment remains pending unless a subsequent software/stack write changes IF.
- New higher-priority requests during entry wait; IME is already Disabled.
- Stack writes to IF/IE still change the registers normally but not the captured vector.
Tests for these are project-model tests and must evolve with the follow-up; they are not hardware expectations.
The late-selection/cancellation follow-up must be completed before claiming the affected hardware behavior or passing ie_push. Same-source request/acknowledgment collisions and exact physical sampling edges remain unverified and require additional research when that follow-up/devices are implemented.

## Acceptance criteria (assistant owns tests)

- Every low-five pending mask, upper-bit noise, all five priorities/vectors, all IME states and boundary/nonboundary/fault cases. Rejected acceptance is mutation-free; accepted source returned once; only its IF bit cleared.
- Exact five-action/tick trace, intermediate PC/SP, all flags/general registers preserved; no opcode read, early PC jump or early stack mutation. Exhaustive return-PC values and a separate exhaustive SP sweep; representative mapped/unmapped/ROM/HRAM/echo/register boundaries and wrapping.
- Repeated immutable action queries at every entry phase; captures persist under test-only register mutation; normal missing-read retry regressions remain intact. Entry has no read action and needs no invented read failure.
- All zero/partial/exact/excess budgets and pauses after every entry cycle. One acknowledgment total, exactly one high and one low write, distinct final event, correct per-call counts, next handler opcode continuation. Full-address-space permitted-change oracle includes accepted IF clear and actual stack destinations/aliases.
- Pending requests arriving during multi-cycle and CB instructions wait for the next boundary. EI;NOP, EI;DI, EI;EI and delayed enable across instruction retries/faults retain their existing behavior. No interrupt acceptance before a following instruction completes.
- First boundary and direct machine-cycle stepping agree with bounded stepping. Existing sticky faults remain sticky even when requests are pending.
- Selected-source re-request, later higher-priority source, IF/IE changes and stack-register writes demonstrate the explicitly labeled captured-source approximation; never claim ie_push conformance.
- Replace the temporary no-dispatch test from #38 with entry checks. Migrate all bool-outcome consumers and retain the existing instruction regression suite. Remove dead-code allowances only where production callers now exist; peripheral-only request hooks may still need theirs.
- Full cargo test --all-targets --quiet, cargo fmt --check, cargo clippy --all-targets -- -D warnings, git diff --check. No dependencies or ROM fixture additions.

## Implementation slices

1. Change completion/trace/step outcomes and migrate consumers without changing instruction behavior. Assistant verifies the existing suite before entry logic.
2. Add CPU acceptance and captured five-cycle entry; assistant verifies CPU cases.
3. Wire System pending/acknowledgment and acceptance ordering; assistant verifies integration and budgets.
These are review checkpoints within one focused task, not permission to refactor unrelated instruction transitions.

## References checked 2026-11-17

- [Pan Docs interrupts](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupts.md): normal priority, vectors, IME/EI and five-M-cycle entry overview.
- [Mooneye ie_push source](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/interrupts/ie_push.s): primary hardware-verified evidence of late IE effects; reference only, not imported or run.
- [SonoSooS CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595): fetch/execute overlap research linked by Pan Docs. Its microarchitecture is not equivalent to our coarse action trace.

No emulator implementation or automated tests changed while specifying this contract.

## Transition/outcome slice verification - 2026-11-17

The user implemented the private state-transition and public outcome/trace refactor. Assistant migrated the regression tests and added two public API tests. Full suite: 341 tests pass (241 unit, 100 integration). Formatting and strict Clippy pass after user-approved temporary dead-code allowance on InterruptEntryFinished and removal of a redundant return in the fault arm. The two direct public API tests were rerun after those cleanups and pass. Whitespace checks pass. No external ROM suite ran. Changes are uncommitted.

All execution-state assignments in cycle completion now occur in the wrapper. Existing instruction timing, results, captured operands, retryability, sticky faults and delayed-EI regression checks remain green. The shared test-only completion adapter rejects unexpected entry; it does not add production compatibility operators. Failed-cycle tests now inspect CycleRecord, which intentionally has no success outcome. This validates the refactor slice, not interrupt-entry behavior. Next user implementation: explicit CPU acceptance and captured entry states, then System pending/acknowledgment wiring. #39 remains open.

## CPU entry slice - 2026-11-17

User implemented try_accept_interrupt and five captured CPU entry states. Assistant added three CPU tests: all 256 pending masks across Disabled/Delayed/Enabled IME, rejection during representative immediate/CB/CALL/PUSH/fault states, exact repeated action queries and per-cycle snapshots/outcomes, all 65,536 return PCs and a separate all-65,536 SP sweep, each vector and representative stack boundaries, next handler-opcode continuation, and captured PC/SP under test-only mutation. Focused tests pass; all 244 library tests pass. Test source formatting and diff whitespace checks pass. No production behavior was changed by the assistant.

CPU progression matches the first-stage contract by inspection and tests. This does not validate System acknowledgment, budgets, elapsed entry ticks or real Bus write effects: System acceptance wiring is not implemented yet. Full all-target tests and strict Clippy were not run for this intermediate slice. cargo check --lib passes with expected unused production acceptance/entry warnings until System wiring arrives. No ROM suite ran, and no commit was made. Next: wire System acceptance/one-time acknowledgment before request_cycle, preserving zero-budget behavior and the explicit entry-completion result.

## System integration evidence - 2026-11-17

User wired pending-mask sampling and newly accepted source acknowledgment before request_cycle. Assistant added six System tests and replaced the temporary #38 no-dispatch assertion. Coverage includes all 1,024 low-bit IF/IE pairs with upper IE bits set; all five vectors; zero/partial/exact/excess entry budgets; exact five-cycle actions/ticks/outcomes and per-call resume counts; next handler-opcode continuation; and full 65,536-address expected-memory comparisons at every pause.

Stack targets cover ROM/unmapped memory, WRAM and echo, HRAM, wrapping, IF and IE writes. Expected effects include the one-time IF acknowledgment and each completed stack byte. Re-requesting the selected source and adding a higher-priority request after acceptance proves acknowledgment is not repeated and the vector remains captured. Tests explicitly record this first-stage approximation rather than claiming hardware ie_push behavior. Delayed-EI followers include immediate, CB register/BIT/modify-memory and EI; DI cancellation and requests arriving mid-instruction while IME is already enabled are covered. Sticky faults and zero-budget calls preserve pending requests and state.

CPU acceptance/progression coverage also checks all sixteen flag combinations for every source, all pending byte masks/IME states, all return PCs and a separate exhaustive SP sweep. No external ROM binaries were introduced or run. #40 remains required for hardware late selection/cancellation. No interrupt-entry commit has been made yet.

Validation: all 349 tests pass (249 unit, 100 integration). The nine focused CPU/System entry tests also pass after adding all-flag coverage. Formatting, strict Clippy and diff whitespace checks pass. No correctness defect found within the first-stage contract. Remove now-obsolete temporary dead-code allowances on APIs/variants with production callers during checkpoint cleanup; the peripheral-only request hook can retain its allowance. #39 remains open pending the reviewed checkpoint; #40 is unchanged.

## Checkpoint cleanup - 2026-11-17

The user removed obsolete dead-code allowances; only the two peripheral request_interrupt hooks retain them. Fresh formatting, strict Clippy and whitespace checks pass after this attribute-only cleanup. Behavioral evidence remains the 349-test full run plus the nine focused entry tests after all-flag coverage. The user requested this checkpoint be committed. #39 is complete within its explicitly limited first-stage model; #40 still tracks hardware late-selection/cancellation behavior. Earlier unimplemented/pending-cleanup notes are historical.
