# STOP: core standby and device prerequisites

Core task: [#43](https://github.com/Lorithan/gameboy-emu/issues/43), M2. Device follow-up: [#44](https://github.com/Lorithan/gameboy-emu/issues/44), dependent on M3 divider/timer and M4 joypad work.

Status: core standby implemented and verified under #43, followed by divider reset/pause under #46 on 2026-11-18. Latest suite: all 390 tests pass. Joypad wake and finer STOP behavior remain #44. User owns application implementation; assistant owns tests, documentation and review. Historical proposal/review notes below are superseded by completion evidence.

Baseline: published b9c765e, 371 previously passing tests (271 unit, 100 integration), formatting and strict Clippy passing. STOP is currently unsupported; coverage remains 243/244 legal non-prefix base plus all 256 CB encodings. No emulator tests were rerun for this documentation-only proposal.

## Scope and evidence

STOP is not another HALT. Pan Docs describes wake through selected P10-P13 joypad lines and conditional STOP/HALT/NOP/one-byte behavior. RGBDS documents the usual two-byte encoding, default padding 00, unchanged flags, and deliberately gives no fixed cycle count. Neither justifies a universal two-byte/two-M-cycle implementation.

The CPU-internals reference describes clock suspension and fetch overlap, with different outcomes depending on WAKE and pending interrupts. Our current CPU model has no hardware prefetch pipeline or WAKE input. Translating ordinary standby into one opcode-read M-cycle and a skipped padding address is a **project approximation**, not evidence of exact STOP bus timing.

Pan Docs also requires DIV reset on STOP and suspension until STOP ends. DIV and JOYP do not exist yet. Implement the core scheduling distinction now; integrate those device effects in #44. Keep the limits visible when reporting opcode coverage.

## Proposed ownership and interfaces

Keep CycleAction::{Read, Write, Idle} as actual clocked bus/internal actions. Idle still means one M-cycle; it must retain HALT's four-tick behavior.

| Owner/location | Proposed type/change | Meaning |
| --- | --- | --- |
| CPU, cpu/cycles.rs | CycleRequest::{Execute(CycleAction), Stopped} | A real next cycle or no cycle while the clock is stopped |
| CPU request_cycle | Result<CycleRequest, CpuError> | Immutable, repeatable query; faults retain existing error behavior |
| CPU private state module | ExecutionState::Stopped, CycleTransition::StopFinished | Persistent standby and the one-time instruction transition into it |
| CPU completion protocol | CycleOutcome::StopEntered | This instruction completed and entered standby; one event, rather than an outcome plus an independent boolean |
| System, system.rs | MachineStep::{Cycle(CycleTrace), Stopped} | A real trace or a normal no-cycle result |
| System step_machine_cycle | Result<MachineStep, SystemError> | Can report a stopped clock without manufacturing a CycleRecord |
| System bounded wrapper | InstructionStep::Stopped { mcycles, ticks } | Stop running and report actual work consumed by this invocation |

CPU owns execution state and IME. System owns scheduling and the accumulated elapsed_ticks counter. Bus will own divider/joypad state. Do not add a second stopped boolean to System or let CPU access Bus. No new CycleResponse variant is needed because there is no cycle to complete while stopped.

Propose CpuError::NoCycleRequested for a caller that invokes complete_cycle while request_cycle reports Stopped. This is protocol misuse, not a normal step result. Reject every response in that state before mutation; never accept IdleDone as a polling operation. Preserve sticky fault precedence and existing wrong-kind validation for real cycles. The completion wrapper must gate the inner completion handler on a real request.

## Proposed ordinary standby behavior

For the supported ordinary case, an unsuppressed opcode fetch at P reads 10, advances the usual opcode PC, then skips the padding address with wrapping arithmetic. The resulting PC is P+2 modulo 65536. Preserve general registers, F and SP. There is no added padding read or extra Idle action in this approximation.

The validated opcode completion yields StopFinished. The shared wrapper installs Stopped and emits StopEntered exactly once. Include StopEntered in shared instruction-boundary delayed-EI promotion: EI;STOP matures EI at STOP completion; polling never promotes it again. STOP does not independently enable or disable IME.

Existing HALT suppression still belongs to the successful opcode-fetch path. Do not add a second suppression mechanism or recompute PC from P in a way that bypasses the existing latch. Its composition with STOP can be tested for internal consistency, but hardware HALT/STOP interaction remains unverified and belongs to #44.

In this first approximation, every decoded 10 takes the same standby path. CPU does not read or validate the padding value. Documented ordinary examples use 10 00; nonzero-padding and pending-at-entry/held-input hardware branches are not supported by this model. This deterministic fallback must not be described as faithful behavior for those cases.

Already stopped:

- request_cycle returns Stopped without mutation.
- System returns MachineStep::Stopped before tick advancement, bus access or CPU completion. No CycleRecord exists for that call.
- Existing HALT wake and interrupt-acceptance guards must leave Stopped alone. Changing IE/IF, including a joypad interrupt flag, does not itself resume this model or acknowledge a source.
- No public force-resume method or synthetic joypad device in #43. Until #44, the core can report standby but cannot resume from it through emulated input.

## Bounded stepping contract

| Situation | step_instruction result | Work counted |
| --- | --- | --- |
| max_mcycles = 0, any state | BudgetExhausted | 0 cycles, 0 ticks, no mutation |
| Ordinary STOP entered by this call | Stopped | Actual cycles through StopEntered; normally 1 cycle, 4 ticks |
| Already stopped, positive budget | Stopped | 0 cycles, 0 ticks |
| HALT waiting, no eligible interrupt | BudgetExhausted | Existing clocked idles, 4 ticks each |

On MachineStep::Cycle, use the existing event handling plus StopEntered. On MachineStep::Stopped, return immediately without counting an iteration as a cycle. A budget limits real cycles, not polling attempts. Never loop waiting for host input inside the headless core; a future frontend can keep processing host events while emulated time is suspended.

## Alternatives and reasons

- **Treat STOP as HALT:** small change, but advances clocks and applies the wrong wake condition. Reject for the standby contract.
- **Use Option for requests and machine steps:** valid and smaller; None can represent no cycle. Named enum variants communicate why there is no cycle and support explicit matching. Recommend the named variants, without adding hypothetical pause reasons.
- **Add Stopped to CpuError for normal requests:** avoids a successful-result wrapper, but makes ordinary guest standby a failed execution step. Reserve errors for actual faults/protocol misuse.
- **Check is_stopped in System and leave request_cycle unchanged:** requires a second convention for direct CPU callers and still needs a defined request result in Stopped. Prefer one total CPU request protocol.
- **Defer all STOP work until devices exist:** valid if device-complete behavior is preferred. The core slice is useful now because it settles no-cycle stepping independently; #44 remains required for real wake/divider behavior.

## Implementation slices and acceptance

1. **Request/step protocol migration, preserving current execution.** Add the request and machine-step result types, wrap all real cycles, update complete_cycle validation and bounded stepping. Assistant migrates tests while preserving independent oracles. All 371 existing tests must still pass; no STOP decode yet.
2. **Standby entry.** Add STOP decode, private state/transition, StopEntered and shared EI bookkeeping. Add bounded stopped results and reject completion without a cycle. Assistant replaces STOP-as-unsupported expectations with a genuinely invalid opcode and retains sticky fault checks.
3. **Verification and checkpoint.** Run focused tests, full suite, formatting, strict Clippy and whitespace checks. Record the implemented approximation, not just successful compilation.

Acceptance cases for the assistant:

- Independent 10 decode classification; preserve all invalid slots and every other opcode's meaning.
- All initial flags and IME states, PC wrapping at FFFE/FFFF, ordinary P+2 advancement, unchanged registers/SP and memory. No padding bus read, write or extra idle in the declared model.
- Repeated request queries and wrong-kind opcode responses preserve PC/state/IME/latches. Every response while stopped fails atomically with NoCycleRequested; sticky faults still report their original error.
- One StopEntered event/real trace on entry; repeated stopped machine steps have no trace, tick movement, memory change, EI promotion or acknowledgment.
- Zero, one and excess instruction budgets; already-stopped positive budgets return immediately. Contrast HALT's actual idle traces and tick consumption.
- EI;STOP, DI;STOP and pending requests after entry; enabled IME cannot bypass Stopped. Existing pre-fetch interrupt acceptance still wins before fetching a prospective STOP when eligible.
- Preserve HALT suppression consumption in the shared fetch path, while labeling HALT/STOP composition as model consistency, not hardware conformance.
- Existing instruction/interrupt entry, cancellation, RETI, cycle protocol, trace, wrapping and full-memory regression checks retain their oracles.

Closing #43 means core standby is implemented and verified. It does not close #44, M2's ROM-validation gate, or establish complete CPU correctness. If the decode count reaches 500/500, qualify it with STOP's device and edge-behavior limitations.

## Device follow-up (#44)

Specify the one-shot divider-reset effect at StopEntered without resetting elapsed_ticks. Connect selected JOYP lines to a CPU wake operation through System; do not reuse HALT's IE&IF wake test. Define clock domains, host-input delivery while stopped, held inputs, pending-at-entry branches, resumed PC, wake timing and IME/IF behavior before implementing them. Revisit prefetch/bus observability and HALT/STOP interactions against evidence. Distinguish deterministic supported behavior from revision-specific oscillator/glitch behavior. CGB speed switching remains out of scope.

External validation needs versioned test selection, DMG target, source/license, success protocol, prerequisites and bounded execution before importing any fixtures. No ROM source/binary has been copied or run for this task.

## Sources consulted 2026-11-18

- [Pan Docs power consumption / STOP](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Reducing_Power_Consumption.md): normal encoding, selected joypad wake and conditional behavior; its linked diagram needs review before device-complete branch implementation.
- [RGBDS gbz80 STOP](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7): two-byte encoding, default 00 padding, unaffected flags and deliberately unspecified fixed cycles.
- [SonoSooS CPU internals, NOP and STOP](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595): fetch overlap, clock suspension, WAKE/pending cases and wake glitch discussion. Used as research context, not a substitute for hardware validation.
- [Pan Docs timer/divider registers](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md): DIV reset and pause during STOP.

## Device prerequisite roadmap - 2026-11-18

User clarified that #44 is the current focus; external ROM runner proposal #45 is retained for later. This is a proposed work sequence, not a finalized device implementation contract.

1. **Divider and device time.** Bus owns the counter, eventually shared with the programmable timer. Propose a wrapping u16 represented in base ticks, with FF04 exposing its upper byte. System advances owned devices for each real M-cycle using the existing four-tick convention before its bus action; stopped polls advance nothing. Writes to FF04 reset the entire counter. Specify post-boot counter state, phase/ordering and future TIMA edge integration before implementation; do not invent a hardware startup value or derive the resettable divider from accumulated elapsed_ticks.
2. **Headless joypad.** Bus owns eight button states and FF00 row selection. Define active-low selected input lines, both/neither row selection, repeated input updates, and high-to-low transitions requesting the joypad interrupt. Propose a System input method that updates buttons even when emulated clocks are stopped. Tests can call it directly; SDL2/keyboard mapping remains M4. Finalize register startup/unused bits and transition rules before implementation.
3. **Ordinary STOP integration.** On the one-time StopEntered event, System resets the divider after the completing entry cycle under the declared coarse ordering. Keep elapsed_ticks cumulative. Existing no-cycle stepping pauses the counter. For the ordinary case of released inputs at entry followed by a selected input going low, System delivers a distinct STOP wake signal to CPU; CPU returns to its fetch state without changing IME or acknowledging IF as part of wake. Pending interrupt service is then governed by the normal acceptance path. Exact wake ordering/delay remains to be specified, rather than silently adding fake idle cycles or reusing HALT's IE&IF wake rule.

Acceptance should show FF04 phase/wrap/reset, HALT continuing divider clocks, STOP resetting/pausing once, host input delivery without elapsed-time movement, selected versus unselected buttons, joypad requests independent of IME/IE gating, and normal resumed execution/entry. Review held inputs and eligible interrupts at STOP entry against the hardware branch diagram before claiming those cases or closing #44. The current always-standby decoder must eventually be revisited for those conditional cases; bolting on a generic resume method is insufficient.

Additional primary references: [joypad register](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Joypad_Input.md), [joypad interrupt](https://github.com/gbdev/pandocs/blob/master/src/Interrupt_Sources.md#int-60--joypad-interrupt), [shared counter and timer edges](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_Obscure_Behaviour.md). No application changes, fixtures or new test run accompany this roadmap.

Divider prerequisite #46 is now implemented and verified: System advances devices for the STOP opcode cycle, resets the counter once after StopEntered, and advances nothing on later stopped polls. All 390 tests pass, including 11 divider tests; [evidence](DIVIDER.md#divider-completion-verified---2026-11-18). This completes the roadmap's divider slice only. JOYP/input/wake and conditional STOP cases still require their contracts; no external ROM ran.

## CycleRequest migration review - 2026-11-18

User added CycleRequest::{Execute(CycleAction), Stopped}, wrapped every existing CPU action, and rejects stopped completion with CpuError::NoCycleRequested before mutation. Fault propagation and wrong-kind response validation remain in place. System unwraps Execute, with a temporary todo for Stopped; that branch is currently unreachable because no Stopped execution state exists yet.

Assistant migrated existing CPU-request tests through a test-only helper that requires Execute and fails on unexpected Stopped, preserving their action/result/flag/memory/trace oracles. All 371 tests pass (271 unit, 100 integration); cargo check --lib, formatting, strict Clippy and whitespace checks pass. This verifies existing execution after the request-type migration; it does not exercise stopped completion or no-cycle System results. No external ROM suite ran.

Next user slice: define MachineStep in system.rs, return Cycle for real traces and Stopped before tick advancement for stopped requests, then adapt bounded instruction stepping to return InstructionStep::Stopped without counting a cycle. Assistant owns the resulting System-test migration. Finish that path before adding STOP decoding or an ExecutionState::Stopped that makes it reachable.

## MachineStep migration verified - 2026-11-18

System now returns MachineStep::Cycle for actual traces and MachineStep::Stopped before advancing time or completing a cycle. Bounded stepping reports InstructionStep::Stopped using the count of previously executed cycles (the loop index, without adding one). Zero budgets retain BudgetExhausted with no work. The temporary todo is removed.

Assistant migrated System-step tests through a test-only helper requiring a real Cycle, preserving trace/flag/memory/budget oracles and explicitly rejecting unexpected standby. Existing exhaustive result matches also reject Stopped for ordinary memory operations. Fresh verification passes all 371 tests (271 unit, 100 integration), formatting, strict Clippy and whitespace checks. No external ROM suite ran.

This checkpoint verifies existing clocked execution under both new result wrappers. ExecutionState::Stopped does not exist yet, so no-cycle polling and NoCycleRequested rejection are reviewed structurally but not exercised. Add those tests when standby becomes reachable; do not claim STOP support from this checkpoint.

Next user slice: STOP decode, private Stopped/StopFinished, public StopEntered, shared delayed-EI completion bookkeeping and bounded handling of the entry event. StopEntered is a real instruction-completing cycle; a later MachineStep::Stopped is a no-cycle poll. The former counts its entry cycle, the latter does not. Follow the ordinary-standby approximation above; divider/joypad integration stays in #44.

## STOP acceptance review - 2026-11-18

State/transition/event wiring, stopped completion rejection, System entry versus polling counts and shared delayed-EI handling are implemented. One correctness finding remains: the decoded STOP arm returns StopFinished immediately, so only the shared opcode increment happens. Ordinary entry retains P+1 rather than the required P+2; with HALT fetch suppression it retains P rather than P+1. Reuse the existing wrapping increment in the STOP-specific arm, preserving the shared fetch path and adding no bus cycle. The user owns this application fix.

Assistant added eight tests in cpu/tests/stop.rs and system/tests/stop.rs covering entry/register preservation, flags/IME/pending-mask combinations, invalid-response atomicity, all stopped response kinds, wake/acceptance guards, fetch suppression composition, full-memory preservation, entry/poll traces, zero/large budgets, EI/DI, interrupt-before-STOP/RETI and HRAM/IE/ROM PC boundaries. Hardware conditional STOP cases remain explicitly outside this deterministic approximation.

Existing decoder/protocol/EI tests now recognize STOP as supported and its event as instruction completion in the applicable oracle; the old unsupported-STOP integration case now uses invalid D3. The full run initially passed 372 tests (272 unit, all 100 integration) and failed seven: six padding-related acceptance failures and one stale all-base EI oracle. After migrating that oracle, its focused test passes. The focused STOP run passes two tests and fails six, all at padding PC assertions. Full suite was not repeated after that final test-only migration. Library check, formatting, strict Clippy and whitespace checks pass. No external ROM ran.

Acceptance and coverage promotion remain pending the padding fix and subsequent checks. The passing stopped-guard and EI/DI tests do not establish complete STOP behavior; #44 and M2's external validation gate remain open.

## Core standby completion verified - 2026-11-18

User added the STOP-specific wrapping PC increment, retaining the shared fetch/suppression path. All eight focused STOP tests now pass, including ordinary P+2, suppressed P+1 and FFFE/FFFF wrap cases. Fresh full verification passes all 379 tests (279 unit, 100 integration), formatting, strict Clippy and whitespace checks. The earlier six acceptance failures and stale test expectations are resolved. No application behavior was changed by the assistant.

Evidence covers one opcode-read entry cycle/four ticks, padding skipped without another bus action, registers/flags/SP preservation, private stopped state and one StopEntered event, shared delayed-EI promotion, immutable/retry-safe requests, rejection of all completion kinds while stopped, no interrupt wake/acknowledgment during polling, zero/positive/large budgets, no-cycle polls, full-memory preservation, interrupt-before-fetch/RETI and HALT-suppression composition. The latter and nonzero-padding/pending-at-entry fallbacks verify the declared model only, not conditional hardware behavior.

Implemented encoding coverage is now 244/244 legal non-prefix base plus 256/256 CB = 500/500, excluding the CB dispatch byte and eleven invalid base slots. STOP's core approximation has no emulated wake path yet. Divider reset/pause, selected JOYP wake, hardware prefetch/conditional entry and oscillator behavior remain #44. No external ROM source/binary was imported or run. Keep M2 open for its ROM-validation evidence; next planning work is a pinned/licensed CPU ROM subset and a bounded headless validation contract with explicit device prerequisites and success/failure reporting.
