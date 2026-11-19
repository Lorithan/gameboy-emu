# Selected-line STOP entry and wake

Task: [#48](https://github.com/Lorithan/gameboy-emu/issues/48), under [#44](https://github.com/Lorithan/gameboy-emu/issues/44). Divider #46 and headless JOYP #47 are complete. User owns application implementation; assistant owns tests/docs/review. Level queries, read sampling, conditional entry and immediate host-input wake are verified for the declared deterministic scope. Original specification baseline was 03aff43 with 401 passing tests; slice evidence below supersedes it.

## Integration issue audit - 2026-11-19

#44's deterministic STOP divider/joypad integration is complete through #46/#47/#48. Source review confirms Stopped returns before System advances elapsed/device ticks, only StopEntered applies the divider reset, and System::set_button updates Bus/IF then offers the selected-line level to the guarded CPU wake. Existing entry/wake/polling/clock/interrupt traces are included in the latest 442-test verification at dcedd79. No tests rerun for this documentation-only audit. Physical DMG revision/startup/edge/glitch assessment moves to separate deferred [#64](https://github.com/Lorithan/gameboy-emu/issues/64), allowing integration parent #44 to close. Current support remains the documented coarse deterministic DMG model, with no physical revision or analog/glitch fidelity claim. Earlier statements below keeping #44 open for those limits are historical and superseded; their accuracy limitations still apply. External STOP-specific hardware validation is not claimed.

## STOP wake completion verified - 2026-11-18

User implemented guarded CPU wake_from_stop_if and System delivery after Bus button/IF updates. Fresh verification passes all 414 tests (312 unit, 102 integration), including 22 focused STOP-related tests and 13 focused joypad unit tests. Formatting, strict Clippy and whitespace checks pass. Assistant added five wake tests and migrated the earlier Stopped input-purity expectation explicitly; assertions for other states and unrelated memory remain intact. No assistant application logic edits, allowances, dependencies or fixtures.

Evidence covers inactive guards, preserved registers/PC/IME/fetch latch, idempotence, all eight buttons/four selections, unselected/no-row inputs, rapid press/release between execution calls, stale IF/pre-entry pulses, one-byte/two-byte resume PC, IE/IME-independent wake, zero input time/divider effects, zero budgets, mapped-memory preservation and resumed divider phase. Pending enabled interrupts enter on the next step through five cycles and late acknowledgment; wake itself neither writes the stack nor acknowledges IF. A public-API test resumes INC B immediately after a full input pulse without an extra wake cycle. Existing fault/partial-entry/HALT/running purity checks pass.

#48's declared deterministic scope is complete. #44 stays open for oscillator startup, physical edge timing, zombie power and nondeterministic wake-glitch accuracy. No external ROM ran, so M2's ROM gate remains open. Next planning work can revisit deferred #45's bounded ROM-validation contract and provenance before fixtures. Earlier pending-wake notes below are historical.

## Conditional-entry completion verified - 2026-11-18

User added the missing wrapping padding increment to the Halt completion arm. Fresh verification passes all 409 tests (308 unit, 101 integration), all 18 focused STOP-related tests, formatting, strict Clippy and whitespace checks. Assistant application logic changes: none. The failed-review notes below describe the previous intermediate state.

CPU coverage now includes every pending-mask byte, both selected-line levels, all IME modes, PC wrapping, fetch suppression consumption, retry rejection and following actions across all four branches. Existing flags/register preservation tests now use conditional padding expectations. Four independent System tests cover real selected/unselected inputs, unrelated VBlank eligibility, upper IE noise, delayed EI, zero budgets, exact fetch/time/outcome, all mapped memory, actual-STOP-only divider reset, no-cycle stopped polling, retained HALT divider phase and continued fetch. These supply behavioral evidence for the previously structural System read-sample connection.

This verifies the declared coarse DMG entry model, not oscillator startup, analog timing or power consumption. No external ROM was imported or run. #48/#44 remain open: System input still only updates Bus/IF, so selected input does not yet wake Stopped. Next: the guarded CPU transition and immediate input delivery described below, then pulse/IE/IME/purity/interrupt-entry acceptance. Preserve #47's CPU-state guarantees for other execution states and update its Stopped expectation explicitly.

## Read-sample slice review - 2026-11-18

Fresh full verification: all 405 tests pass (304 unit, 101 integration); formatting, strict Clippy and whitespace checks pass. Includes the three level-query tests from the previous slice and the new accessor test. Conditional entry/wake remain unimplemented; #48/#44 stay open.

User added the mandatory Read field, its typed accessor, and System sampling after each bus read alongside pending_interrupts. Review confirms CPU still ignores the new level, preserving existing STOP behavior. Assistant migrated test response constructors with false as the ordinary default. All five focused protocol tests pass: the new accessor test exhausts both levels and every byte/pending value, checks wrong response kinds, and ordinary-read regression now exercises both wake levels across immediate, indirect and CB reads with all pending masks and IME states. Existing phase/retry tests retain their response-kind checks.

Strict Clippy passes without temporary allowances; the production Bus/Joypad callers resolve the earlier dead-code errors. System sample timing is structurally reviewed; CPU behavior tests cannot yet demonstrate transport of the level through System because the CPU does not consume it. Conditional STOP integration tests will supply that evidence. No external ROM ran. Next user slice: carry a private StopBehavior through StopFinished, pass the read sample into opcode handling, and apply the documented entry table with conditional wrapping padding in the shared completion wrapper. Host-input wake follows separately.

## First implementation slice

Add `Joypad::stop_wake_active(&self) -> bool`, visible to its parent module, and a crate-visible Bus method forwarding the same query. Return true exactly when at least one selected P10-P13 line is low. Derive it from the existing private `input_lines()` result: all four high means inactive. A held button in an unselected row is inactive; either selected row can assert a shared line. With neither row selected, the signal is inactive regardless of buttons.

Both methods are pure: no stored wake flag, interrupt request/clear, button update, CPU transition or time advancement. Keep `input_lines()` private. Do this slice first; the following sections describe dependent work for review.

### Level-query slice review - 2026-11-18

Fresh full regression: all 404 tests pass (303 unit, 101 integration); formatting and whitespace checks pass. No external ROM ran. Strict Clippy has only the unused-query errors described below, so finish the production sampling slice before the next verified commit checkpoint.

User implemented both queries with the agreed visibility and no additional state. Three assistant-owned acceptance tests pass: exhaustive held-mask/selection queries, Bus forwarding independent of representative IF/IE combinations, and repeated-query preservation of all mapped memory, hidden DIV phase and instance isolation. Strict Clippy currently rejects the two unused production methods; the next read-response sampling slice will supply their production caller. No temporary allowances or application changes were added by the assistant. CPU STOP behavior remains unchanged. Next: add mandatory `stop_wake_active` to Read responses and sample it in System, preserving existing execution before implementing entry branches.

## Distinct signals

The selected input level answers whether WAKE is active now. `JoypadEvent::FallingEdge` requests joypad IF when a selected line changes from high to low. IF persists after the input returns high; eligible pending interrupts are `IE & IF & 0x1F`. These signals cannot replace each other.

Propose adding mandatory `stop_wake_active: bool` to `CycleResponse::Read`, alongside value and pending_interrupts. System samples both signals after each bus read. Other read consumers ignore the wake sample. This keeps CPU independent of Bus and avoids optional data that could be missing at STOP completion. WriteDone/IdleDone and wrong-kind validation stay as they are. Assistant migrates test response builders with false as the ordinary default, then supplies explicit samples in STOP tests.

## Conditional entry

### Conditional-entry acceptance review - 2026-11-18

User implemented the three completion arms. Stop conditionally skips padding and emits StopEntered; Continue resumes FetchOpcode and emits InstructionComplete. Halt installs Halted/InstructionComplete but omits its unconditional padding increment. A selected held input with no pending eligible interrupt therefore leaves PC at P+1 instead of P+2. The required user fix is the shared wrapping PC increment in that arm; preserve existing shared EI promotion and divider event handling.

Assistant migrated the old CPU unconditional-padding oracles, expanded CPU tests across all pending masks, both wake levels, IME modes, wrapping and fetch suppression, and added four separately runnable System branch tests. System cases exercise selected/unselected rows, unrelated VBlank pending flags, delayed EI, zero budgets, complete memory preservation, outcomes and real/no-cycle continuation; the HALT case will also check retained DIV phase after the PC assertion passes. Focused command cargo test --lib stop --quiet: 16 pass, two fail, both reproducing the omitted HALT-like padding increment. The other three System entry branches pass independently. Formatting, strict Clippy and whitespace checks pass. No full regression rerun with this known defect, no passing checkpoint claimed, no assistant application fix, no external ROM executed. #48/#44 remain open.

### Behavior-selection review - 2026-11-18

User implemented StopBehavior and its transition payload, threaded the Read wake sample into read_opcode and selected the documented branches using pending & 0x1F. Decode's unconditional padding increment is removed. Structural review agrees with the contract. The wrapper remains StopFinished(_) -> Stopped/StopEntered; it must consume the behavior before this slice is complete. Existing focused CPU STOP tests report one pass/two failures: ordinary no-pending entry now lacks its padding increment, and an old suppressed-fetch/pending test still expects unconditional padding. Assistant will migrate that obsolete oracle and add the full four-branch acceptance matrix when the wrapper is implemented. No full suite rerun or new passing checkpoint claimed; no assistant application edits or external ROM execution.

The wrapper must conditionally skip padding for Stop, always skip for Halt and never skip for Continue. Install Stopped/StopEntered, Halted/InstructionComplete or FetchOpcode/InstructionComplete respectively. Preserve shared delayed-EI bookkeeping and the existing System reset hook, which only actual StopEntered triggers. These completion changes are the next user slice; host-input wake remains separate.

The [Pan Docs STOP chart](https://raw.githubusercontent.com/gbdev/pandocs/master/src/imgs/stop_diagram.svg) distinguishes these DMG branches. The [CPU internals reference](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595) also describes the pending/no-WAKE branch as zombie mode. Translate them into this project's coarse model as follows:

| Selected input low | Eligible interrupt pending | Skip padding | Next state | Completion outcome | Reset DIV |
| --- | --- | --- | --- | --- | --- |
| No | No | Yes | Stopped | StopEntered | Yes |
| No | Yes | No | Stopped | StopEntered | Yes |
| Yes | No | Yes | Halted | InstructionComplete | No |
| Yes | Yes | No | FetchOpcode | InstructionComplete | No |

Pending is masked to five bits and does not require IME. System's existing enabled-interrupt acceptance before opcode fetch still takes precedence; the table applies when STOP is actually fetched. We do not reproduce zombie-mode power consumption.

Keep one real opcode-read M-cycle and no padding bus read, as in the current approximation. Apply the usual successful opcode increment/suppression first, then the conditional wrapping padding increment. Never reconstruct PC as opcode address plus two. Preserve flags/SP/general registers. Every branch is an instruction completion for shared delayed-EI bookkeeping. The HALT-like branch does not independently apply the HALT opcode's suppression decision.

Proposed private representation: `StopBehavior::{Stop { skip_padding: bool }, Halt, Continue}` carried by `CycleTransition::StopFinished(StopBehavior)`. Halt implies padding; Continue implies no padding. The completion wrapper applies the padding and state/outcome together; decode selects behavior rather than mutating state. This replaces the current unconditional STOP padding increment. Names remain user choices. Avoid independent next-state/reset/padding fields that permit contradictory combinations. Only actual StopEntered invokes the existing System divider reset.

## Delivering wake without a running clock

Propose a crate-private CPU method such as `wake_from_stop_if(wake_active: bool)`, returning unit. It changes only Stopped with an active signal to FetchOpcode, preserving PC, registers, IME and other latches. An inactive signal or any other execution state does nothing.

In `System::set_button`, first perform the existing Bus input update and IF event handling. Then sample the Bus wake level and offer it to that CPU method. Waking costs zero emulated cycles/ticks, does not reset or advance DIV, and does not acknowledge/service an interrupt. The next real step uses existing wake/arbitration logic; eligible enabled interrupts can then enter through the existing five-cycle protocol. IF alone cannot resume Stopped.

Delivering wake during the input call matters: a selected press followed by release between execution calls must leave CPU ready to fetch. Its execution state retains that wake without another pending-input flag or queue. Input while running must not queue a wake for a later STOP. Already-held selected input is handled by the entry table. No extra polling/mutation in request_cycle, fake idle wake cycle, or second System stopped flag is needed.

This deliberately extends #47's CPU-state preservation contract only for Stopped with active selected input. Preserve its guarantees for running, halted, partial-entry and faulted states. Public input still advances no time. JOYP selection writes already happen during a CPU cycle; entry sampling handles their resulting level.

## Acceptance evidence to add after implementation

- Exhaust all held-button masks and row selections for pure Joypad/Bus level queries; preserve IF, IE, divider phase and unrelated memory.
- Migrate Read inputs and check ordinary execution, wrong-kind atomic rejection and stopped completion rejection remain intact.
- Exercise all four entry branches, upper pending-mask noise, IME/delayed-EI, wrapping and existing fetch suppression. Verify padding, outcomes, exact real-cycle count and reset only on actual STOP entry.
- Exercise selected/unselected buttons, neither/both rows, shared-line blocking, IE/IME independence, stale IF without active input, and input before entry without stale wake.
- A selected press/release between execution calls wakes once and retains resume PC; input itself changes no registers/time/divider. Repeated/release input has no additional CPU effect.
- Pending interrupt service follows ordinary arbitration after wake; wake itself neither clears IF nor starts stack writes. Existing five-cycle/late-selection tests remain meaningful.
- Preserve no-cycle stopped polling, zero-budget execution, sticky errors, memory expectations and #47 purity tests for every other state. Update STOP-specific expectations explicitly rather than weakening general assertions.
- Run focused acceptance, full suite, formatting and strict Clippy. No ROM execution is implied by synthetic tests.

## Accuracy boundary and sources

[Pan Docs](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Reducing_Power_Consumption.md) describes selected P10-P13 wake and conditional entry. [CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595) records oscillator/interrupt wake glitches; those are not portable deterministic outcomes. Our immediate host-event wake and subsequent ordinary interrupt arbitration are explicit project approximations. No oscillator startup delay, analog transition timing, nondeterministic vector/stack corruption or CGB KEY1 speed switching is proposed. #44 remains open to track these accuracy limits even if #48's declared deterministic scope passes. No external ROM, fixture or dependency is introduced.
