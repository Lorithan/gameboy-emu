# HALT, wake-up and the HALT bug

GitHub task: [#42](https://github.com/Lorithan/gameboy-emu/issues/42), M2 - CPU execution.

Status: specified 2026-11-18; implementation has not started. Behavior requirements are sourced below; proposed API/state names and coarse timing choices need user review before implementation. User writes application code; assistant owns tests/docs/review.

Baseline a40668f: prior verified suite 361 tests (261 unit, 100 integration), formatting/strict Clippy pass; not rerun for this specification. Coverage remains 242/244 non-prefix base plus 256 CB until HALT is verified. STOP is separate.

## Required behaviors and scope

76 HALT completes its opcode cycle once, preserves general registers/F/SP, then may suspend opcode execution. A pending source means IF & IE & 1F, regardless of IME. Distinguish:
- No eligible request: enter halted execution. Advance time without fetching instructions until a request becomes eligible.
- Wake with IME disabled: resume ordinary fetch without servicing or acknowledging the request. A request arriving after normal halt does not itself cause the HALT bug.
- Wake with IME enabled: begin ordinary interrupt entry before executing the following opcode.
- HALT executes while IME is not yet enabled and a request is already eligible: do not remain halted; suppress the next opcode fetch's PC increment once (the HALT bug).
- EI delay still matures when HALT completes. A pending interrupt after EI;HALT must have the sourced return-address behavior described below.

No timer/PPU/frontend/STOP, shared ownership, edge-accurate bus model or ROM fixture additions. Synthetic request injection is sufficient for this task. Existing five-cycle late selection/cancellation remains unchanged.

## Proposed read response

This is a concrete new requirement for samples on a read. Replace Read(u8) with:
CycleResponse::Read { value: u8, pending_interrupts: u8 }.
Keep WriteDone { interrupt_sample } and IdleDone unchanged. No optional read sample or outer input struct.

System advances four ticks, performs the read, samples Bus.pending_interrupts(), then supplies both values to CPU completion. Current register reads are pure. CPU masks pending bits itself. Ordinary read handlers use read_value() and ignore pending; HALT uses both. Sampling every read lets System remain independent of private CPU phases. Write sample ordering remains IF-before-write/IE-after-write.

read_value() still returns u8 or the response-kind error. Add a constrained read-specific extraction/helper if needed; do not make a general optional sample that reintroduces a missing-data case. Validation must happen before any register/state/IME/fetch-latch mutation. Migrate this API in a reviewed slice before adding HALT behavior.

These are coarse M-cycle sampling points. Future device signals occurring during clock advancement are evaluated at these documented boundaries; physical within-cycle wake edges are not established by this task.

## Proposed states and transitions

- Add DecodedOpcode::Halt for 76; preserve exclusion from LD (HL),(HL).
- Add private ExecutionState::Halted.
- Halted request_cycle returns Idle. Correct IdleDone completion remains Halted and returns InProgress. No register or memory change and no new instruction boundary.
- Add a private one-shot suppress_next_opcode_increment latch, initially false. It records a fetch effect, not a second IME flag.
- At successful HALT opcode completion, use the read's pending mask and current pre-bookkeeping IME. If IME is Enabled or pending is zero, choose Halted; otherwise choose FetchOpcode with the suppression latch armed.
- Represent successful HALT finishing explicitly in CycleTransition so the completion wrapper can select Halted or FetchOpcode while reporting InstructionComplete. Do not use ordinary InstructionFinished if it unconditionally restores FetchOpcode.
- Every successfully completed instruction, including HALT, runs delayed-EI maturation exactly once in the central wrapper. Refactor the wrapper to apply that rule based on InstructionComplete after transition application rather than copying it into unrelated branches. Halted idle and interrupt events never mature the delay.
- Application handlers may still update registers after validation. Execution-state application remains centralized in the completion wrapper, except existing explicit acceptance/wake operations.

HALT's opcode completion reports InstructionComplete whether it halts or triggers the bug. A following step with no wake request consumes halted idle cycles and returns BudgetExhausted. Do not invent a successful instruction on each halted idle.

## Wake and System ordering

Propose a small mutating CPU wake operation, suggested wake_if_pending(pending), called by System before try_accept_interrupt and request_cycle on every positive machine-cycle step. Only Halted with a nonzero masked request changes to FetchOpcode. Leave requests/IME/registers/faults unchanged. Wake itself adds no ticks or dummy idle cycle.

The existing try_accept_interrupt then either starts entry when Enabled or leaves a normal fetch when Disabled. Keeping wake separate makes the different IME rules visible. A combined polling method is acceptable if those responsibilities and ordering remain explicit.

System order is wake -> try acceptance -> immutable action query -> four ticks -> Bus action and required samples -> CPU completion -> selected-source acknowledgment -> trace. A zero-budget step calls none of those operations. If a request appears during one halted idle's tick advancement, it is observed at the next coarse polling opportunity.

IME-enabled pending requests normally preempt a HALT opcode before it is fetched. If a request becomes eligible during the opcode cycle, finish HALT then recognize it on the next step, with no extra wake penalty.

## Fetch suppression and EI interaction

In read_opcode, capture the instruction-start address normally. After a validated opcode read:
- If the suppression latch is clear, increment PC once as today.
- If set, clear the latch and omit that one increment.
Operand reads, data reads, CB subopcode reads and future fetches advance normally. Repeated request queries and invalid responses cannot consume the latch.

Example with IME disabled and a pending source:
HALT; LD A,12 uses the LD opcode byte 3E again as its immediate operand, loading A=3E. A one-byte opcode after HALT executes twice because the first fetch does not advance PC. RST pushes the address of its own opcode; test this independently. A CB prefix consumes suppression on its prefix fetch only, so its second read sees CB again.

EI;HALT requires special attention: if pending at HALT completion while IME is Delayed, HALT arms suppression and its instruction completion enables IME. The next step accepts an interrupt before the suppressed fetch. In this coarse non-prefetch model, acceptance must consume the armed latch by rewinding the captured return PC by one (wrapping), then clearing the latch so it does not affect the handler's first opcode. That return address points to HALT. Returning through RETI can therefore execute HALT again.

This acceptance-time correction is a proposed translation of sourced EI/HALT behavior into our fetch convention, not a claim about hardware internal bookkeeping. It belongs only to acceptance with the armed HALT effect; ordinary interrupt entries retain their current PC. Cancellation still consumes the effect once and follows the existing destination-0000 rule. Do not let the latch leak into a handler or survive a serviced/canceled entry.

With EI;HALT and no pending source during HALT completion, HALT enters Halted, the delay matures, and a later request services normally with return PC after HALT. Do not treat Delayed as Enabled when deciding whether a currently pending source triggers the bug, and do not skip HALT's instruction-boundary bookkeeping.

## Acceptance criteria (assistant tests)

- Independent 76 mapping, supported-opcode classification, preservation of HALT exclusion from byte transfers; invalid slots and STOP remain unsupported. Update coverage only after acceptance.
- All initial flags, IME states and low pending masks/upper-bit noise; pending at opcode completion versus arriving after entering Halted; preserve registers/SP/memory and capture PC wrapping.
- Exact one-cycle HALT completion, then repeated Idle/InProgress traces with four ticks each and unchanged PC. Zero/partial/excess budgets are bounded/resumable, never count idle cycles as instructions or hang.
- Wake IME-disabled without entry/IF clear, wake IME-enabled with existing five-cycle entry and late selection; IF alone or IE alone is insufficient. No emulated wake penalty beyond the modeled cycles.
- Mandatory read-response sample migration, wrong-kind retry atomicity and ignored pending values for non-HALT reads. Pending read samples do not acknowledge interrupts.
- Suppression consumed only after successful opcode fetch, including wrong responses/repeated queries/zero budgets; immediate-byte duplication, repeated single-byte execution, CB prefix and RST return address, wrapping and repeated HALT sequences.
- EI;HALT with pending source: correct stacked HALT address and clean handler fetch. Two buffered sources can service in priority order through RETI before HALT re-executes, then wait for a later injected request whose return PC is after HALT. Also cover no-pending EI;HALT, EI;DI;HALT, cancellation and preservation of ordinary EI/RETI behavior.
- Keep full-memory/stack/trace/IME/late-cancellation regressions. Run full tests, formatting, strict Clippy and whitespace checks. No external ROM conformance claim without actually running a versioned/licensed fixture.

## Review slices

1. Review/accept read sampling, halted instruction-completion transition and EI/HALT translation. Migrate read response without changing execution behavior; assistant validates existing regressions.
2. Implement halted idles and wake polling, with shared instruction-boundary IME bookkeeping; assistant adds ordinary HALT tests.
3. Implement fetch suppression and EI interaction, then full acceptance tests. No checkpoint is called complete HALT support until the bug/interaction cases pass.

## Sources consulted 2026-11-18

- [Pan Docs HALT](https://raw.githubusercontent.com/gbdev/pandocs/master/src/halt.md): wake conditions, HALT bug, EI and RST interactions.
- [RGBDS instruction reference](https://raw.githubusercontent.com/gbdev/rgbds/master/man/gbz80.7): instruction behavior.
- [Mooneye halt_ime0_nointr_timing](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/halt_ime0_nointr_timing.s), [halt_ime0_ei](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/halt_ime0_ei.s), [halt_ime1_timing](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/halt_ime1_timing.s), [DMG timing2](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/halt_ime1_timing2-GS.s): hardware-tested reference cases; timer/PPU-dependent execution is not presently available.
- [SameSuite ei_delay_halt](https://raw.githubusercontent.com/LIJI32/SameSuite/master/interrupt/ei_delay_halt.asm): stacked return-address/counter expectations for buffered requests and EI/HALT.

Sources are references only; no ROMs/source fixtures copied or run. This task fixes observable coarse-model behavior and does not establish physical bus-edge, timer or PPU accuracy. Baseline checks were not rerun for this documentation-only design work.
