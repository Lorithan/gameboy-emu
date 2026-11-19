# Selected-line STOP entry and wake

Proposed task: [#48](https://github.com/Lorithan/gameboy-emu/issues/48), under [#44](https://github.com/Lorithan/gameboy-emu/issues/44). Divider #46 and headless JOYP #47 are complete. User owns application implementation; assistant owns tests/docs/review. This proposal is not implemented. Baseline is 03aff43 with the previous 401 passing tests (300 unit, 101 integration); no emulator tests rerun for this documentation-only task.

## First implementation slice

Add `Joypad::stop_wake_active(&self) -> bool`, visible to its parent module, and a crate-visible Bus method forwarding the same query. Return true exactly when at least one selected P10-P13 line is low. Derive it from the existing private `input_lines()` result: all four high means inactive. A held button in an unselected row is inactive; either selected row can assert a shared line. With neither row selected, the signal is inactive regardless of buttons.

Both methods are pure: no stored wake flag, interrupt request/clear, button update, CPU transition or time advancement. Keep `input_lines()` private. Do this slice first; the following sections describe dependent work for review.

## Distinct signals

The selected input level answers whether WAKE is active now. `JoypadEvent::FallingEdge` requests joypad IF when a selected line changes from high to low. IF persists after the input returns high; eligible pending interrupts are `IE & IF & 0x1F`. These signals cannot replace each other.

Propose adding mandatory `stop_wake_active: bool` to `CycleResponse::Read`, alongside value and pending_interrupts. System samples both signals after each bus read. Other read consumers ignore the wake sample. This keeps CPU independent of Bus and avoids optional data that could be missing at STOP completion. WriteDone/IdleDone and wrong-kind validation stay as they are. Assistant migrates test response builders with false as the ordinary default, then supplies explicit samples in STOP tests.

## Conditional entry

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
