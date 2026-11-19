# Game Boy emulator project plan
## Current checkpoint - decoder validity refactor verified, 2026-11-19

User completed #55's CB From and base TryFrom together, with IllegalOpcode leaf reporting and no Unknown/UnsupportedCbOpcode. The fetch-ordering fix preserves original address capture, validated fetch PC/suppression effects and wrapper-owned sticky faults. All 441 tests pass (316 unit, 125 integration), explicitly including five external fixtures; formatting, strict Clippy and whitespace checks pass. Assistant migrated decoder/EI tests, strengthened exact illegal-byte classification and added leaf/fault acceptance. [Evidence](tasks/CPU_READABILITY.md#decoder-validity-checkpoint-verified---2026-11-19). No assistant application changes or dependencies. Next is [#56](https://github.com/Lorithan/gameboy-emu/issues/56), infallible private completion helpers/callers while response validation stays fallible. Register ownership, fetch-state suppression, immediate-word consolidation and #54 ALU extraction remain separate; M2 stays complete. Earlier ordering failures below are historical.

## Current review - decoder conversions need fetch-effect ordering fix, 2026-11-19

User implemented infallible CB From and base TryFrom with IllegalOpcode, removing Unknown and the unreachable CB error type/branch. Assistant migrated decoder assertions and EI classification tests while preserving instruction/result oracles; exact eleven-illegal-byte and leaf-error checks pass. All 17 CB-focused tests and six decoder tests pass; strict Clippy passes. New fault acceptance fails: read_opcode returns Faulted before advancing the fetched PC/consuming suppression, changing the existing invalid-fetch behavior. Next user fix: capture the original opcode address, apply the existing successful-fetch increment/suppression effects, then handle the decode result into Faulted or instruction dispatch. Keep wrapper fault latching and delayed-EI behavior. No full suite rerun or passing checkpoint claimed; assistant changed tests/docs only. Last verified checkpoint remains 56f99bd with 439 passing tests.

## Current checkpoint - cartridge error reporting verified, 2026-11-19

User completed CartridgeError Display/Error. Validation variants remain leaves with expected/actual context; FileIO displays its path and exposes the original io::Error. Two new unit tests and one public-loader test verify diagnostic fields, Unicode path context, typed I/O causes and Box<dyn Error> interoperability, including a real missing-file error. All 439 tests pass (314 unit, 125 integration) with all five external fixtures explicitly included; formatting, strict Clippy and whitespace checks pass. Standard CPU/System/cartridge error reporting is complete; no loader/execution/timing behavior or dependency change. Next accepted cleanup is [#55](https://github.com/Lorithan/gameboy-emu/issues/55), infallible CB decoding and removal of UnsupportedCbOpcode, with mappings and protocol behavior preserved. Base decoding, fetch-state suppression and #54 ALU extraction stay separate; M2 stays complete.

## Current checkpoint - System error reporting verified, 2026-11-19

User added Display/Error for SystemError and Display for CycleAction. Completion messages report tick_after and read/write/idle details; both SystemError variants expose the wrapped CpuError without duplicating its text. Three assistant-owned tests cover typed nested causes, leaf termination and completion tick/action metadata. All 436 tests pass (312 unit, 124 integration) with all five external fixtures explicitly included; formatting, strict Clippy and whitespace checks pass. No assistant application changes, execution/timing changes or dependencies added. Next user slice: CartridgeError Display/Error, with path context and the original io::Error source for FileIO, and expected/actual details for leaf validation failures. CPU structural/decoder cleanup remains queued; M2 stays complete.

## Current checkpoint - CPU error reporting verified, 2026-11-19

User completed cpu/error.rs extraction, stable root exports, readable CycleKind/opcode/fault messages and standard Error source chains. CpuError supplies concise outer context; typed fault/protocol causes retain their details, and NoCycleRequested is a leaf. All 433 tests pass (312 unit, 121 integration) with all five external fixtures explicitly included; formatting, strict Clippy and whitespace checks pass. Three assistant-owned error tests verify standard-trait use, public imports, essential diagnostic fields and typed causes. No assistant application changes or dependencies added. Next user slice: Display/Error for SystemError, exposing CpuError from both variants and preserving completion CycleRecord context; CartridgeError follows separately. #54 ALU extraction and accepted decoder/register cleanup remain queued; positional set_flags is discarded. Earlier incomplete-export/trait reviews below are historical; M2 stays complete.

## Current review - CPU error traits and exports verified, 2026-11-18

User restored public root exports and implemented CpuError::source for wrapped fault/protocol errors, with NoCycleRequested as a leaf. Three new CPU error acceptance tests and two existing public cycle-outcome tests pass; strict Clippy and whitespace checks pass. Earlier export/missing-trait findings below are resolved. No full regression rerun or completed checkpoint claimed yet. Remaining feedback is message wording: outer Display currently embeds a cause also exposed through source, so chained reports can repeat it. Prefer concise outer context, detail in leaf errors and an action-specific NoCycleRequested message. No application edits by assistant.

## Current review - CPU error extraction and standard traits, 2026-11-18

User accepted standard Display/Error cleanup and rejected positional set_flags booleans. Current CPU error extraction has two incomplete pieces: root cpu:: error exports became private, breaking existing public callers, and CpuError has Display but no Error/source implementation. Library check passes with an unused CpuError import warning; the existing cycle_outcomes integration target and three new CPU error acceptance tests cannot compile through the old public paths. Assistant left application fixes to the user. Next: restore explicit public root exports, implement CpuError sources for wrapped faults/protocol errors (None for NoCycleRequested), and remove the stale cycles import. Then assistant verifies traits/diagnostic fields and regressions. No passing error-cleanup checkpoint claimed; #54 ALU extraction stays queued and M2 remains complete.

## Current user focus - CPU readability cleanup, 2026-11-18

User chose CPU structure/readability cleanup before M3. [#54](https://github.com/Lorithan/gameboy-emu/issues/54) proposes a first mechanical extraction of the eight existing arithmetic/bit helpers into private cpu/alu.rs, retaining impl CPU and behavior. [Boundaries and acceptance](tasks/CPU_READABILITY.md) distinguish that small slice from later opcode-dispatch/completion extraction. User writes application changes; assistant owns verification/test wiring. M2 stays complete; #53 timer contract and #52 DAA confidence work remain queued. No application change or emulator test run for this review; last executable verification remains f0283fa with 430 passing tests, formatting/strict Clippy pass.

## Current milestone decision - M2 functional CPU complete, 2026-11-18

[#51 completion review](M2_COMPLETION_REVIEW.md) maps every original M2 criterion to source/test evidence: 500/500 legal encodings, focused edge cases, deterministic stepping/traces and five actual pinned external ROM passes. M2 is complete for the declared functional DMG M-cycle scope; full physical accuracy is not claimed. No code/tests changed or rerun for this review; last full verification remains f0283fa with 430 passing tests including fixtures, formatting and strict Clippy passing. Next implementation planning is M3 [#53](https://github.com/Lorithan/gameboy-emu/issues/53), the programmable timer contract before user code. [#52](https://github.com/Lorithan/gameboy-emu/issues/52) is a separate non-blocking Mooneye DAA confidence check, selected after comparing broader vector/ROM suites. #44 physical STOP limits, mapper/PPU/DMA prerequisites and authentic boot remain explicit follow-ups. Earlier open-M2 notes below are historical.

## Current checkpoint - four additional external CPU ROMs pass, 2026-11-18

#50 is complete: pinned Mooneye if_ie_registers, interrupts/ie_push, ei_sequence and div_timing all pass with packet/register signatures. [Source/prerequisite audit and measured results](tasks/MOONEYE_CPU_SUBSET.md) document software IRQs, the bounded PPU-helper timeout, unchanged budgets and fixture hashes. All 430 tests pass (312 unit, 118 integration) when explicitly including all five external fixtures; formatting, strict Clippy and whitespace checks pass. No emulator fix, application/configuration/dependency change or committed ROM binary. M2 stays open for [#51](https://github.com/Lorithan/gameboy-emu/issues/51): review milestone exit criteria against evidence and select missing independent instruction-result coverage, distinguishing essential work from deferred hardware accuracy. General arithmetic/DAA/CB results are still primarily covered by local tests. #44 physical STOP limits and later device milestones remain separate.

## Current checkpoint - first external CPU ROM passes, 2026-11-18

#49 is complete: pinned official Mooneye reg_f passes with matching output/register signature in 3,946 M-cycles / 15,784 ticks. [Evidence and reproduction](tasks/MOONEYE_REG_F.md) include verified archive/ROM/symbol/license hashes, build/font provenance and the explicit checksum-gated test command. All 426 tests pass (312 unit, 114 integration) when explicitly including the external fixture; formatting, strict Clippy and whitespace checks pass. Ordinary default-ignore is not counted as external evidence. No application/configuration/dependency changes or committed ROM binaries. M2 remains open: this checks F masking through stack/register instructions only. Next is [#50](https://github.com/Lorithan/gameboy-emu/issues/50): audit a small next CPU ROM subset for actual DMG/startup/device/reporting dependencies, run supported fixtures and isolate any user-owned implementation fixes. #44 retains physical STOP limits; TIMA/PPU/frontend remain separate. Earlier no-external-ROM checkpoints below are historical.

## Current checkpoint - CPU ROM runner synthetic acceptance verified, 2026-11-18

#45's contract is selected and assistant-owned test-only runner is implemented. All 11 runner tests pass; fresh full regression passes 425 tests (312 unit, 113 integration), formatting, strict Clippy and whitespace checks. [Evidence](tasks/CPU_ROM_VALIDATION.md#synthetic-runner-checkpoint---2026-11-18) distinguishes result/false-positive/budget/error/file-setup tests from actual external validation. No application/configuration/dependency changes or external ROM fixtures. #45's specification is complete; [#49](https://github.com/Lorithan/gameboy-emu/issues/49), assigned to M2, is next: prepare pinned Mooneye reg_f with artifact/build/component provenance and digest, add explicit fixture invocation, and record the first result. Assistant owns preparation/harness; user implements fixes demonstrated by failures. No external ROM ran and M2's broader validation gate remains open. #44 keeps physical STOP timing limits; TIMA/PPU/SDL2 remain separate.

## Current checkpoint - selected-input STOP wake verified, 2026-11-18

#48's deterministic scope is complete: System updates Bus/IF then offers the selected-line level to guarded CPU wake. Wake preserves PC/registers/IME, consumes no cycles and retains input pulses; subsequent execution performs existing arbitration. All 414 tests pass (312 unit, 102 integration), including 22 focused STOP-related tests and 13 focused joypad unit tests; formatting, strict Clippy and whitespace checks pass. [Evidence](tasks/STOP_WAKE.md#stop-wake-completion-verified---2026-11-18). Assistant migrated only #47's Stopped-state expectation. #44 retains oscillator/edge/glitch accuracy limits; no external ROM ran and M2's ROM gate remains open. Next planning task: revisit deferred #45's bounded CPU ROM validation and artifact provenance before fixtures. SDL2 and TIMA/audio remain separate.

## Current checkpoint - conditional STOP entry verified, 2026-11-18

User fixed the HALT-like padding increment and completed #48's conditional entry. All 409 tests pass (308 unit, 101 integration), including 18 focused STOP-related tests; formatting, strict Clippy and whitespace checks pass. [Evidence](tasks/STOP_WAKE.md#conditional-entry-completion-verified---2026-11-18) covers the four branches, masked pending interrupts, typed read sampling, PC wrap/fetch suppression, delayed EI and actual-STOP-only divider reset. Prior missing-padding reviews below are historical. Next user slice: guarded CPU wake_from_stop_if(bool), called from System::set_button after Bus/IF updates using the current selected-line level. Wake costs zero cycles, preserves PC/registers/IME and lets the next execution step perform ordinary interrupt arbitration. Assistant owns pulse/purity/wake acceptance and explicit migration of the Stopped case in #47 input tests. #48/#44 stay open; oscillator/glitch accuracy and M2's external-ROM gate remain open. No external ROM ran.

## Current review - STOP HALT-like branch needs padding skip, 2026-11-18

User implemented completion behavior; review found StopBehavior::Halt sets Halted but omits its wrapping padding increment. Assistant migrated old unconditional-padding CPU oracles and added four System branch tests plus expanded CPU entry coverage. Focused STOP-related tests: 16 pass, two fail on the same missing increment (PC P+1 instead of P+2). Formatting, strict Clippy and whitespace checks pass; no full suite rerun while this known failure remains. Next user action: increment PC in the Halt completion arm before entering Halted. Assistant then verifies the full entry matrix and regressions. [Evidence](tasks/STOP_WAKE.md#conditional-entry-acceptance-review---2026-11-18). No assistant application fixes or external ROM execution; #48/#44 remain open and last complete passing checkpoint is 81261bd.

## Current review - STOP behavior selection connected, 2026-11-18

User passes the mandatory wake sample into opcode handling and selects the agreed StopBehavior from selected-line level and masked pending interrupts. Decode no longer skips padding. The completion wrapper still ignores the behavior and always enters Stopped, so this is a partial slice. Next: consume StopFinished(behavior), apply conditional wrapping padding and the specified state/outcome. Focused existing CPU STOP tests: one passes, two fail during this transition; no full regression rerun. One old suppression/pending oracle also assumes unconditional padding and must be migrated to the new contract during assistant acceptance work. No application code changed by the assistant. See [review](tasks/STOP_WAKE.md#behavior-selection-review---2026-11-18). Last complete verified checkpoint remains 81261bd with 405 passing tests; no external ROM ran.

## Current checkpoint - STOP input sampling complete, 2026-11-18

User implemented #48's pure level queries and mandatory Read wake field/accessor with System sampling. Assistant migrated test constructors and added four tests across the query and response slices, extending ordinary-read regression to both levels. Fresh full verification passes all 405 tests (304 unit, 101 integration), formatting, strict Clippy and whitespace checks, without allowances. [Evidence and limits](tasks/STOP_WAKE.md#read-sample-slice-review---2026-11-18). CPU still ignores the new sample; conditional entry and host-input wake are not implemented. Next: private StopBehavior carried by StopFinished, opcode-level sample consumption and the documented four-branch entry table; then wake delivery. #48/#44 remain open, no external ROM ran, and M2's ROM gate remains open.

## Current slice review - STOP wake level queries, 2026-11-18

Fresh regression passes all 404 tests (303 unit, 101 integration), formatting and whitespace checks. No external ROM ran; finish production sampling to resolve unused-query Clippy errors before the next checkpoint.

User implemented the pure Joypad/Bus queries for #48. Three focused acceptance tests pass, covering every held mask/selection, IF/IE independence, memory/divider-phase preservation and instance isolation. [Evidence](tasks/STOP_WAKE.md#level-query-slice-review---2026-11-18). Strict Clippy flags the two methods until production sampling is connected; assistant added no allowances or application changes. Next user slice: mandatory wake level in CycleResponse::Read, accessor matching the existing response-kind error contract, and System sampling after reads. Assistant owns test-constructor migration. STOP entry/wake behavior is still unchanged.

## Current next task - selected-line STOP entry and wake, 2026-11-18

[#48](https://github.com/Lorithan/gameboy-emu/issues/48), under #44, is proposed in [STOP_WAKE.md](tasks/STOP_WAKE.md). Start with pure Joypad and Bus stop_wake_active queries. Subsequent slices supply a mandatory read-response level sample, select the four DMG entry branches, then deliver zero-cycle host-input wake through System to a guarded CPU transition. A selected press/release between execution calls must retain wake; IF and eligible pending interrupts remain separate signals. Review the proposal before dependent implementation. User owns application code; assistant owns tests/docs/review. Baseline remains 03aff43 and its previous 401-test verification; no emulator tests rerun for this documentation-only contract. Hardware oscillator/glitch limits stay visible under #44; #45 and M2's external-ROM gate remain open.

## Current checkpoint - headless joypad complete, 2026-11-18

User implemented Joypad, FF00/IF event handling and public System::set_button for [#47](https://github.com/Lorithan/gameboy-emu/issues/47). All 401 tests pass (300 unit, 101 integration), including 11 focused joypad tests; formatting, strict Clippy and whitespace checks pass. Public input preserves CPU state and clocks across execution states; HALT wake/service occurs on the next cycle through existing gates. FF00 data/full-memory oracles are migrated and obsolete allowances removed. [Completion evidence](tasks/JOYPAD.md#headless-joypad-completion-verified---2026-11-18). #47 is complete, #44 stays open. Next planning task: specify selected-line STOP wake delivery and conditional entry, including input pulses between calls, held inputs, pending interrupts and CPU sampling, before implementation. No external ROM ran. SDL2, TIMA/audio and #45 remain separate; M2's ROM gate remains open.

## Current checkpoint - divider foundation complete, 2026-11-18

User implemented Timer, FF04 routing and System advancement/STOP-reset integration for [#46](https://github.com/Lorithan/gameboy-emu/issues/46). All 390 tests pass (290 unit, 100 integration), including 11 focused divider tests; formatting, strict Clippy and whitespace checks pass. Assistant migrated time-dependent DIV memory oracles while preserving all other memory expectations. [Evidence and limits](tasks/DIVIDER.md#divider-completion-verified---2026-11-18) cover startup/read phase, reset/write ordering, HALT/entry cycles, stopped polls and errors. #46 is complete; #44 remains open. Next planning task: specify headless JOYP state/selection/startup, interrupt transitions, host-input delivery while stopped and wake ownership before user implementation. ABC8 remains a model-phase inference. No external ROM ran; TIMA/audio are separate, #45 stays deferred, M2's ROM gate remains open and SDL2 stays deferred to M4.

## Current user focus - STOP device prerequisites, 2026-11-18

User clarified that the requested next work is STOP divider reset and joypad wake, [#44](https://github.com/Lorithan/gameboy-emu/issues/44). The external-ROM proposal #45 remains available for later; no fixture or runner was introduced. Start by specifying divider ownership, tick advancement, FF04 reset/read behavior and startup, then headless JOYP selection/input/interrupt behavior, then ordinary STOP reset/pause/wake integration. See [roadmap](tasks/STOP.md#device-prerequisite-roadmap---2026-11-18). These device contracts require review before user implementation; the roadmap is not a claim of complete conditional STOP support. SDL2 remains deferred to M4. The last emulator verification remains 379 passing tests at 6df35c5; no tests rerun for this documentation-only clarification. M2's ROM-validation gate remains open.

## Current next design review - external CPU ROM validation, 2026-11-18

[#45](https://github.com/Lorithan/gameboy-emu/issues/45) proposes a first pinned Mooneye bits/reg_f fixture and test-only bounded runner in [CPU_ROM_VALIDATION.md](tasks/CPU_ROM_VALIDATION.md). Source audit suggests current fallback I/O can support its fast report path; this is unexecuted. The proposed observer uses FF01 trace writes plus register signatures, with explicit failure/timeout/stopped outcomes. Assistant owns test infrastructure; user owns any application fixes. Before fixtures, finish artifact/digest/component provenance. No ROM binaries, application changes or dependencies added; no emulator tests rerun for this documentation-only task. Published baseline remains 6df35c5 with 379 passing tests. Keep M2 open; #44 STOP devices remains separate.

## Current checkpoint - core STOP complete, 2026-11-18

[#43 core STOP standby](https://github.com/Lorithan/gameboy-emu/issues/43) is implemented and verified: all 379 tests pass (279 unit, 100 integration), including eight focused STOP tests; formatting, strict Clippy and whitespace checks pass. The user fixed the padding increment using the shared wrapping helper. Entry, no-cycle polling, PC wrap/suppression, EI/DI, interrupt-before-fetch/RETI and memory preservation are covered. See [completion evidence](tasks/STOP.md#core-standby-completion-verified---2026-11-18). Earlier padding/unimplemented-state notes below are historical.

Encoding coverage reaches 244/244 legal non-prefix base plus all 256 CB (500/500) for the declared behavior. This does not establish complete hardware accuracy: STOP divider/joypad/conditional behavior remains [#44](https://github.com/Lorithan/gameboy-emu/issues/44), after M3/M4 prerequisites. No external ROM suite ran. Keep M2 open for its ROM-validation gate. Next planning task: select a pinned/licensed CPU test-ROM subset and specify bounded headless execution, result reporting and device prerequisites before introducing fixtures or implementing a harness. User owns application implementation; assistant owns tests/docs/review. Frontend and unrelated refactors remain separate.

## Current review - STOP padding increment required, 2026-11-18

User implemented STOP decode/state/entry event and shared delayed-EI completion for #43. Review found the STOP arm omits the padding skip, leaving PC at P+1 instead of the agreed ordinary P+2 (also wrong with HALT suppression and wrapping). Assistant added eight acceptance tests and migrated stale decoder/protocol/fault/EI expectations. Focused STOP results: two pass, six fail on that defect. All 100 integration tests passed in the full run; one stale EI oracle from that run is fixed and passes its focused check. See [exact evidence](tasks/STOP.md#stop-acceptance-review---2026-11-18). Library check, formatting, strict Clippy and whitespace checks pass. Next user action: add the wrapping padding increment in STOP handling; assistant then verifies acceptance. Keep #43 open and coverage at 499/500 until verified. #44 device work and M2 ROM gates remain separate; no external ROM ran.

## Current checkpoint - STOP stepping protocol migrated, 2026-11-18

User implemented CycleRequest, CPU completion validation, MachineStep and bounded Stopped results for [#43](https://github.com/Lorithan/gameboy-emu/issues/43). Assistant migrated CPU/System request and trace tests; fresh verification passes all 371 tests (271 unit, 100 integration), formatting, strict Clippy and whitespace checks. See [evidence](tasks/STOP.md#machinestep-migration-verified---2026-11-18). No stopped execution state exists yet, so no-cycle polling and NoCycleRequested rejection remain structurally reviewed but unexercised. Next: user implements STOP decode/state/entry event and shared EI bookkeeping; assistant owns standby tests. The accepted stepping direction supersedes proposal status below. Coverage remains 499/500; no external ROM ran. Device follow-up #44 and M2 ROM gates remain open.

## Current next design review - STOP, 2026-11-18

[#43 core STOP standby](https://github.com/Lorithan/gameboy-emu/issues/43) is proposed in [STOP.md](tasks/STOP.md). Settle a CPU request result and System step result that distinguish real cycles from a stopped clock before decoding STOP. Proposed StopEntered represents instruction completion plus standby entry; shared delayed-EI bookkeeping includes it. Bounded callers return Stopped without manufacturing idle cycles. First implementation slice is the protocol migration with existing execution preserved; the user owns application changes and the assistant owns test migration/review.

This is a proposal for ordinary core standby, not approved or implemented device-complete STOP. [#44](https://github.com/Lorithan/gameboy-emu/issues/44) tracks DIV reset/pause, selected joypad-line wake and hardware conditional/wake behavior after M3/M4 prerequisites. No force-resume API or fake peripherals in #43. Baseline remains b9c765e and the previous 371-test verification; no emulator tests rerun for this documentation-only work. Coverage stays 499/500; M2 remains open for instruction and ROM evidence. This supersedes older next-task notes below.

## Current checkpoint - HALT complete, 2026-11-18

#42 HALT/wake/HALT-bug behavior is implemented and verified: all 371 tests pass (271 unit, 100 integration), ten focused HALT tests pass, formatting/strict Clippy/whitespace checks pass. System wake precedes acceptance; HALT completion matures delayed EI once while waiting idles do not. One-shot opcode suppression, wrapping, rejected acceptance, CB/RST/repeated-HALT and EI;HALT buffered-handler return addresses are covered. See [evidence](tasks/HALT.md). No external ROM suite ran and no physical device-edge accuracy is claimed.

Coverage is 243/244 legal non-prefix base plus all 256 CB (499/500). Next task: specify STOP and its clock/wake/device prerequisites before implementation. M2 remains open for the remaining instruction and hardware ROM validation evidence. This checkpoint supersedes older incomplete-HALT statements below. Frontend, peripherals and unrelated refactors remain separately scoped.
## Current checkpoint - HALT read-response slice, 2026-11-18

#42's read-response migration is verified: Read now carries value plus pending_interrupts, sampled by System after the read. All 362 tests pass (262 unit, 100 integration), four focused protocol tests pass, formatting/strict Clippy/whitespace checks pass. No external ROM suite ran. Existing behavior is unchanged and HALT is still unsupported. See [slice evidence](tasks/HALT.md).

Next: implement Halted idles and HALT-specific instruction completion with shared EI bookkeeping, then wake polling and fetch-suppression/EI interactions per #42. User owns application implementation; assistant owns tests/docs/review. Coverage stays 242/244 base and 256/256 CB. #42 stays open.

## Current next design review - HALT, 2026-11-18

[#42 HALT](https://github.com/Lorithan/gameboy-emu/issues/42) is specified in [HALT.md](tasks/HALT.md). Before implementation, review mandatory pending-mask data on read responses, a Halted state and instruction-completion transition, wake polling before ordinary interrupt acceptance, and one-shot opcode increment suppression. The EI;HALT return-address translation into our non-prefetch M-cycle model is explicit and requires review. These are proposals, not yet approved implementation details. Baseline a40668f retains 361-test verification; no new emulator tests were run for this documentation-only task.

Implement in slices: read-response migration first, then ordinary HALT/wake, then the bug and EI/RST/CB interactions. User owns application implementation; assistant owns tests/docs/review. Do not claim HALT complete until the interaction cases pass. Coverage remains 242/244 base plus all CB; STOP is separately scoped. Physical edge/device timing and external ROM validation remain unverified.
## Current checkpoint - RETI complete, 2026-11-18

#41 RETI is implemented and verified: all 361 tests pass (261 unit, 100 integration), all six focused RETI tests pass, formatting/strict Clippy/whitespace checks pass. Typed return mode shares stack reads/final return handling with RET, enabling IME at final RETI completion. Synthetic chained interrupt handlers run before the returned-to opcode as required. See [acceptance evidence](tasks/RETI.md). No application correctness fixes were needed during review. No external ROM suite ran.

Coverage is 242/244 legal non-prefix base plus all 256 CB (498/500). Next: specify HALT, wake-up with/without IME and the HALT bug before user implementation. STOP and peripheral/physical-edge timing remain separate. This checkpoint supersedes older pending-RETI statements below; keep M2 open.

## Current next task - RETI, 2026-11-18

[#41 RETI](https://github.com/Lorithan/gameboy-emu/issues/41) is specified in [RETI.md](tasks/RETI.md). Implement D9 as four cycles (opcode read, low stack read, high stack read, final idle), restoring PC/SP and enabling IME at final completion with no following-instruction delay. Proposed architecture carries a typed return mode through shared return states and a return-specific final idle; exact private names remain user choices. User implements; assistant owns acceptance tests/review. Baseline is published 11a2eca with the prior 355-test validation. RETI is not implemented yet; coverage remains 241/244 non-prefix base and 256/256 CB.

Begin with the decoded RETI operation and return-mode/final-state representation. Preserve RET/conditional-return and JR/JP timing. Tests will verify immediate pending-interrupt entry before the returned-to opcode, all budget/retry/wrap/capture cases and existing regressions. No external ROM imported/run. HALT/STOP and finer device/IF collision timing remain separate. This supersedes older next-task statements below.
## Current checkpoint - late interrupt selection complete, 2026-11-18

#40 late selection/cancellation is implemented and verified: all 355 tests pass (255 unit, 100 integration), 13 focused entry tests pass, formatting/strict Clippy/whitespace checks pass. CPU latches IE after the upper stack write and resolves against IF sampled before the lower write; System acknowledges only the cycle-4 InterruptSelected event. Cancellation completes both writes/five cycles, lands at 0000, leaves IME disabled and reports a distinct result. The public enum event replaces the proposed CycleCompletion struct. See [acceptance evidence](tasks/LATE_INTERRUPT_SELECTION.md). Synthetic hardware-derived IE cases pass; no external ROM suite ran and no physical edge/IF collision accuracy is claimed.

This supersedes older pending #40 and early-ack statements below. Next task: specify RETI's return cycles and immediate IME enable semantics before user implementation. HALT/STOP, peripherals, frontend and finer IF collision timing remain separate. M2 stays open.

## Current next task - approved late-selection design, 2026-11-17

[#40 late interrupt selection](https://github.com/Lorithan/gameboy-emu/issues/40) now has a [proposed contract](tasks/LATE_INTERRUPT_SELECTION.md). Review CycleResponse::{Read(u8), WriteDone { interrupts: InterruptSample }, IdleDone}, explicit completion/acknowledgment effects, response-kind validation and cancellation outcomes before user implementation. The user approved CycleResponse and the wider sampled-input/completion-effect design. The user approved the revised proposal on 2026-11-17; implementation has not started. Begin with the response/effect protocol migration, preserving instruction behavior before changing entry selection. Every write response carries mandatory samples; ordinary writes ignore them, and read/idle responses carry no samples. System need not inspect private CPU phases. The schedule latches IE after the high stack write and combines it with IF sampled before the low write, then acknowledges after that low write. Hardware-tested IE cases and less-certain IF collision ordering are distinguished explicitly. The implemented #39 early-ack/captured-source behavior remains unchanged until this proposal is approved and implemented. RETI follows as a separate instruction task.
## Current checkpoint - interrupt entry complete, 2026-11-17

The user completed the first-stage #39 interrupt-entry implementation and removed obsolete staged dead-code allowances. All 349 tests pass (249 unit, 100 integration), with nine focused entry tests also passing after all-flag coverage. Formatting, strict Clippy and whitespace checks pass after allowance cleanup. No external ROM suite ran. This checkpoint supersedes earlier next-task/unimplemented notes below; see [entry evidence](tasks/INTERRUPT_ENTRY.md). Source/vector capture and acknowledgment at acceptance remain the documented approximation.

The user requested a local commit. Next planning work is #40: research and specify hardware late source selection, IE stack-write cancellation, acknowledgment and cancellation outcomes before implementation. RETI, HALT/STOP, peripherals, frontend and required-read extraction remain separately scoped. Keep M2 open.

## Current next task - 2026-11-17

User selected the explicit acceptance/CPU-owned entry design. [#39 - Resumable interrupt entry](https://github.com/Lorithan/gameboy-emu/issues/39) is specified in [INTERRUPT_ENTRY.md](tasks/INTERRUPT_ENTRY.md), under M2; implementation has not started. Begin with explicit CPU completion, trace and bounded-step outcomes while preserving existing instruction behavior, then add CPU entry and System coordination in reviewed slices. User owns application changes; assistant owns test migration and acceptance checks. Baseline is d6b0bb7 with the prior 339-test validation; no new test run was needed for this documentation-only change.

The first-stage model captures source/vector and acknowledges at acceptance. This is an explicit approximation: hardware-tested Mooneye ie_push shows source changes/cancellation when an upper-byte stack write modifies IE. [#40](https://github.com/Lorithan/gameboy-emu/issues/40) tracks late selection/acknowledgment research and implementation before claiming that behavior or ie_push conformance. No physical sampling-edge accuracy is claimed. RETI, HALT, STOP, peripherals and frontend remain separate work.

## Current checkpoint - 2026-11-16

This checkpoint supersedes historical next-task and publication notes below. Local and live GitHub main match 6a634f8; all four handoff commits are present. Fresh verification passes 334 tests (236 unit, 98 integration), formatting and strict Clippy. No external ROM suite was run. DI/EI issue #37 is closed; coverage is 241/244 legal non-prefix base encodings and all 256 CB encodings.

Current next task: [#38 - Interrupt controller and IF/IE Bus mapping](https://github.com/Lorithan/gameboy-emu/issues/38), under M2, specified in [INTERRUPT_REGISTERS.md](tasks/INTERRUPT_REGISTERS.md). Implementation has not started. Agreed ownership: Bus owns the controller, CPU owns IME, System connects them without CPU borrowing Bus. Proposed interface names/types remain open to user review. DMG post-boot IF reads E1 and IE reads 00; IF unused bits read one, while IE retains all eight written bits and only its low five participate in eligibility.

User implements the controller/Bus task; assistant owns acceptance tests and migration of stale unmapped-IF/IE expectations. Interrupt sampling/entry, acknowledgment timing, RETI, HALT and STOP require subsequent contracts. No peripherals, broad refactors, ROM fixtures or frontend setup in this task.
## DI/EI completion checkpoint - 2026-11-14

This checkpoint supersedes earlier next-task notes. DI/EI (#37) are complete locally: private cycle-completion helper, CPU-owned IME state and delayed enable pass review. The last full run passed all 334 tests (236 unit, 98 integration); after the visibility-only fix, all eight DI/EI tests, formatting and strict Clippy pass. See [DI_EI.md](tasks/DI_EI.md) for evidence. Changes remain uncommitted; publication not checked. Coverage: 241/244 base plus 256/256 CB; RETI, HALT and STOP remain. Next: specify IF/IE and the CPU/System interrupt interface before interrupt dispatch, RETI and HALT implementation. STOP remains separate. No external ROM suite was run.
## Next task - DI/EI, 2026-11-14

CB work and integration validation are committed locally as 6494b7a; publication has not been checked. The verified baseline is 326 passing tests (228 unit, 98 integration), formatting and strict Clippy pass. Next: [#37 DI/EI and instruction-boundary IME lifecycle](https://github.com/Lorithan/gameboy-emu/issues/37), specified in [DI_EI.md](tasks/DI_EI.md). CPU owns IME and pending enable, initially disabled. The proposed design centralizes delayed-enable advancement at successful instruction completion while preserving retry, fault and bounded stepping semantics. The user owns implementation and exact private representation. No public snapshot expansion, IF/IE storage, interrupt dispatch or extra execution states are required for this task. Those features and RETI/HALT/STOP follow in separately specified work. This note supersedes earlier next-task and uncommitted-status notes below.
## CB integration validation checkpoint - 2026-11-14

This checkpoint supersedes older next-task notes below. Focused CB integration validation is complete locally: all 326 tests pass (228 unit, 98 integration), formatting and strict Clippy pass. New tests cover all 256 encodings through System/Bus with exact traces, independent result/flag oracles, every budget boundary, full-memory preservation/echo behavior for all 32 memory forms, branch interactions and a wrapped opcode fetch. See [CB_INTEGRATION.md](tasks/CB_INTEGRATION.md) for exact scope and limits. No emulator implementation or dependencies changed. No external ROM suite was run. BIT/RES/SET and this validation remain uncommitted; GitHub synchronization/publication is pending.

Next: specify interrupt/IME behavior and acceptance criteria before DI/EI/RETI/HALT implementation; define STOP separately. Current instruction implementation coverage remains 239/244 non-prefix base plus 256/256 CB, with five base instructions left. CPU refactors remain user-owned and deferred unless requested.
## CB table implementation checkpoint - 2026-11-14

This checkpoint supersedes older next-task notes. BIT, RES and SET are now implemented alongside the CbType/CbTransform type split. All 321 tests pass (227 unit, 94 integration); formatting and strict Clippy pass. All 256 CB bytes decode. BIT and RES/SET tests exhaust all eight bit indices, all eight targets, all input bytes and all initial flag combinations, checking full CPU snapshots and two/three/four-cycle completion as appropriate. BIT memory retry, captured address, PC wrapping and continuation checks also pass. Tests that expected unsupported CB bytes have been replaced by all-byte mapping checks because no CB slots remain unimplemented. Base unsupported-instruction tests remain.

Coverage: 239/244 non-prefix base instructions plus 256/256 CB instructions; five base instructions remain (STOP, HALT, RETI, DI, EI). This is instruction implementation coverage, not hardware or ROM-suite certification. Dedicated CB System trace/budget/memory validation and broader rotate/shift flag/target combinations remain gaps; no external ROM suite was run. Next: a focused CB validation checkpoint, followed by an explicit interrupt/IME design contract before DI/EI/RETI/HALT implementation. CPU transition/read refactors remain user-owned and deferred unless requested. These BIT/RES/SET changes are not yet committed; GitHub synchronization remains pending.
## Current checkpoint - 2026-11-13
Latest local checkpoint: CB 20-3F shifts and SWAP are implemented. All 319 tests pass (225 unit, 94 integration); formatting and strict Clippy pass. New shift tests exhaust values and initial flags for A and (HL); SWAP tests exhaust values and flags across all eight targets and check register preservation and cycle completion. CB-specific System traces/budgets and full shift-target coverage remain acceptance gaps; no external ROM suite was run. Implementation coverage is now 239/244 non-prefix base instructions and 64/256 CB instructions, leaving five base plus 192 CB encodings. Next implementation task: specify BIT, including its read-only (HL) path and distinct flag rules. The next-task sentence below describes the prior checkpoint.

This checkpoint supersedes older next-task notes below. CB dispatch and RLC/RRC/RL/RR for all eight targets are committed locally as 3ff50d9. The implementation shares rotate arithmetic, keeps base/CB Z policies explicit, uses fallible CB decoding and preserves Base/Cb fault encoding with a sticky fault at the instruction's prefix address. The last full run passed 316 tests (222 unit, 94 integration); after removal of the redundant CPU error variant, all nine CB tests, formatting and strict Clippy passed. No CB-specific System integration tests or external ROM suite were run; existing cycle tests exercise the CPU interface. Publication has not been verified.

Next task: [CB shifts and nibble swap](tasks/CB_SHIFTS_SWAP.md), starting with SLA, then SRA, SWAP and SRL. Reuse the existing CB cycle paths; the task records proposed selectors, flag/timing requirements and acceptance criteria. The user owns implementation. Current coverage is 239/244 non-prefix base instructions and 32/256 CB instructions; five base and 224 CB encodings remain. GitHub task synchronization remains pending.


## Ownership and scope

The user owns emulator implementation. The assistant owns planning, issue and milestone management, review, writing and maintaining automated tests, running checks, and recording results. Test ownership was clarified explicitly by the user; minimal test-module wiring is included in the assistant's scope.

The assistant also leads architecture: draft component boundaries, ownership, interface contracts, invariants, and tradeoffs before dependent implementation begins. The user can challenge or revise those proposals and owns the implementation. Architecture review must consider the overall system, not only compiler errors and test results.

## Development goals

The primary goal is to build software development skills and a portfolio project while enjoying emulator development. The user has intermediate Rust experience, including traits and generics, and has built a basic PICO-8 emulator. This project should develop experience structuring a larger Rust application, debugging interacting systems, validating behavior, and delivering a usable release.

Prioritize a finished, explainable scope over broad hardware support. Portfolio evidence should include a reproducible build, meaningful automated tests, documented design decisions and tradeoffs, focused issues and pull requests, a short demonstration, and an honest compatibility report. These are project goals, not guarantees about hiring outcomes.

Prefer concrete types and simple ownership initially. Introduce abstractions when an actual testing or implementation need justifies them. The assistant should explain requirements and review reasoning without supplying implementation code unless requested.

The current architecture discussion should begin with the CPU, bus, cartridge, system coordinator, and frontend boundaries. Settle timing and startup behavior before marking the core contract issue complete.

## Initial release scope

Proposed first release: an original monochrome Game Boy (DMG) emulator in Rust, with a desktop frontend, keyboard controls, audio, and ROM-only plus MBC1 cartridges. These are planning defaults, open to revision before the relevant milestone. Game Boy Color, link cable, additional cartridge controllers, save states, and performance optimization are deferred.

Scope clarification: focus implementation and architecture on DMG. Game Boy Color is a possible future extension, with no committed milestone or speculative implementation. Game Boy Advance is outside the project plan. Do not build a general multi-system framework in anticipation of either system.

First aim for observable, deterministic behavior. Introduce hardware timing checks as components become available. Declare compatibility by tested behavior and ROM results, without claiming full hardware accuracy.

## Frontend decision

Use SDL2 through the Rust sdl2 crate for the desktop frontend. The user has prior SDL2 experience.

Defer all SDL2 setup, dependencies, native libraries, frontend scaffolding, and feasibility work until frontend development begins in M4. Initial core development and tests should run headlessly. Audio integration follows in M6.

Keep SDL2 types and host resources in the frontend. The core should expose framebuffer data and audio samples and accept Game Boy button state without depending on SDL2.

## Core ownership decision

The frontend owns a System instance. System owns the CPU and Bus. CPU owns registers and execution state. Bus owns the cartridge, work RAM, and hardware devices as they are introduced.

System coordinates execution and gives CPU temporary mutable access to Bus when stepping. CPU and Bus do not own each other. Begin with ordinary ownership and borrowing rather than shared ownership through `Rc<RefCell<_>>`.

Bus routes memory accesses to the resources it owns. System coordinates advancement of emulated time. The core remains usable without a frontend.

The shared Memory trait has been removed by agreement. Use inherent Bus read/write methods for CPU addresses; Cartridge exposes its read-only ROM slice. ROM-only writes are ignored by Bus. Revisit a CPU-facing abstraction only when a concrete testing requirement calls for it. Bus access methods must be visible to sibling core modules (pub(crate) is sufficient).

After this refactor, all 27 tests, formatting, and strict Clippy pass. One obsolete direct-cartridge-write test was removed because that operation no longer exists; Bus ROM-write protection remains tested. Bus read/write methods are public and available to future sibling core modules.

## Timing decision

Target machine-cycle-level coordination from the start. CPU execution must allow System to interleave device advancement and CPU bus accesses within instructions. Do not execute all instruction side effects atomically and only then advance devices for the instruction's total duration.

Retain an instruction-step operation for tests and debugging, implemented through the smaller internal steps. Keep host pacing outside the core. Define the exact ordering of CPU accesses and device updates, timing units, and idle/interrupt steps before implementing the scheduler; devices may require finer internal timing.

Distinguish CPU machine cycles from device/system time explicitly. Implement the DMG relationship first; do not add GBC speed modes or hardware-model abstractions until that scope is adopted.

This is an accuracy target, not a claim of cycle accuracy. Validate access ordering and device interactions through focused tests and appropriate external ROM checks.

## Startup decision

Initially skip execution of the boot ROM and start cartridge execution from an explicitly defined DMG post-boot state. Authentic boot ROM execution is deferred.

Before implementation, specify the target hardware/boot variant and cite references for the chosen CPU register, device register, and memory initialization values. Mark approximations and unknown state explicitly rather than treating all memory as known or claiming exact power-on behavior. Skipping the boot ROM does not change the machine-cycle coordination target.

Still to settle for issue #1: exact timing conventions, the detailed startup profile and its sources, and the initial cartridge/bus test cases.

## Headless execution decision

Support advancing one machine cycle or stepping to an instruction boundary for tests and debugging. Instruction stepping must also be bounded so a halted or otherwise non-progressing CPU cannot hang a test.

Longer runs require an explicit emulated cycle budget and a caller-defined completion condition. Report completion, unsupported behavior, and budget exhaustion distinctly; exhausting a budget is never evidence of success. Host pacing does not determine emulated outcomes.

On failure, expose CPU state and bounded recent execution history. Define the trace fields and stopping semantics alongside the stepping API, keeping diagnostics independent of SDL2.

The broad architecture decisions are agreed. Remaining work for issue #1 is to specify exact timing and event ordering, source the startup profile, and list concrete initial cartridge/bus test cases. Do not mark the issue complete until those details are reviewed.

## Current baseline

- Rust 2024 library and binary crate; no dependencies. Core modules are declared by src/lib.rs; public API integration tests live in tests/core_api.rs.
- src/main.rs prints Hello, world!.
- ROM-only cartridge loading is in d03788d and the initial bus is in 85c6312. CPU now supports single-cycle NOP and LD A immediate execution, with register/startup/snapshot and trace/error tests. Hardware devices remain unimplemented.
- Public GitHub repository: [Lorithan/gameboy-emu](https://github.com/Lorithan/gameboy-emu), connected as origin.
- GitHub connector access and local GitHub CLI access are verified. The Windows keyring works outside the sandbox; sandboxed CLI authentication misleadingly reports an invalid token. Use approved execution outside the sandbox for authenticated CLI operations.
- Initial issues and all seven [GitHub milestones](https://github.com/Lorithan/gameboy-emu/milestones) are published. Issue #1 is assigned to M0; issues #2 and #3 are assigned to M1.
- M1 is complete. Issue #1/M0 remains open for architecture review and staged device startup; the first execution slice uses the explicit M-cycle approximation documented in task #5.
- Library extraction and the first CPU/System execution slice are verified: 67 tests (61 unit, six integration) and formatting pass. Strict Clippy is blocked by private unread CpuFault fields, unused future register variants, and pair setters. No warnings have been suppressed to hide these gaps.
- This checkpoint includes the library, CPU, System, tests, and architecture documentation. Publication is pending; bounded instruction stepping is not yet implemented.

## Proposed milestones

| Milestone | Deliverable | Completion evidence |
| --- | --- | --- |
| M0 — Project foundation | Scope decisions, component boundaries, timing vocabulary, test strategy, and repository setup | Decisions documented; build/test baseline recorded; GitHub linked; first implementation issues ready |
| M1 — Cartridge and memory foundation | ROM-only cartridge loading, header interpretation, and initial bus mapping | Valid and invalid input cases covered; memory regions, read-only ROM behavior, and unsupported hardware behavior verified |
| M2 — CPU execution | Registers, decoding, instruction families, flags, stack/control flow, and cycle accounting | Focused instruction tests cover edge cases; deterministic stepping and traces exist; selected CPU ROM checks run with documented outcomes |
| M3 — Timers and interrupts | Timer behavior, interrupt requests/service, HALT behavior, and device time advancement | Timing and interrupt tests pass for the declared scope; relevant external ROM tests run without hangs |
| M4 — Video and controls | DMG display, backgrounds/window/sprites, PPU timing, DMA, and joypad | Known visual fixtures compared; targeted PPU/DMA/input checks recorded; an agreed ROM-only interactive demo works |
| M5 — Cartridge compatibility | MBC1 banking and applicable battery-backed RAM persistence | Bank boundaries, modes, RAM enable/disable, and save/reload behavior verified; selected MBC1 software works |
| M6 — Audio and first release | DMG audio channels, host playback/pacing, usability, and compatibility report | Repeatable audio checks and listening review; regression suite recorded; supported software and known limitations documented |

Milestones express dependency order, not deadlines. Split each into small issues before starting it. Revisit the scope at each gate; test results may require returning to an earlier component.

## Testing strategy

1. Use focused unit tests for CPU flags, boundary values, memory behavior, banking, and device state transitions. Expected values should come from hardware specifications or independent fixtures.
2. Plan deterministic headless execution with a bounded cycle/instruction budget, observable output, and useful failure traces before relying on external test ROMs. A timeout is a failure, never a pass.
3. Add external ROM checks as their prerequisites become available. Record suite revision, hardware target, exact tests selected, success protocol, budget, outcome, and unsupported prerequisites.
4. Use framebuffer comparisons and controlled input scripts for video and integration work; add audio evidence when audio is implemented.
5. Run existing formatting, lint, and test checks during review. A green check alone does not close an issue: its acceptance criteria must also have evidence.

Primary references:

- [Pan Docs](https://gbdev.io/pandocs/) for hardware behavior.
- [gbdev opcode tables](https://gbdev.io/gb-opcodes/optables/) for instruction lookup, checked against the relevant specification sections.
- [Mooneye Test Suite](https://github.com/Gekkio/mooneye-test-suite) for hardware-oriented regression tests. Select tests matching the declared DMG target and implemented features; follow the suite's documented result protocol.

## GitHub workflow

- Repository: [Lorithan/gameboy-emu](https://github.com/Lorithan/gameboy-emu), public.
- Maintain the published GitHub milestones in line with the table.
- Start with a small actionable backlog rather than opening every future task immediately.
- Suggested labels: component:cpu, component:bus, component:cartridge, component:timer, component:ppu, component:audio, component:input, testing, documentation, bug.
- Each implementation issue states the behavior, scope, dependencies, acceptance criteria, validation method, and relevant references. Avoid embedding the implementation solution by default.
- Prefer one focused branch and pull request per issue. Review behavior and validation evidence before closing it.
- Track known inaccuracies and failing external tests explicitly. Reopen or create follow-up issues when evidence shows a regression.

## Design work — Define the emulator core contract

GitHub: [#1 — Define the DMG emulator core contract](https://github.com/Lorithan/gameboy-emu/issues/1).

Review draft: [Core architecture](CORE_ARCHITECTURE.md). It proposes the library boundary, CPU cycle-request/completion interface, and sourced ordinary-DMG CPU startup profile. Bus-access phase and device event ordering remain unresolved; distinguish these proposals from already agreed ownership and scope.

Goal: decide the boundaries that let CPU and hardware components be developed and tested incrementally.

This is a design issue; no emulator implementation is required to finish it.

Acceptance criteria:

- Confirm the initial DMG target and desktop-first scope.
- Describe responsibilities for the CPU, memory bus, cartridge, devices, and frontend.
- Choose and define timing units; distinguish CPU machine cycles from clock ticks and explain how devices receive elapsed time.
- Decide whether the first executable stage skips the boot ROM; document the source of any initial hardware state and keep authentic boot behavior as a separate task if deferred.
- Describe how a headless run stops, exposes results, and reports unsupported behavior.
- Identify focused tests for the first cartridge/bus issue.

## Current checkpoint - 2026-11-06

This checkpoint supersedes historical next-task/unpushed statements below. See [the project review](REVIEW_2026-11-06.md).

Local and live GitHub main are 8a2c2f5 (RST). Fresh checks pass 217 tests (151 unit, 66 integration), formatting and strict Clippy. No external ROM suite was run. No correctness defect was identified in reviewed paths; M2 remains incomplete.

Recovery decision: the later logic work is on an inaccessible crashed PC. The user has chosen to redo it from published RST checkpoint 8a2c2f5. Next: [AND/XOR/OR](tasks/ACCUMULATOR_LOGIC.md) under reopened #24, then CPL/SCF/CCF under #25. Historical 224-test evidence belongs to the inaccessible work; fresh acceptance evidence is required. No need to wait for recovery.

AND/XOR/OR redo is now complete: 226 tests pass (158 unit, 68 integration), formatting and strict Clippy pass, with no correctness findings. See [acceptance evidence](tasks/ACCUMULATOR_LOGIC.md). #25 CPL/SCF/CCF is complete locally: all 231 tests pass (161 unit, 70 integration), formatting and strict Clippy pass. Enum naming cleanup was approved by the user. See [completion evidence](tasks/COMPLEMENT_CARRY.md). Previous task: [#26 accumulator rotates](https://github.com/Lorithan/gameboy-emu/issues/26), specified in [ACCUMULATOR_ROTATES.md](tasks/ACCUMULATOR_ROTATES.md). All four rotates are now verified locally: 236 tests pass (164 unit, 72 integration), formatting and strict Clippy pass. No correctness findings; optional compute-then-commit readability suggestions are recorded in the task document. Rotates are committed locally as cf357f7; publication has not been checked. Previous task: [#27 DAA](https://github.com/Lorithan/gameboy-emu/issues/27), specified in [DECIMAL_ADJUST.md](tasks/DECIMAL_ADJUST.md). DAA is complete locally as of 2026-11-08: explicit wrapping fixes are verified; all 244 tests pass (170 unit, 74 integration), formatting and strict Clippy pass. All input states, independent decimal arithmetic, cycle/budget/memory and flag interactions are verified. See the task completion evidence. No correctness findings remain. Published as af8b12b; local and live GitHub main verified equal on 2026-11-08. Previous task: [#28 16-bit INC/DEC](https://github.com/Lorithan/gameboy-emu/issues/28), specified in [WORD_INC_DEC.md](tasks/WORD_INC_DEC.md). Start INC BC, then DEC BC, then DE/HL/SP. Cycle contract: capture on opcode completion, commit on the second Idle completion; preserve every flag. Word INC/DEC is now complete locally: Unary16 returns to FetchOpcode; all 251 tests pass (175 unit, 76 integration), formatting and strict Clippy pass. No correctness findings; acceptance evidence in the task document. Publication pending. Previous task: [#29 ADD HL,rr](https://github.com/Lorithan/gameboy-emu/issues/29), specified in [WORD_ADD.md](tasks/WORD_ADD.md). ADD HL is now verified locally: all 259 tests pass (181 unit, 78 integration), formatting and strict Clippy pass. Half-carry uses mask 1000; Z is left unchanged. Captured inputs, traces/budgets/memory and continuation checks pass. See WORD_ADD.md for evidence; no correctness findings remain. Publication pending. Previous task: [#30 signed SP offsets](https://github.com/Lorithan/gameboy-emu/issues/30), specified in [SIGNED_SP_OFFSETS.md](tasks/SIGNED_SP_OFFSETS.md). Signed SP offsets are complete locally as of 2026-11-10: all 269 tests pass (189 unit, 80 integration), formatting and strict Clippy pass. All 33,554,432 SP/offset/opcode cases, retry/capture/wrap, exact cycle/budget, memory and pointer/stack interaction checks pass. No correctness findings remain. See task completion evidence; committed as 84c67d1, publication not checked in this turn. Previous task: [#31 A transfers through BC/DE](https://github.com/Lorithan/gameboy-emu/issues/31), specified in [BC_DE_MEMORY_LOADS.md](tasks/BC_DE_MEMORY_LOADS.md). All four BC/DE transfers are now verified locally: 275 tests pass (193 unit, 82 integration), formatting and strict Clippy pass. No correctness findings. See the task completion evidence; publication pending. Coverage: 227 base implemented; 17 base plus 256 CB remain (273 total, CB dispatcher separate). Previous task: [#32 HL auto-update transfers](https://github.com/Lorithan/gameboy-emu/issues/32), specified in [HL_AUTO_UPDATE.md](tasks/HL_AUTO_UPDATE.md). All four HL auto-update transfers are complete locally: all 282 tests pass (197 unit, 85 integration), formatting and strict Clippy pass. No correctness findings. Captured-address updates, wrapping/retries/traces/budgets, full-memory and buffer checks pass; see task completion evidence. Publication pending. Coverage: 231 base implemented; 13 base plus 256 CB remain (269 total; CB dispatch separate). Previous task: [#33 absolute A transfers](https://github.com/Lorithan/gameboy-emu/issues/33), specified in [ABSOLUTE_A_TRANSFERS.md](tasks/ABSOLUTE_A_TRANSFERS.md). EA/FA are now complete locally: all 289 tests pass (201 unit, 88 integration), formatting and strict Clippy pass; no correctness findings. See task completion evidence. Publication pending. Coverage: 233 base implemented, 11 base plus 256 CB remain (267 total; prefix dispatch separate). Deferred repeated-read/PC-advancement extraction is recorded in CPU_REFACTOR_REVIEW.md; Previous task: [#34 LDH transfers](https://github.com/Lorithan/gameboy-emu/issues/34), specified in [LDH_TRANSFERS.md](tasks/LDH_TRANSFERS.md). LDH is complete locally as of 2026-11-11: all 295 tests pass (205 unit, 90 integration), formatting and strict Clippy pass; no correctness findings. See task completion evidence; publication pending. Coverage: 237 base implemented, seven base plus 256 CB remain (263 total; CB dispatch separate). Previous task: [#35 LD SP,HL](https://github.com/Lorithan/gameboy-emu/issues/35) in LOAD_SP_FROM_HL.md; F9 is now verified locally: all 301 tests pass (209 unit, 92 integration), formatting and strict Clippy pass, with no correctness findings. Publication pending. Coverage: 238 base implemented, six base plus 256 CB remain (262 total; prefix dispatch separate). Previous task: [#36 LD (a16),SP](https://github.com/Lorithan/gameboy-emu/issues/36) in STORE_SP_ABSOLUTE.md; all base load/store instruction encodings are now verified locally as of 2026-11-11: all 307 tests pass (213 unit, 94 integration), formatting and strict Clippy pass. No correctness findings. Publication pending. Coverage: 239 base implemented, five base interrupt/control plus 256 CB remain (261 total; CB dispatch separate). Next work: specify CB dispatch and register rotates/shifts, then memory variants and bit-operation families in small issues. Interrupt/HALT/STOP work requires its design contract before implementation. Deferred read/transition refactors remain separate user-owned work. HL auto-update is committed locally as 41da2bb, not yet pushed at this checkpoint. Coverage after #30: 223 base instructions implemented, 21 base plus 256 CB instructions remain (277 total), with CB dispatch also missing and invalid slots excluded. See [opcode coverage](OPCODE_COVERAGE.md). A separate proposed cycle-transition return refactor is recorded in CPU_REFACTOR_REVIEW.md, prompted by the user; implementation is deferred until a deliberate checkpoint. CB operations remain separate future work. Retain CPU/System/Bus boundaries. Specify interrupt sampling, device event ordering and startup before peripheral work. SDL2 remains deferred to M4.

## Current implementation task

Current CALL/RET checkpoint: all five CALL forms and C9 RET under [#21](https://github.com/Lorithan/gameboy-emu/issues/21) are complete and committed locally as 49fbe24 (not yet pushed). All 205 tests pass (143 unit, 62 integration); formatting and Clippy pass without warnings. Exact taken/untaken cycle paths, exhaustive targets, captures, wrapping, retries, budgets, nested HRAM/WRAM subroutines and full-address-space memory preservation pass. No correctness findings remain. See [CALL_RET.md](tasks/CALL_RET.md). [#22 — Conditional returns](https://github.com/Lorithan/gameboy-emu/issues/22) is complete locally, pending commit, with evidence in [CONDITIONAL_RETURNS.md](tasks/CONDITIONAL_RETURNS.md). All 211 tests pass (147 unit, 64 integration), formatting/Clippy pass without warnings, and no correctness findings remain. All four conditions, captures, exact five/two-cycle timing, retries, budgets, nested calls and full memory preservation are verified; C9 remains four cycles. Next specify RST; interrupts/RETI follow separately. The following checkpoints are historical.

Current stack checkpoint: all PUSH/POP BC/DE/HL/AF mappings and cycle paths under [#20](https://github.com/Lorithan/gameboy-emu/issues/20) are complete and committed locally as 9f6e47e (not yet pushed). All 192 tests pass (135 unit, 57 integration); formatting and Clippy pass without warnings. Source/destination selection, captured payloads, wrapping/retry/budgets, nested LIFO stacks, AF masking and full-memory preservation checks pass. See [STACK_PUSH_POP.md](tasks/STACK_PUSH_POP.md) for evidence. No correctness findings remain. Next: [#21 — Unconditional CALL/RET](https://github.com/Lorithan/gameboy-emu/issues/21), specified in [CALL_RET.md](tasks/CALL_RET.md). Begin with CD CALL a16, then C9 RET. Extend captured stack continuations to avoid an extra CALL jump cycle; conditional forms, RST and RETI follow separately.

[#6 — Byte-register immediate loads](https://github.com/Lorithan/gameboy-emu/issues/6), the pure decoder extraction, and [#7 — Immediate 16-bit loads](https://github.com/Lorithan/gameboy-emu/issues/7) are complete and published in commit b14d33f.

[#8 — Initial memory loads and stores through HL](https://github.com/Lorithan/gameboy-emu/issues/8) is complete and published in b14d33f: `LD (HL),A`, `LD A,(HL)`, and `LD (HL),d8`. See [the task document](tasks/HL_MEMORY_LOADS.md) for acceptance evidence. Baseline: 102 tests pass (77 unit, 25 integration), formatting passes; strict Clippy still reports unread CpuFault fields and unused set_af.

[#9 — Complete byte transfers between registers and memory at HL](https://github.com/Lorithan/gameboy-emu/issues/9) is complete and published in d826c0d, with acceptance evidence in [BYTE_TRANSFERS.md](tasks/BYTE_TRANSFERS.md). The 0x40–0x7F transfer block except HALT uses one new decoded register-copy operation and the existing memory states. Register-copy and register-store handlers share a source-register helper. Baseline: 111 tests pass (82 unit, 29 integration); formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af. No correctness findings remain.

[#10 — Byte-register INC and DEC with flag updates](https://github.com/Lorithan/gameboy-emu/issues/10) is complete and committed locally as 172556b (not yet pushed), with evidence in [REGISTER_INC_DEC.md](tasks/REGISTER_INC_DEC.md). A/B/C/D/E/H/L operations complete in the opcode cycle, set Z/N/H, and preserve C. Baseline: 117 tests pass (86 unit, 31 integration), including 57,344 exhaustive arithmetic/flag cases; formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af.

[#11 — INC and DEC through HL with read-modify-write cycles](https://github.com/Lorithan/gameboy-emu/issues/11) is complete and committed locally as cb42ea3, with evidence in [MEMORY_INC_DEC.md](tasks/MEMORY_INC_DEC.md). Captured-address ReadOperation leads to existing WriteMemory; shared helpers preserve the register arithmetic behavior. Flags update after the successful data read in the initial M-cycle model, and memory changes only in the final write. Baseline: 124 tests pass (91 unit, 33 integration); formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af. No correctness findings remain. Commits 172556b and cb42ea3 are unpushed.

[#12 — Register-source ADD and SUB with full flag updates](https://github.com/Lorithan/gameboy-emu/issues/12) is complete and committed locally as 6a53606, with evidence in [REGISTER_ADD_SUB.md](tasks/REGISTER_ADD_SUB.md). Captured original operands, all four flags, wrapping PC advancement, and explicit opcode-fetch completion are reviewed. Baseline: 131 tests pass (96 unit, 35 integration), including exhaustive operand pairs for all non-A sources and self-source/initial-flag coverage; formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af. Commits 172556b, cb42ea3, and 6a53606 remain unpushed.

[#13 — Memory and immediate operands for ADD and SUB](https://github.com/Lorithan/gameboy-emu/issues/13) is complete and committed locally as bc13caf, with evidence in [OPERAND_ADD_SUB.md](tasks/OPERAND_ADD_SUB.md). Unary and accumulator operation selectors are separate; immediate/memory accumulator states retain original A and the data address where needed. Completion, addressing, PC advancement, and retry captures are reviewed. Baseline: 138 tests pass (101 unit, 37 integration), including 262,144 exhaustive operand/source-kind/operation cases; formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af. Arithmetic commits 172556b, cb42ea3, 6a53606, and bc13caf remain unpushed.

[#14 — ADC and SBC across register, memory, and immediate operands](https://github.com/Lorithan/gameboy-emu/issues/14) is complete and committed locally as 17802f7, with evidence in [ADC_SBC.md](tasks/ADC_SBC.md). Helpers consume explicit carry input; pending reads retain it in AddC(bool)/SubC(bool) payloads with original A and captured memory address. Ordinary ADD/SUB ignore C and unary operations preserve C. Baseline: 146 tests pass (107 unit, 39 integration), including 2,097,152 non-A operand/source/operation/carry cases and the captured-carry regression; formatting passes. Clippy reports only existing unread CpuFault fields and unused set_af. No correctness findings remain. Arithmetic commits from 172556b through 17802f7 remain unpushed.

[#15 — CP across register, memory, and immediate operands](https://github.com/Lorithan/gameboy-emu/issues/15) is complete and committed locally as bee1ed0, with evidence in [COMPARE.md](tasks/COMPARE.md). CP preserves A, updates flags via subtraction with carry false, and reuses existing cycle states. Shared opcode PC advancement follows successful validation, and faults retain original opcode addresses. The last full run passed all 152 tests (111 unit, 41 integration), including 524,288 non-A comparison pairs and prior fault/PC regressions. The user then removed the obsolete unused addr local; formatting passes and Clippy reports only existing unread CpuFault fields/unused set_af. Recent arithmetic/comparison commits remain unpushed.

[#16 — Consolidate CPU opcode decoding and shared operation dispatch](https://github.com/Lorithan/gameboy-emu/issues/16) is complete and committed locally as 32aef0f (not yet pushed), with evidence in [ACCUMULATOR_DECODE_REFACTOR.md](tasks/ACCUMULATOR_DECODE_REFACTOR.md). Load/unary/arithmetic decoding is consolidated, regular transfer/arithmetic groups use bit fields, and execute_accumulator shares flags/A execution across all operand paths. Resolved carry and addresses remain captured. At that checkpoint all 152 tests pass (111 unit, 41 integration), formatting passes; Clippy reports unused set_af/set_pc only. Review priorities and deferred suggestions remain in [CPU_REFACTOR_REVIEW.md](CPU_REFACTOR_REVIEW.md). The user implements; the assistant owns tests and architectural review.

[#17 — Relative jumps](https://github.com/Lorithan/gameboy-emu/issues/17) is complete and committed locally as f34571f (not yet pushed). All 162 tests pass; detailed evidence is in [RELATIVE_JUMPS.md](tasks/RELATIVE_JUMPS.md). [#18 — Absolute jumps and JP HL](https://github.com/Lorithan/gameboy-emu/issues/18) is complete and committed locally as 4ef1bf1 (not yet pushed), with evidence in [ABSOLUTE_JUMPS.md](tasks/ABSOLUTE_JUMPS.md). All 169 tests pass (121 unit, 48 integration); formatting passes and Clippy reports only unused set_af. All six mappings, every immediate/HL target, condition captures, retry/wrap behavior, timing, budgets, and mixed jumps are verified. No correctness findings remain. [#19 — High RAM before stack instructions](https://github.com/Lorithan/gameboy-emu/issues/19) is complete and committed locally as 6d04444 (not yet pushed), with evidence in [HIGH_RAM.md](tasks/HIGH_RAM.md). All 175 tests pass (125 unit, 50 integration), formatting passes, and Clippy reports unused set_af only. Independent HRAM storage, non-aliasing, boundaries, CPU transfers/fetches, and budget-delayed writes are verified. Next: [#20 — Stack PUSH/POP](https://github.com/Lorithan/gameboy-emu/issues/20), specified in [STACK_PUSH_POP.md](tasks/STACK_PUSH_POP.md). Begin with PUSH BC then POP BC; use a separate BC/DE/HL/AF selector, captured stack data/addresses, and explicit per-cycle SP updates. CALL/RET follow separately.

2026-11-04 final review: execute_accumulator changes only A/flags, CP remains flags-only, and callers retain PC/state/completion handling. The duplicate live-carry check and StoreImmediateHL variant are removed; immediate stores use resolved destinations through LoadImmediate. Existing captured-state, missing-read, PC-wrap, unsupported-fault, and timing regressions pass. Further refactoring is deferred.

2026-11-04 relative-jump completion: #17 is complete and committed locally as f34571f. All five mappings, captured branch decisions/targets, signed PC wrapping, two/three-cycle timing, traces, retries, instruction budgets, and a finite DEC/JR NZ loop are verified. All 162 tests pass (117 unit, 45 integration); formatting passes and Clippy reports unused set_af only. See [RELATIVE_JUMPS.md](tasks/RELATIVE_JUMPS.md) for evidence. The next task is #18, absolute jumps and JP HL.

Execution-state refactor for #8 is complete: named pending cycles, a shared immediate-byte destination selector, captured memory addresses/data, and one write state. ReadMemory's destination is RegisterValue, and missing data reports the captured address. Shared register assignment has no cycle side effects. Full #8 coverage now includes all-byte round-trips, traces, budgets with no early writes, bus boundaries, flags, and wrap/retry behavior. The assistant changed tests/documentation only; application implementation remains user-owned.

Final #7 review: decoder mappings recognize all four word loads. Separate `RegisterValue` and `WordValue` types remove invalid destination combinations and the production `unreachable!()` branches. Acceptance coverage includes exhaustive operand values, flags, missing-data retries, all PC wrap stages, public traces, budget resumes, and mixed byte/word/NOP execution. Issue #7 is closed with local validation evidence.

Initial implementation backlog:

- [#2 — Load and validate a ROM-only cartridge](https://github.com/Lorithan/gameboy-emu/issues/2), complete and published in d03788d.
- [#3 — Establish cartridge ROM and work RAM bus behavior](https://github.com/Lorithan/gameboy-emu/issues/3), complete in checkpoint 85c6312.

Checkpoint d0960a4 contains the core library, CPU registers/flags, System startup, public snapshots, and single-cycle execution/traces. Checkpoint 3a388e9 adds bounded instruction stepping and completes #5; GitHub main is verified to match it. The baseline is 73 passing tests with formatting passing. Remaining strict Clippy failures concern unread fault fields and unused register scaffolding. Physical edge accuracy, device startup, HRAM, and other instruction families remain later work.

## Interrupt-register completion - 2026-11-17

This supersedes the 2026-11-16 next-task statement: #38 IF/IE controller and Bus mapping is implemented and verified locally. All 339 tests pass (241 unit, 98 integration), formatting, strict Clippy and diff whitespace checks pass. No external ROM suite was run. See [acceptance evidence](tasks/INTERRUPT_REGISTERS.md). No correctness findings remain. User owns implementation; assistant added five tests, updated old fallback expectations and applied approved temporary dead-code allowances for staged production hooks. Publication of this work has not been verified.

Current next task is the interrupt-entry architecture contract: sampling boundaries, priority, acknowledgment ordering, five-M-cycle entry, and a result that distinguishes interrupt service from instruction completion without advancing delayed-EI bookkeeping. Specify before implementation. RETI, HALT, STOP and peripherals remain separately scoped.

## Transition/outcome refactor checkpoint - 2026-11-17

The first slice of #39 is verified locally: private CycleTransition with centralized execution-state application, public CycleOutcome, successful CycleTrace plus CycleRecord failure diagnostics, and explicit bounded-step interrupt completion. User implemented application changes; assistant migrated tests and added two direct public API checks. All 341 tests pass (241 unit, 100 integration), formatting/strict Clippy/whitespace checks pass; no external ROM suite ran. User approved the staged-variant dead-code allowance and redundant-return cleanup. Changes are uncommitted. See [slice evidence](tasks/INTERRUPT_ENTRY.md).

Current next implementation: CPU acceptance and captured five-cycle entry states, followed by System coordination under the #39 contract. Interrupt acceptance/entry is not implemented yet; #39 stays open. Required-read extraction, RETI, HALT, STOP and hardware late-selection #40 remain separate.

## Interrupt-entry validation checkpoint - 2026-11-17

CPU acceptance, five entry states and System pending/acknowledgment coordination under #39 are implemented and verified locally. All 349 tests pass (249 unit, 100 integration); focused entry tests also cover all flags, formatting/strict Clippy/whitespace checks pass. No correctness defect found within the specified coarse model. No external ROM suite ran. See [acceptance evidence](tasks/INTERRUPT_ENTRY.md). The transition refactor was committed as 24f9739; entry implementation and tests are uncommitted. #39 remains open pending checkpoint review/commit. Remove obsolete staged dead-code allowances where production callers now exist before closing the checkpoint.

Hardware late selection/IE stack-write cancellation remains #40. RETI and HALT/STOP remain separate contracts; frontend/peripherals and required-read extraction remain deferred.

## Response-protocol validation - 2026-11-17

The #40 CycleResponse migration slice is implemented and verified: all 352 tests pass (252 unit, 100 integration), formatting/strict Clippy/whitespace checks pass. A lost signed JR conversion found by regression tests was fixed by the user; obsolete MissingReadValue is removed. Assistant migrated tests and added three response-protocol checks. No external ROM suite ran. Changes uncommitted. See [evidence](tasks/LATE_INTERRUPT_SELECTION.md).

Remaining #40 work: explicit CycleCompletion effects/private transitions, late source selection, cancellation, and System acknowledgment integration. Current entry still follows #39; do not treat sampled register construction alone as late-selection correctness.
