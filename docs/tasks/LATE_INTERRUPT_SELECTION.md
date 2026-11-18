# Late interrupt selection and IE-write cancellation

GitHub: [#40](https://github.com/Lorithan/gameboy-emu/issues/40), M2.
Status: proposal approved by the user on 2026-11-17. Response-protocol migration is implemented and under regression validation; late selection, completion effects and cancellation outcomes are not implemented yet. #39's acceptance-time selection remains the running implementation. Follow the reviewed slices below; the user owns application implementation.

## Goal and evidence boundaries

Keep CPU/Bus ownership, immutable request_cycle, five resumable entry cycles and private transition application. Replace immediate source selection/IF acknowledgment with later selection. User owns implementation; assistant owns tests/docs/review.

Hardware-tested evidence: Mooneye acceptance/interrupts/ie_push.s demonstrates cancellation or reselection following an upper-byte IE write; a lower-byte IE write cannot cancel it, and cancellation leaves IME disabled. The canceled request remains in IF. These are acceptance requirements.

Supporting implementation evidence, not independent hardware proof: SameBoy Core/sm83_cpu.c samples IE after the upper stack write, combines it with IF around the lower write and clears the selected request later. Its special IF-write path uses the previous IF value and explicitly notes unresolved timing collisions. We will use this to inform a coarse M-cycle convention, not claim verified T-cycle behavior.

The contract below is a proposed project-level schedule consistent with the cited IE test. Device signal edges, DMA/OAM behavior and simultaneous real-device IF writes remain outside this task. Passing synthetic equivalents is not a claim that the external ROM suite ran.

## Ownership and acceptance

- System reads current eligibility from Bus and asks CPU to start entry at an enabled FetchOpcode boundary, just as today.
- Acceptance captures PC and SP, disables IME, but does not choose a source or acknowledge IF. Suggested acceptance return is bool: entry started or no change. Pending is masked to 1F and is only the initial gate.
- Keep registers/flags/PC/SP unchanged at acceptance. Preserve rejection behavior, sticky faults, delayed EI and zero-budget no-op semantics.
- CPU later selects the source from sampled register values. System applies the returned acknowledgment through Bus.
- Do not assume the source that caused entry is the eventual selected source.

## Proposed inputs and effects

Approved design: one CycleResponse enum with mandatory interrupt samples on write completion. The late-selection schedule, completion effects, validation and cancellation outcomes below are the agreed task contract. Exact helper names and private representation remain implementation choices within that contract.

Suggested public types:
- CycleResponse::{Read(u8), WriteDone { interrupts: InterruptSample }, IdleDone}.
- InterruptSample { enable_after_bus: u8, flags_before_bus: u8 }.
- CycleCompletion { outcome: CycleOutcome, acknowledge: Option<Interrupt> }.
- complete_cycle takes CycleResponse and returns Result<CycleCompletion, CpuError>.

CycleResponse records how the driver serviced the CycleAction returned by request_cycle. The name describes that request/response relationship more precisely than the general CycleInputs. A successful read supplies Read(value), every completed write supplies WriteDone { interrupts: sample }, and a completed idle cycle supplies IdleDone. There is no outer CycleInputs struct, optional sample or generic None response. A write acknowledgment cannot omit the required samples, and a read or idle response cannot carry unused samples.

The field names intentionally expose the sampling order. System collects flags_before_bus after advancing this cycle's four ticks but before its bus action. It collects enable_after_bus after servicing that action. These are samples for the current cycle, not long-lived shared references. Existing IF/IE reads are pure; no extra emulated bus action or tick is added.

System supplies samples for every write completion, including ordinary instruction stores. Ordinary write handlers ignore them; entry high/low writes consume the relevant field. Read and idle completions require no sampling. This policy lets System build responses from CycleAction without inspecting private CPU phases or expanding the request API. Sampling two cheap, pure registers per write is a small deliberate redundancy; it consumes no extra emulated ticks or CPU Bus actions. Do not add speculative combinations for future devices; revisit the enum if an implemented feature needs samples on a read or idle cycle.

| Pending action/phase | Response | Interrupt sample |
| --- | --- | --- |
| Ordinary read | Read(value) | Not carried |
| Ordinary write | WriteDone { interrupts } | Mandatory; ignored by ordinary handler |
| Ordinary idle or entry wait/jump | IdleDone | Not carried |
| Entry upper stack write | WriteDone { interrupts } | Mandatory; consumes enable_after_bus |
| Entry lower stack write | WriteDone { interrupts } | Mandatory; consumes flags_before_bus |

## Completion-input validation

The public completion wrapper validates the response kind against the current pending action before any register/state/IME mutation. A Read action accepts Read(value), Write accepts WriteDone, and Idle accepts IdleDone. Ordinary handlers may extract read data internally after this check; none should silently accept a write acknowledgment as a missing read or a read byte as an idle completion.

Use a retryable protocol error for a mismatched response, suggested UnexpectedCycleResponse { expected, received } using small public response-kind identifiers. Validate the kind before applying any handler effects. No MissingInterruptSample error is needed: WriteDone cannot be constructed without a sample. Rejected responses leave CPU registers, state, IME and pending transition unchanged, and emit no acknowledgment. Repeated request_cycle calls still return the same action. An already faulted CPU retains and reports its original sticky fault before validating new inputs. This enum prevents invalid payload shapes, but runtime validation is still required because a caller can send Read while CPU expects a write.

This replaces the old public Option<u8> input contract. With a required Read(u8) response, there is no representable successful read lacking a byte; the old MissingReadValue tests should migrate to wrong-response retry tests rather than inventing a zero byte or retaining an ambiguous None response. If an internal helper still uses Option while migrating, preserve its validation-first rule. Removing or retaining the obsolete public error variant is an explicit API migration decision; the recommendation is to remove it once it has no callers. A missing response means the driver has not called completion yet.

System normally constructs the correct response, so these errors diagnose driver misuse. They do not roll back elapsed ticks or a Bus write already performed by a driver; only CPU completion is mutation-free on rejection. System still propagates the failure with its CycleRecord and must not automatically replay a side-effecting write. CPU-level retry tests resubmit the corrected completion response without redoing the Bus action. A successful response is the driver's assertion that it serviced the action; CPU cannot independently prove a Bus write happened or detect every duplicate completion whose kind matches the next action.

Alternatives considered: an exclusive Read/InterruptSample/None enum cannot express write acknowledgment plus samples, while the old read_value: Option<u8> struct cannot distinguish write from idle completion. An outer response-plus-optional-sample struct permits unused read/idle sample combinations and write completion without required samples. The chosen direction puts samples directly in WriteDone, trading a little redundancy on ordinary writes for a stronger payload guarantee. This protocol is a software design choice, not a hardware requirement.

CPU uses enable_after_bus when completing the upper stack write and flags_before_bus when completing the lower stack write. The stored IE sample in the low-write state is combined with that later IF sample. Future devices must obey these documented coarse phases until a finer timing contract supersedes them.

CycleCompletion lets the wrapper return one explicit external effect without a hidden acknowledgment field, drain operation or callback. No error result carries an acknowledgment. Most cycles return acknowledge=None; the source is emitted once during selection.

Because complete_cycle is public, promote the existing small Interrupt source enum and its memory re-export to public visibility if using this proposal. Keep the controller and execution states private; bit_index remains private. This deliberately exposes a meaningful source identifier rather than duplicating vector-to-source conversion in System or leaking private types. Document source enum visibility as an API decision. If the user prefers a fully crate-private effect API instead, settle its public CPU stepping semantics before implementing; do not silently feed zero register samples.

Direct CPU drivers supply the samples and apply acknowledgment themselves. System is the production coordinator. No Bus borrow enters CPU. This input/output change requires test migration and review as a first slice; no other instruction logic changes.

## Cycle and acknowledgment ordering

Let P and S be captured return PC/SP.

| Entry cycle | Action | CPU completion and System effect |
| --- | --- | --- |
| 1 | Idle | Advance; no IF clear |
| 2 | Idle | Prepare upper stack write; no IF clear |
| 3 | Write high(P) to S-1 | Commit SP=S-1; latch sample.enable_after_bus & 1F |
| 4 | Write low(P) to S-2 | Commit SP=S-2; select from latched_IE & sample.flags_before_bus & 1F; return selected-source acknowledgment, or None on cancellation |
| 5 | Idle | Set PC to selected vector, or 0000 if canceled; restore FetchOpcode and report the appropriate entry result |

All address subtraction wraps. Both stack writes occur even on cancellation. Preserve all general registers and F. PC changes only on cycle 5 in our existing trace model; entry duration remains five M-cycles/20 ticks.

System's per-cycle order:
1. Offer initial acceptance (no immediate acknowledgment).
2. Obtain one immutable action.
3. Advance four ticks.
4. For a write action, sample IF before bus access.
5. Service action once.
6. For a write action, sample IE after bus access.
7. Complete CPU using the matching CycleResponse: Read(value), WriteDone { interrupts: sample }, or IdleDone.
8. If completion succeeds and requests acknowledgment, clear only that source through Bus.
9. Publish trace with the plain public outcome and CycleRecord.

A lower stack write to IF is a normal mapped write; selection uses its pre-write sample, then acknowledgment clears the selected source from the post-write IF value. This IF-write collision ordering is explicitly a model convention informed by SameBoy, not established by ie_push alone. Do not claim physical collision accuracy. A high stack write to IF is already visible to the subsequent lower-cycle IF sample. A lower IE write cannot change the previously latched IE.

Resolve and acknowledge inside the same machine-cycle call that completes cycle 4. Do not defer that work to a later step call: pausing must not change which samples were used, or cause acknowledgment to run twice.

## Outcomes and trace design

- Cycles 1-3 return InProgress with acknowledge=None.
- Cycle 4 returns a new nonterminal outcome, suggested InterruptSelected { vector }, or InterruptSelectionCanceled; selected completion also carries acknowledge=Some(source), canceled completion carries None.
- Cycle 5 returns existing InterruptEntryComplete { vector } for successful selection or new InterruptEntryCanceled for cancellation. Keep a private selected/canceled payload so a finish-instruction transition is impossible here.
- The public completion wrapper applies state and derives outcome/effect centrally. This will require a private transition expressing continuation with selection/acknowledgment; do not reintroduce handler-owned self.state assignments.
- CycleTrace still carries CycleRecord and a plain CycleOutcome. It can now show the selection event on cycle 4; acknowledgment is applied before that trace is returned.
- step_instruction continues through progress/selection outcomes and stops on cycle-5 completion. Add InstructionStep::InterruptCanceled { mcycles, ticks }; do not report cancellation as instruction completion or a successful interrupt to a real source.
- Resume counts remain per invocation. A pause after three cycles then resume reports two cycles/eight ticks. Excess budget stops at successful or canceled entry completion and never executes the destination opcode.
- Cancellation is not CpuError or a sticky fault. Fetch from 0000 on the following call, with IME disabled and SP decremented twice. Delayed EI does not advance on any entry event.

## Required synthetic cases

| Setup at acceptance | Required result |
| --- | --- |
| P=0200, S=0000, IF=04, IE=04 | High push writes IE=02; canceled, final PC=0000, IF low bits=04, SP=FFFE, IME disabled |
| After cancellation set Joypad IF/IE | Ordinary instruction continues; no new entry while IME disabled |
| P=0235, S=0001, IF=08, IE=08 | Low push writes IE=35; Serial still selected, final PC=0058, selected IF cleared, SP=FFFF |
| P=0200, S=0000, IF=03, IE=03 | High push writes IE=02; LCD STAT selected, final PC=0048, VBlank remains pending |

These parameters reproduce the register-write effects of Mooneye's cases without copying its ROM. Test both SP wrapping writes independently, destination next-opcode behavior and exact traces.

## Additional acceptance criteria

- All initial low-bit eligibility pairs and upper-bit noise still gate acceptance correctly. No source is acknowledged on acceptance or either wait.
- Sweep latched IE versus later IF combinations independently. Select lowest eligible bit; zero means cancellation. Check all vectors and every general-register/F pattern as appropriate.
- Change IE before/after high-write completion and IF before/after low-write completion: prove the two latch boundaries, without resampling on pause/resume. Pending requests arising before the late IF sample may redirect priority; those after it cannot redirect this entry.
- Same-source IF coalesces before acknowledgment. A new request after cycle-4 acknowledgment survives unless a subsequent ordinary write changes IF. Do not claim counts of repeated edges from a single-bit latch.
- Inject low-cycle IF/IE writes and high-cycle IF/IE writes; use explicit expected-memory oracles including write masking and the subsequent single-source clear.
- Every zero/partial/exact/excess budget, exact five-action sequence, intermediate PC/SP and final outcomes; cancellation still has both writes and five cycles. Compare uninterrupted and paused execution with identical events.
- Maintain CPU capture, stack wrap, ROM/WRAM/echo/HRAM boundary and full-memory preservation tests. Replace #39 frozen-source/early-clear expectations; retain other regression checks.
- Preserve EI;NOP, EI;DI, EI;EI, CB indivisibility, retry/fault guarantees, public traces and failed-cycle records. Exhaust response-kind mismatches for representative read/write/idle and every entry phase. Corrected completion retries preserve captured values and do not repeat Bus writes. Rejected responses never acknowledge requests or mature delayed EI. Include correct and wrong responses while already faulted. Migrate obsolete MissingReadValue expectations to the explicit response protocol. Verify ordinary writes work with differing sample values and no hidden dependency on those samples; required write samples are enforced by the variant shape rather than a missing-sample runtime test.
- Full tests, formatting, strict Clippy and whitespace checks. No dependency/build changes or ROM binary fixtures.

## Review slices and limits

1. Review/accept the CycleResponse enum with mandatory write samples and completion-effect interface, validation errors and public Interrupt visibility. Migrate existing tests without implementing the later selection behavior. User owns application/API implementation; assistant owns test migration and protocol cases.
2. Move selection to cycles 3/4, add cancellation and explicit outcomes; assistant adds CPU tests.
3. Wire ordered samples/effects in System; assistant updates early-ack tests and adds the synthetic cases/full integration evidence.
The interfaces are now approved. Begin with slice 1; verify the existing instruction behavior before moving to later selection and System integration. Slice 1 must preserve #39's current acknowledgment behavior until slices 2/3 deliberately replace it; when adapting the completion return, do not acknowledge both on initial acceptance and on completion.

RETI, HALT/wake/HALT bug, STOP, peripheral edge timing and external ROM harness setup remain separate. Exact IF collision timing remains an explicitly tracked limitation after this task; do not close broader hardware-accuracy work on the strength of the IE synthetic cases.

## References checked 2026-11-17

- [Mooneye hardware test ie_push](https://raw.githubusercontent.com/Gekkio/mooneye-test-suite/main/acceptance/interrupts/ie_push.s): primary hardware-tested IE behavior; reference only, not imported or run.
- [SameBoy SM83 implementation](https://raw.githubusercontent.com/LIJI32/SameBoy/master/Core/sm83_cpu.c): GB_cpu_run entry branch and cycle_write_if, used as implementation evidence, including its own timing uncertainty.
- [SonoSooS CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595): supports late priority changes during entry; its fetch-overlap trace is not our coarse trace convention.

Baseline: local HEAD a8726ba, clean before specification. Prior behavior evidence is 349 passing tests for #39; no tests rerun for this documentation-only proposal. No implementation/test code changed, no commit or push requested for this planning turn.

## Response-protocol slice review - 2026-11-17

User implemented public cycle types in cpu/cycles.rs, CycleResponse validation and read extraction, plus System write sampling. Late source selection, cancellation and CycleCompletion effects are not implemented in this slice; #39 entry behavior remains. Assistant migrated CPU/public API tests to explicit responses, preserving requested-address assertions separately from error payloads, and added three protocol tests covering every base and CB encoding under all IME states, wrong response kinds in instruction/entry phases, sticky faults and ignored sample values for ordinary writes. The shared test-only response builder preserves existing instruction-data oracles; None for a pending read now supplies an intentionally incorrect IdleDone response.

Focused protocol tests pass. Library regression run: 249 pass, three relative-jump tests fail. Review found ReadDisplacement lost its signed i8 conversion when replacing the optional-read match; FE is now interpreted as +254 instead of -2. The user was asked to restore that conversion and remove the now-unused MissingReadValue variant. Tests do not require that obsolete variant. Formatting, strict Clippy and whitespace checks pass at this intermediate checkpoint. Full all-target validation remains incomplete pending the application fix; no external ROM suite ran. No commit or push made.

## Response-protocol validation complete - 2026-11-17

The user restored the JR signed conversion and removed MissingReadValue. All 352 tests pass (252 unit, 100 integration), including the six focused JR tests and three new protocol tests. Formatting, strict Clippy and whitespace checks pass. The earlier three failures above are resolved. No external ROM suite was run; changes remain uncommitted.

CycleAction/CycleOutcome plus the new CycleResponse/InterruptSample/CycleKind/error types are in cpu/cycles.rs and re-exported by CPU. System provides correct response kinds and samples IF before writes/IE after them. CPU validates kind before mutation and read handlers extract typed bytes. No correctness findings remain in this response-protocol slice. Samples are currently ignored by entry as well as ordinary writes; end-to-end late-selection sampling effects will be verified when entry consumes them.

Next implement the approved CycleCompletion acknowledgment effect and its private transition representation while keeping #39 entry behavior stable, then move source selection/IF acknowledgment to the specified later stages and add cancellation. Avoid clearing IF at both acceptance and later selection. #40 stays open.
