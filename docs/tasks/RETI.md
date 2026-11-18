# RETI: return and enable interrupts

GitHub task: [#41](https://github.com/Lorithan/gameboy-emu/issues/41), M2 - CPU execution.

Status: implemented and verified on 2026-11-18. Required behavior below is sourced; the completion section records the selected private representation. User owns implementation; assistant owns tests/docs/review.

## Scope and baseline

Implement D9 RETI only, after #40. Baseline: 11a2eca, 355 passing tests (255 unit, 100 integration), formatting and strict Clippy from the preceding verified checkpoint; not rerun for this specification. CPU currently supports C9 RET and conditional returns using ReadReturnLow, ReadReturnHigh and final ApplyJump. Current coverage is 241/244 legal non-prefix base instructions and all 256 CB; after verified RETI it will be 242/244, leaving HALT and STOP. Do not change coverage until implementation is tested.

No HALT/STOP, peripherals, frontend, dependencies, ROM fixtures, timing refinement or broad refactor in this issue.

## Required behavior

D9 is one byte/four M-cycles (16 ticks). Pop the low then high return-address bytes from the ordinary Bus stack, increment SP twice with u16 wrapping, and set PC to that target. Preserve all general registers and F. RETI does not require an earlier interrupt or CALL and does not track host-language call nesting.

At successful final completion, assign CPU IME Enabled directly, clearing any delayed-enable interpretation. This applies from Disabled, Delayed and Enabled. Do not emulate EI after RET: that would delay enabling until an extra instruction completes. Do not enable at opcode fetch or either stack read in our M-cycle convention.

RETI itself neither reads nor clears IF/IE as control operations; ordinary stack reads may target those mapped registers. It emits no interrupt-selection acknowledgment and does not service a pending request itself. System accepts a pending interrupt on its next machine-cycle call after RETI finishes, before fetching the returned-to opcode.

An enabled CPU may accept an interrupt before fetching RETI, just like any other instruction. Once RETI starts, entry cannot interrupt its operand/idle stages. Do not special-case RETI in System's scheduler.

## Cycle contract

Let P be the opcode address, S the stack address captured after successful opcode validation, and T the assembled target.

| Cycle | Requested action | Successful completion |
| --- | --- | --- |
| 1 | Read opcode at P | PC=P+1 wrapping; capture S and RETI mode; report InProgress |
| 2 | Read low byte at S | Capture low; SP=S+1 wrapping; PC remains P+1; report InProgress |
| 3 | Read high byte at S+1 | Capture T; SP=S+2 wrapping; PC remains P+1; report InProgress |
| 4 | Idle | PC=T, IME=Enabled; InstructionFinished restores FetchOpcode centrally and reports InstructionComplete |

Preserve the initial IME value until cycle 4. The existing completion wrapper continues to run its instruction-boundary rule; because RETI explicitly sets Enabled, it must not reintroduce Delayed or postpone the effect.

Incorrect CycleResponse kinds at any stage leave registers, execution state, captured data and IME unchanged. A correct retry completes only the pending CPU cycle. Repeated request_cycle calls remain immutable; no invented memory write or extra fetch/idle is allowed.

## Proposed architecture

- Add a distinct decoded RETI operation (suggested ReturnInterrupt) mapping D9. Preserve ordinary Return and conditional decode behavior.
- Introduce a small typed return mode, suggested ReturnKind::{Normal, EnableInterrupts}. Carry it through the captured return low/high states. Ordinary and conditional RET enter with Normal; RETI enters with EnableInterrupts.
- Use a return-specific final idle state, suggested FinishReturn { target, kind }, so final return behavior can set PC and optionally enable IME without adding unrelated effects to generic ApplyJump. JR/JP continue using ApplyJump. RET and taken conditional RET retain their exact current cycle paths.
- Final return handler commits PC and, for RETI mode only, IME Enabled, then returns existing CycleTransition::InstructionFinished. The wrapper remains the sole execution-state assignment point; no public completion variants are needed.
- Normal RET has no explicit IME effect, but still participates in existing delayed-EI instruction bookkeeping. In particular EI;RET must still mature the delayed enable at RET completion.
- Exact helper names/location or an equally constrained continuation are user choices. Do not create a general jump-effects framework, a global pending-RETI boolean, or duplicate entire return implementations. A mode carried in the pending state avoids stale enable intent leaking into later instructions.
- Keep CPU free of Bus borrows, keep IME private, and use the current CycleResponse/trace/step interfaces. Public stepping reports Complete for RETI, then InterruptEntered/InterruptEntryCancelled separately if the subsequent entry occurs.

Example: with S=FFFC, bytes 34 and 12 at FFFC/FFFD, RETI finishes at PC=1234, SP=FFFE with IME Enabled. With an eligible Timer request, the next step performs entry and pushes return address 1234; it must not fetch/execute ROM[1234] first.

## Acceptance (assistant owns tests)

- Decode D9 explicitly and retain independent classification of all base bytes/CB prefix. Update tests that still list D9 as unsupported; keep negative cases for HALT, STOP and invalid slots.
- Exhaust all 65,536 target values and a separate all-SP sweep with wrapping; all sixteen F patterns and initial IME states at representative targets. Preserve all unrelated registers, memory and flag bits.
- Exact four-action trace and intermediate PC/SP/IME. Capture the initial stack address and low byte across pauses and test-only register mutations. Verify target assembly and no opcode/stack-address confusion.
- Wrong response kinds at every read/idle stage: unchanged snapshots, pending actions/captures, IME and no acknowledgment; correct retry continues. Include opcode PC=FFFF and S=FFFF stack wrap independently, plus mapped ROM/WRAM/echo/HRAM/unmapped/IF/IE read behavior.
- Zero/partial/exact/excess budgets, including a pause before final idle: zero changes nothing, enable occurs only at final completion, excess stops before returned-to opcode or pending interrupt entry. Per-call cycle/tick counts remain correct.
- Compare C9 RET and RETI from disabled IME: identical return actions/target/SP/memory, different final IME. Verify ordinary/conditional returns and JR/JP remain unchanged; EI;RET bookkeeping remains intact.
- Synthetic two-pending-source program based on reti_intr_timing: EI;INC B allows one INC B, first handler updates a register then RETI, second pending interrupt begins before the second mainline INC B. Verify entry priority, stack restoration/re-push of the same return PC, IF acknowledgments and final counters.
- No-pending RETI executes the returned-to opcode on the next step. RETI with pending requests becomes interruptible immediately at the next boundary from Disabled or Delayed; no extra instruction delay. Existing late-selection/cancellation tests remain green.
- Full cargo test --all-targets --quiet, cargo fmt --check, cargo clippy --all-targets -- -D warnings, git diff --check. Report actual evidence; no external ROM conformance claim.

## Implementation order

1. Add decoded RETI and constrained return-mode/final-state representation; migrate existing return paths without changing C9/conditional timing.
2. Wire RETI mode through the reads and enable IME during the final idle.
3. Assistant reviews, adds/migrates acceptance tests, runs full regressions, records evidence and publishes the verified checkpoint.

## References checked 2026-11-18

- [RGBDS instruction reference, RET/RETI](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7): four cycles, one byte, unchanged flags and enable at instruction completion.
- [Pan Docs interrupts](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupts.md): IME ownership and instruction behavior.
- [Mooneye reti_intr_timing source](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/reti_intr_timing.s): primary hardware-tested immediate re-entry scenario.
- [Mooneye reti_timing source](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/reti_timing.s): hardware-tested cycle ordering. Its DMA/PPU-dependent execution is outside today's implemented devices.

Mooneye sources consulted as references only; no source copied, ROM binary imported or ROM suite run. Coarse internal commit points are project conventions consistent with observable instruction boundaries, not a physical-edge claim.

## Implementation and acceptance evidence - 2026-11-18

User implemented D9 ReturnFromInterrupt with ReturnKind::{Normal, Interrupt}. Shared ReadReturnLow/ReadReturnHigh carry the mode into FinishReturn; normal/conditional RET use Normal, and only Interrupt sets IME Enabled at the final idle. ApplyJump stays separate for JR/JP. The response/transition/event APIs and System scheduling are unchanged.

Assistant added six tests (three CPU, three System), minimal module wiring and updated the two decoder expectations that treated D9 as unsupported. CPU tests cover all 65,536 target addresses and a separate all-65,536 SP sweep with opcode PC=FFFF, all sixteen F patterns and all three initial IME states for RET/RETI, exact actions/intermediate snapshots, repeated queries, wrong response kinds at every phase and correct retry, plus captured SP/low byte/return mode under test-only mutation. Normal RET retains existing delayed-EI maturity.

System tests cover every zero/partial/exact/excess budget, full-address-space memory preservation and stack reads through WRAM, echo, HRAM, IF/IE, ROM/unmapped boundaries and FFFF wrap. Pending Timer entry begins on the next call after RETI from Disabled or Delayed without fetching the returned-to opcode. A synthetic EI;INC B program services VBlank then Serial handlers through RETI before executing a second mainline INC B; it restores/re-pushes the same return PC, updates D/E once each and takes 92 ticks in total. With no remaining request, the second RETI resumes the mainline normally.

The first focused run caught an incorrect expected total in the new synthetic test (88 versus the correct 92); that oracle was corrected. Initial regression also found a second stale D9-unsupported classification; updated independently. No application correctness fix was required. The six focused RETI tests pass; formatting, strict Clippy and diff whitespace checks pass. No ROM source/binaries were imported and no external suite ran; DMA/PPU timing and physical edge accuracy remain unverified.

Final verification: all 361 tests pass (261 unit, 100 integration), including all six focused RETI tests. Formatting, strict Clippy and whitespace checks pass. #41 is complete; next specify HALT/wake/HALT-bug behavior, with STOP separately scoped. M2 remains open for remaining instructions and ROM validation evidence.
