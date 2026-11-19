# Core architecture

Implemented contract, 2026-11-19. [Plan](PROJECT_PLAN.md) · [CPU](CPU.md) · [devices/startup](DEVICES.md).

## Ownership

The frontend will own System. System owns CPU and Bus; CPU owns registers, execution state and IME; Bus owns Cartridge, WRAM/HRAM, interrupt controller, Timer and Joypad. CPU never borrows Bus: System performs requested accesses and returns typed responses. Use ordinary ownership, not shared Rc/RefCell storage. There is no shared Memory trait or separate ALU object. Core library execution is independent of SDL2 and host pacing.

## Machine-cycle schedule

One DMG M-cycle is four base ticks. System::step_machine_cycle currently:

1. Offers eligible pending interrupts for HALT wake and interrupt acceptance.
2. Requests a CPU action. Stopped returns without advancing time/devices; a request fault also consumes no cycle.
3. Adds four to monotonic elapsed_ticks and advances Bus devices four ticks.
4. Offers fresh eligible interrupts before bus execution. If CPU accepts, refreshes the action from InterruptEntry without another device/time advance. Performs that read/write/idle and collects response samples; a replaced opcode fetch is represented by entry's first idle.
5. Completes CPU execution, records the trace and applies acknowledgment/STOP-entry effects.

This is a coarse advance-before-access convention, not arbitrary subcycle/pin accuracy. [Timer reload/write ordering](tasks/TIMER.md) must explicitly fit or deliberately revise it.

## Typed protocol and policy

CycleRequest is Execute(CycleAction) or Stopped; actions are Read, Write or Idle. Idle is a real cycle. Stopped polling generates no trace and cannot be completed with a fake idle response.

CycleResponse is Read with value, post-read eligible interrupt mask and selected JOYP wake level; WriteDone with IF-before-write and IE-after-write samples; or IdleDone. Public complete_cycle validates kind before effects. Wrong kinds preserve CPU state for retry; completing while stopped reports NoCycleRequested; sticky faults preserve metadata.

Private handlers return CycleTransition. The public wrapper applies state/completion/fault changes and shared delayed-EI policy. CycleOutcome distinguishes progress, instruction completion, interrupt selection/cancellation, entry completion/cancellation and actual StopEntered. System acknowledges only InterruptSelected and resets DIV only on StopEntered. [CPU details](CPU.md).

## Stepping and diagnostics

Public System APIs provide single-cycle stepping, bounded instruction stepping, owned snapshots and synchronous button updates. Instruction stepping distinguishes instruction completion, interrupt entry, stopped execution and exhausted budget. Zero budgets are pure; budget/timeout exhaustion is never success.

CycleRecord stores before/after ticks, action and optional read value; successful CycleTrace adds outcome. SystemError separates request errors from completion errors retaining their already-consumed cycle. CPU/System/cartridge Display/Error implementations expose typed causes; leaves preserve opcode/address, protocol kinds, paths or expected/actual data without duplicated outer cause text.

## CPU files

| File | Responsibility |
| --- | --- |
| src/cpu.rs | Storage, construction/snapshot, requests, public validation/transition/fault/EI policy, acceptance/wake guards |
| cpu/opcode.rs | Pure decoding and operand/operation types |
| cpu/dispatch.rs | Fetch effects and first-cycle execution/initial transition |
| cpu/execution.rs | Pending-cycle completion and storage/word-finish helpers |
| cpu/alu.rs | Arithmetic/flags/bit helpers on the same CPU |
| cpu/state.rs | Private states, typed continuations and transitions |
| cpu/registers.rs | Register storage/selectors/pairs and flag invariants |
| cpu/cycles.rs, cpu/error.rs | Public cycle protocol and diagnostics |

Use only necessary parent/sibling visibility; CPU fields stay private to its tree. Bus-owned devices retain their own state. Do not introduce speculative abstractions for future consoles.
