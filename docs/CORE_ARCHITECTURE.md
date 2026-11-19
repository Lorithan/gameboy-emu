# DMG core architecture
Latest naming checkpoint - 2026-11-19: StackRegister and EnableInterrupts replace ambiguous names; public CpuSnapshot spelling and all repository references are updated. cpu.rs uses qualified ExecutionState variants. No behavior changes; all 442 tests pass including five hash-verified fixtures, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#cpu-naming-checkpoint-verified---2026-11-19). Selected structural/naming cleanup is complete; next is M3 timer contract #53 before implementation.

Latest stack checkpoint - 2026-11-19: PUSH, taken CALL and RST share Push/PushHigh/PushLow with private PushThen::Finish/Jump. Captured values, idle/high/low cycles, wrapping SP updates and final-low-write jump are preserved; interrupt entry retains its separate sample/selection/acknowledgment sequence. All 442 unchanged tests pass including five hash-verified fixtures, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#shared-stack-push-checkpoint-verified---2026-11-19). No public API or dependency changes.

Latest execution checkpoint - 2026-11-19: private cpu/execution.rs owns pending-cycle completion and its three storage/word-finish helpers as another impl CPU. CPU retains request generation, response validation, transition/fault/EI policy, storage and interrupt/wake entry points. All four moved bodies preserve behavior; all 442 unchanged tests pass including five hash-verified fixtures, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#private-execution-checkpoint-verified---2026-11-19). The proposed ALU/dispatch/execution module split is complete.

Latest dispatch checkpoint - 2026-11-19: private cpu/dispatch.rs owns read_opcode as another impl CPU with parent-only visibility. Fetch effects, decode/fault behavior, captured operands and initial transitions are preserved; the public wrapper and pending-cycle completion remain in cpu.rs. All 442 unchanged tests pass including five hash-verified external fixtures; formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#private-dispatch-checkpoint-verified---2026-11-19). Completion extraction needs a separate boundary/visibility review.

Latest ALU checkpoint - 2026-11-19: eight existing arithmetic/bit helpers moved to private cpu/alu.rs as another impl CPU, with necessary parent-only method visibility. Equivalent shift operators replace Shl/Shr calls; state ownership, call sites, arithmetic and flags are preserved. All 442 unchanged tests pass including five hash-verified external fixtures; formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#private-alu-extraction-checkpoint-verified---2026-11-19). Dispatch-module extraction requires a separate boundary/visibility review before implementation.

Latest immediate-word checkpoint - 2026-11-19: six typed Imm16Use continuations share two validated PC-read states, replacing twelve variants. CALL captures return PC after high-byte increment; existing captured operands and downstream cycles are preserved. The obsolete store_immediate_word helper is removed. Full suite passed 442 tests including five hash-verified fixtures; final formatting and strict Clippy pass after helper deletion. [Evidence](tasks/CPU_READABILITY.md#shared-immediate-word-checkpoint-verified---2026-11-19). No public interface or dependency changes.

Latest fetch-state checkpoint - 2026-11-19: FetchOpcode owns skip_increment; CPU no longer has a separate suppression latch. Validated fetch effects, HALT rearming and interrupt return-PC rewind are preserved; rejected arbitration/responses retain state. All 442 tests pass including five hash-verified external fixtures, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#fetch-state-suppression-checkpoint-verified---2026-11-19). Shared immediate-word continuations need a separate design review before implementation.

Latest 16-bit register checkpoint - 2026-11-19: Registers owns BC/DE/HL/SP get16/set16; all four CPU selector matches delegate without moving operand capture or execution effects. All 441 tests pass including five hash-verified external fixtures, formatting and strict Clippy pass. [Evidence and next contract](tasks/CPU_READABILITY.md#16-bit-register-ownership-checkpoint-verified---2026-11-19). #58 proposes fetch-state-owned HALT suppression, retaining interrupt return-PC rewind and response retry atomicity.

Latest register checkpoint - 2026-11-19: Registers owns typed byte-register get/set; JumpCondition::is_met reads flags through Registers::get_flag and f is private. All 441 tests pass including five hash-verified external fixtures, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#byte-register-ownership-checkpoint-verified---2026-11-19). #57 proposes moving the remaining four BC/DE/HL/SP selection matches into get16/set16 while retaining CPU-owned execution effects and capture timing.

Latest helper checkpoint - 2026-11-19: private store_immediate/store_fetched/store_immediate_word return CycleTransition without Result. Fallible completion callers still validate responses before effects, and the shared wrapper owns state transitions. All 441 tests pass including five hash-verified external fixtures; formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#infallible-helper-checkpoint-verified---2026-11-19). Register ownership is the next proposed independent cleanup; no public API, timing or dependency changes.

Latest decoder checkpoint - 2026-11-19: CB decoding is infallible From; base decoding is TryFrom with an IllegalOpcode leaf for eleven invalid bytes. CPU fetch effects still precede handling the decode result, with original address capture and wrapper-owned sticky faults preserved. All 441 tests pass with five external fixtures included, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#decoder-validity-checkpoint-verified---2026-11-19). Next #56 cleans up infallible completion helper signatures; no public runtime error/timing or dependency change.

Latest cartridge diagnostic checkpoint - 2026-11-19: CartridgeError now supports Display/Error. Its FileIO variant retains path context and the original typed io::Error; validation failures remain leaves. Public callers can use standard boxed reporting without a production API/dependency change. All 439 tests pass with five external fixtures included, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#cartridge-error-checkpoint-verified---2026-11-19). Standard CPU/System/cartridge error cleanup is complete; #55's infallible CB decoding is the next independent cleanup.

Latest System diagnostic checkpoint - 2026-11-19: SystemError now implements Display/Error with typed CpuError sources. Completion messages use the completed tick and CycleAction's readable idle/read/write context; structured CycleRecord details remain intact. All 436 tests pass with five external fixtures included, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#system-error-checkpoint-verified---2026-11-19). Cartridge error reporting remains the next separate cleanup; execution/timing and dependencies are unchanged.

Latest CPU diagnostic checkpoint - 2026-11-19: CPU errors live in cpu/error.rs with stable root exports, Display and standard Error implementations. Typed source chains expose fault/protocol causes; outer context avoids duplicated causes, while structured fault metadata remains available. All 433 tests pass including the five external fixtures, formatting and strict Clippy pass. [Evidence](tasks/CPU_READABILITY.md#cpu-error-checkpoint-verified---2026-11-19). System/cartridge standard error reporting is the next separate cleanup; no emulator state/timing behavior or dependencies changed.

Latest milestone decision - 2026-11-18: [M2 review](M2_COMPLETION_REVIEW.md) finds the original functional CPU gate met; M2 is complete within its documented M-cycle/startup scope. Existing CPU/System/Bus interfaces need no rewrite. Last executable verification remains f0283fa with 430 passing tests including five real fixtures; this review changed documentation only. Next architecture work is M3 timer contract #53. Optional DAA confidence follow-up #52 reuses the test-only runner; PPU/DMA, mapper and physical STOP timing remain separately scoped.

Latest external subset checkpoint - 2026-11-18: four further pinned Mooneye fixtures pass through unchanged public APIs after source/prerequisite and checksum verification. [Evidence](tasks/MOONEYE_CPU_SUBSET.md) covers IF/IE, EI sequencing, high/low stack-write interrupt selection and DIV boundaries. All 430 tests pass with five external fixtures included, formatting and strict Clippy pass. No new production abstraction or emulator fix was needed. M2 remains under #51's evidence/completion review.

Latest external validation checkpoint - 2026-11-18: pinned Mooneye reg_f passes through unchanged public Cartridge/System APIs. Checksum-gated test invocation verifies the artifact; trace observation checks the six FF01 writes plus register signature without production test handling. All 426 tests pass when explicitly including the fixture, formatting and strict Clippy pass. [Evidence](tasks/MOONEYE_REG_F.md) limits the result to the F-mask/stack/register path. M2 remains open; #50 audits the next subset. This supersedes prior no-external-ROM status.

Latest validation checkpoint - 2026-11-18: test-only reg_f observer uses public Cartridge/System/trace/snapshot APIs. It bounds execution, validates the first output packet plus register signature and records diagnostic history; production emulation has no test-ROM special cases. All 425 tests pass, formatting and strict Clippy pass. [Runner evidence](tasks/CPU_ROM_VALIDATION.md#synthetic-runner-checkpoint---2026-11-18). No external ROM ran; #49 fixture provenance/execution is next and M2 remains open.

Latest STOP wake checkpoint - 2026-11-18: #48's deterministic entry/wake scope is verified. System::set_button updates Bus/IF then offers the selected-line level to CPU::wake_from_stop_if; only active Stopped transitions to FetchOpcode. No cycles, divider changes, IF acknowledgment or interrupt service occur in the input call; the next step uses ordinary arbitration. Execution state retains press/release pulses without another latch. All 414 tests pass, formatting and strict Clippy pass. [Evidence](tasks/STOP_WAKE.md#stop-wake-completion-verified---2026-11-18). #44 retains physical timing/glitch limits; no external ROM ran. This supersedes pending-wake/input-purity notes below.

Latest conditional STOP checkpoint - 2026-11-18: #48's StopFinished(StopBehavior) now applies conditional padding and state/outcome in the completion wrapper. Opcode handling selects from sampled JOYP level and masked pending interrupts; common delayed-EI bookkeeping covers all branches. Only actual Stopped entry emits StopEntered/reset DIV. All 409 tests pass (308 unit, 101 integration), including 18 focused STOP-related tests; formatting, strict Clippy and whitespace checks pass. [Evidence](tasks/STOP_WAKE.md#conditional-entry-completion-verified---2026-11-18). Next is immediate System input delivery to a guarded CPU stopped-wake method, with no elapsed cycles or interrupt service in that input call. This supersedes the unconsumed-sample status below. #48/#44 remain open; no external ROM ran.

Latest STOP input checkpoint - 2026-11-18: #48's pure Joypad/Bus level queries and mandatory CycleResponse::Read stop_wake_active field/accessor are implemented. System samples the selected-line level after each bus read alongside pending_interrupts; CPU does not consume it yet. Assistant migrated test constructors and added level/accessor/purity coverage. Strict Clippy passes without allowances. [Slice evidence](tasks/STOP_WAKE.md#read-sample-slice-review---2026-11-18). Next is typed conditional STOP entry; host-input wake remains separate. No external ROM ran. This supersedes the unimplemented-query status below.

Next proposed STOP integration - 2026-11-18: [#48 contract](tasks/STOP_WAKE.md) separates selected JOYP level, falling-edge IRQ requests and eligible pending interrupts. Begin with pure Joypad/Bus level queries, then mandatory Read sampling and a typed STOP behavior transition. Proposed System input handling offers the updated level to a guarded CPU wake method immediately, preserving press/release pulses between execution calls without extra cycles or a duplicate wake latch. Only actual Stopped entry emits StopEntered/reset DIV. This is not implemented; 03aff43 retains the previous 401-test verification. #44 keeps oscillator/glitch accuracy limits open.

Latest input checkpoint - 2026-11-18: #47 headless JOYP is verified. Bus owns Joypad and handles selected-line falling events into IF; public memory::Button and System::set_button deliver logical input without cycles/time/CPU transition. All 401 tests pass, including 11 joypad tests; formatting and strict Clippy pass with obsolete interrupt-request allowances removed. [Evidence](tasks/JOYPAD.md#headless-joypad-completion-verified---2026-11-18) covers device/Bus matrices, public readback, clock/CPU-state purity, HALT/entry behavior and memory oracles. STOP wake/conditional entry is next to specify under #44; current STOP behavior remains unchanged and no external ROM ran.

Latest device checkpoint - 2026-11-18: #46 divider foundation is verified. Bus owns Timer and maps FF04; System advances its base-tick counter before every real bus action and resets it on StopEntered after completion. Stopped/request-error returns advance nothing. All 390 tests pass, including 11 divider tests; formatting and strict Clippy pass. [Completion evidence](tasks/DIVIDER.md#divider-completion-verified---2026-11-18) records startup phase, clock/reset ordering and full-memory-oracle migration. No duplicate paused flag, CPU Bus borrow, speculative TIMA fields or new allowances. ABC8 remains a source-derived calibration for this coarse schedule. #44 JOYP/wake is next to specify; no external ROM ran.

Latest instruction checkpoint - 2026-11-18: #43 core STOP is verified. STOP's arm performs an additional wrapping padding skip, yields StopFinished, and the wrapper installs Stopped/emits StopEntered with shared instruction-boundary EI bookkeeping. System counts the real entry cycle once; later stopped requests produce no trace, tick advancement or CPU completion. All 379 tests pass, including eight STOP cases; [completion evidence and limits](tasks/STOP.md#core-standby-completion-verified---2026-11-18). Coverage is 500/500 encodings for the declared scope. Divider/joypad wake and finer STOP behavior remain #44; no external ROM ran. This supersedes the pending-padding/state notes below.

Current STOP review - 2026-11-18: decode, stopped state, StopFinished/StopEntered, delayed-EI bookkeeping and System handling are implemented. The STOP arm still omits its padding PC increment; six acceptance tests reproduce that single issue, including fetch suppression and wrapping. Stopped completion/wake guards and EI/DI standby tests pass. [Review evidence](tasks/STOP.md#stop-acceptance-review---2026-11-18). User fixes application behavior; assistant owns tests. Earlier unimplemented-state notes below describe prior checkpoints. Device-complete STOP remains #44.

Current stepping checkpoint - 2026-11-18: user implemented CycleRequest::{Execute(CycleAction), Stopped}, MachineStep::{Cycle(CycleTrace), Stopped} and bounded InstructionStep::Stopped results. CPU rejects completion without a cycle before mutation; System returns Stopped before time advancement and counts no cycle for that poll. All 371 tests pass after assistant request/trace migration. No stopped execution state exists yet, so those no-cycle branches are structurally reviewed but unexercised. [Evidence](tasks/STOP.md#machinestep-migration-verified---2026-11-18). The accepted direction supersedes proposal status below; STOP decode/state/entry event and shared EI bookkeeping are next, with device behavior separate.

Next design review - 2026-11-18: [STOP #43](tasks/STOP.md) proposes CycleRequest::{Execute(CycleAction), Stopped} in CPU and MachineStep::{Cycle(CycleTrace), Stopped} in System. Idle remains a real four-tick cycle; stopped polling has no trace or elapsed time. A private StopFinished transition would emit StopEntered and participate in common instruction-completion EI bookkeeping. CPU owns the single execution state; no duplicate System stopped flag. This is proposed, not implemented or approved. [#44](https://github.com/Lorithan/gameboy-emu/issues/44) tracks device reset/wake and finer STOP behavior after divider/joypad prerequisites; no core wake shortcut is proposed. Existing execution and 371-test baseline remain unchanged. Newer checkpoints supersede historical proposals below.

Latest instruction checkpoint - 2026-11-18: #42 HALT is implemented. HaltFinished applies Sleep or SuppressNextIncrement while reporting InstructionComplete; common post-transition EI bookkeeping runs for that outcome only. Halted idles remain InProgress. System wake_if_pending precedes normal interrupt acceptance. A private one-shot latch suppresses the next successful opcode increment; accepted EI/HALT entry rewinds the captured return PC once and clears the latch, while rejected acceptance preserves it. All 371 tests pass; [contract and limits](tasks/HALT.md). STOP and exact device-edge behavior remain separate.

Next design review: [HALT #42](tasks/HALT.md), 2026-11-18. Proposed read responses carry a value plus the eligible pending mask for HALT completion. CPU would add Halted idles, explicit wake polling independent of IME, a halt-completion transition that still advances EI bookkeeping once, and a one-shot opcode increment suppression latch. EI/HALT acceptance-time return-PC adjustment is documented as a proposed translation into our coarse model. No HALT implementation has started; review these boundaries before dependent work.
Latest instruction checkpoint - 2026-11-18: #41 RETI is implemented through shared ReadReturnLow/ReadReturnHigh/FinishReturn states carrying ReturnKind::{Normal, Interrupt}. IME is set directly to Enabled only on final RETI idle completion. Normal returns retain delayed-EI bookkeeping. System's next boundary can begin pending entry before the return target is fetched. All 361 tests pass; [RETI evidence](tasks/RETI.md). Older RETI proposal/unimplemented notes below are historical.

Next instruction contract: [RETI, #41](tasks/RETI.md), specified 2026-11-18. Required behavior is a four-cycle return with IME Enabled at final completion; no new public cycle events or System scheduler special cases. Proposed private return mode plus final return state share captured stack reads while separating RETI's IME effect from ordinary returns and generic jumps. Implementation has not started; representation remains user-owned.
Latest implementation checkpoint - 2026-11-18: #40 supersedes early source capture/acknowledgment. Acceptance starts entry and disables IME; the high stack-write completion latches IE, the low-write completion selects using later IF, and System acknowledges only InterruptSelected before returning the cycle trace. Cancellation is distinct from instruction completion and finishes entry at 0000. Public CycleOutcome events encode selection/acknowledgment directly; no separate CycleCompletion struct. All 355 tests pass; [contract and limitations](tasks/LATE_INTERRUPT_SELECTION.md) distinguish synthetic IE evidence from unverified IF collision/physical edge behavior.

Latest approved design (user approval 2026-11-17; implementation pending): [late selection contract](tasks/LATE_INTERRUPT_SELECTION.md), #40. CycleResponse would use Read(u8), WriteDone { interrupts: InterruptSample }, and IdleDone variants, making samples mandatory only on write responses; CycleCompletion would contain the public outcome plus an acknowledgment effect. The CPU would validate inputs before mutation; System would supply samples for every write cycle without inspecting private CPU phases. Public source visibility, response/errors and outcome changes are approved under that contract; helper names and private representation remain user implementation choices. Hardware-tested IE cancellation and unverified IF edge/collision timing remain distinct; current code still implements #39.

Interrupt-entry design selected 2026-11-17: [#39 contract](tasks/INTERRUPT_ENTRY.md). System offers pending masks before machine-cycle action queries; CPU explicitly accepts only at an enabled instruction boundary, owns five captured entry stages, and reports instruction/entry completion distinctly. System acknowledges through Bus, with no CPU Bus borrow and immutable request_cycle queries. Source/vector capture and acknowledgment at acceptance are the first-stage project approximation; [#40](https://github.com/Lorithan/gameboy-emu/issues/40) tracks known hardware late-selection/IE-write differences. This is specified, not implemented.

Status: library, registers, snapshots, and System construction are implemented. The first execution interface and explicit initial timing convention are specified in [task #5](tasks/FIRST_CPU_CYCLES.md). This is an M-cycle model, not a pin/edge-accurate implementation. Issue #1 stays open for architecture review and staged device initialization.

## Responsibilities and ownership

| Component | Owns | Responsibilities | Does not own |
| --- | --- | --- | --- |
| Frontend / executable | System instance, host resources | Input events, display, playback, host pacing | CPU execution rules |
| System | CPU, Bus, elapsed-time counter | Execute one coordinated machine cycle; bounded runs; snapshots/traces | Windowing or sleeping |
| CPU | Registers, current instruction state, temporary operands, execution/interrupt state | Determine the next cycle's action; consume read results; report instruction boundaries | Bus storage or device clocks |
| Bus | Cartridge, work RAM, later device state | Route accesses and enforce region rules; advance owned devices when System requests it | CPU registers or host time |
| Cartridge | Validated ROM, later mapper/RAM state | Cartridge-specific storage and mapping | The full CPU address space |

Keep ordinary ownership. No shared ownership, global machine state, callbacks into System from devices, or general hardware plugin framework is needed.

## Implemented library boundary

src/lib.rs declares the memory module as the emulation-core entry point. The executable no longer declares a duplicate memory module. Public API tests live in tests/core_api.rs and import gameboy_emu as an external library consumer; nested unit tests remain under memory. The executable will consume the library when machine execution is introduced.

Do not move every file at once or add dependencies. The current memory module can remain in place. Library extraction should be a separate small implementation task with the existing tests still green.

## Address-space interface

Keep Bus's inherent byte read/write methods and its u16 CPU addresses. Cartridge continues exposing its read-only ROM slice for ROM-only mapping. No Memory trait or Cartridge address-space methods are needed now.

Reads and writes route accesses without silently advancing time. Only coordinated System execution advances the machine. Later debug inspection must be distinguished from CPU reads if a device introduces read side effects; do not assume all future device reads can remain immutable. Defer that API change until an implemented device requires it.

## CPU cycle interface proposal

Use an explicit CPU state machine whose next externally relevant action is one of: read an address, write an address/value, or perform an internal/idle cycle. An instruction spans as many machine cycles as its documented sequence requires.

System obtains the cycle action, performs the coordinated bus/device work, and supplies any read value back to CPU. CPU commits the corresponding transition and signals whether an instruction finished. Internal register-only transitions must not fabricate extra elapsed cycles. Keep the cycle request and completion protocol internal to the core, with exactly one completion per request.

This keeps scheduling in System and permits deterministic trace tests without requiring CPU to own or retain a borrow of Bus. It adds explicit in-progress instruction state, which is the tradeoff for avoiding atomic whole-instruction execution. A CPU-facing memory trait is not necessary for this model.

Do not implement a queue of arbitrary heap-allocated micro-operation objects or a generic event framework. A small set of enums/state fields is sufficient; exact Rust representation belongs to the user once the interface is settled.

## Timing and observability

Use a clearly named elapsed base-tick counter, distinct from CPU machine-cycle counts. On DMG, one M-cycle spans four base ticks; the terminology follows [Pan Docs rendering](https://github.com/gbdev/pandocs/blob/master/src/Rendering.md).

The external machine-cycle step should advance those four ticks, including an idle cycle when applicable. Device advancement must remain capable of operating at base-tick resolution internally. Instruction stepping repeats cycle stepping with a budget; it does not bypass scheduling.

The initial execution convention is explicit: query one CPU action without mutation, advance four base ticks centrally, service the bus action once, complete the CPU transition once, then publish its trace. This is a project-level M-cycle abstraction, not a claim about the physical sampling edge. Hardware research describes level-sensitive bus behavior and fetch/execute overlap; exact edge interactions remain deferred until devices require them. Keep phase ordering centralized and validate it against device tests when refined. This supersedes the earlier plan to fully settle physical edge ordering before any instruction execution. Interrupt entry, HALT, and STOP require explicit state transitions in their own tasks.

Cycle traces should contain elapsed ticks, cycle action, address/value when applicable, and instruction-boundary information. CPU snapshots expose registers without granting mutable access to execution internals. Bounded run results distinguish an observed completion condition, unsupported behavior, and budget exhaustion.

## Startup profile proposal

Target the ordinary DMG boot variant, excluding DMG0/MGB/SGB. Skip boot ROM execution and prepare a documented cartridge-entry profile. From [Pan Docs power-up state](https://github.com/gbdev/pandocs/blob/master/src/Power_Up_Sequence.md):

| CPU state | Proposed value |
| --- | --- |
| A | 0x01 |
| BC | 0x0013 |
| DE | 0x00D8 |
| HL | 0x014D |
| PC | 0x0100 |
| SP | 0xFFFE |
| F | 0x80 if the header checksum byte is zero; otherwise 0xB0 |

The source notes checksum-dependent half-carry/carry flags, so a universal F=0xB0 is insufficient. Keep this initialization in a named profile rather than conflating it with a generic Default implementation. Tests can construct explicit CPU states independently of boot profiles.

The current Bus is not a full post-boot machine: HRAM is implemented at FF80-FFFE, but device registers still use fallback behavior. Before adding timer/interrupt or PPU behavior, specify device startup state and internal timing phase from relevant references. Observable register values alone do not specify complete device state. Continue deterministic zero WRAM as the documented project approximation.

## Interrupt-register boundary - 2026-11-16

Agreed: Bus owns InterruptController, CPU owns private IME, and System coordinates them; CPU never borrows Bus. The next scoped task is [#38](https://github.com/Lorithan/gameboy-emu/issues/38), with register rules, proposed visibility/interfaces and acceptance criteria in [INTERRUPT_REGISTERS.md](tasks/INTERRUPT_REGISTERS.md). Controller storage and Bus mapping are not implemented yet. Source enum and method names remain proposals. This task does not change CPU stepping or implement interrupt service.

The sourced DMG cartridge-entry register profile is IF=E1 and IE=00. IF high bits always read one; IE high bits retain written values but do not enable sources. Eligibility is IF & IE & 1F, independent of CPU IME. Register reads/pending inspection are pure; requests set one IF bit, acknowledgments clear one IF bit. Simultaneous request/write timing and interrupt-entry acknowledgment ordering remain for later scheduling contracts.

## Next small implementation tasks, after review

1. Completed: extract the core library boundary without changing behavior. Verification: 26 library unit tests and two public API integration tests pass; formatting and strict Clippy pass.
2. Represent CPU registers and flags with explicit invariants and snapshots. Specify their acceptance tests before implementation.
3. Ready: implement [task #5](tasks/FIRST_CPU_CYCLES.md), using the documented initial cycle ordering, traces, NOP/LD A immediate slice, and bounded instruction stepping.
4. Add HRAM before stack/control-flow instruction tests. Add other hardware only when its task is specified.

This draft does not complete issue #1: cycle event ordering and the staged device startup specifications remain open. Existing cartridge/bus tests already provide the initial memory evidence.

## Register implementation review — 2026-11-02

The user added a private register bank, register-pair helpers, and a Flag newtype with named flag accessors and a masking constructor. Individual byte registers use direct private field access. The corrected AF mask and Flag constructor pass exhaustive input checks; flag setters preserve other flags for all valid combinations.

Current verification: 32 library unit tests plus two public API tests pass; formatting passes. Strict Clippy fails on unused CPU scaffolding and new_without_default. Do not add a misleading boot Default just to satisfy the lint: separate explicit register-state construction from the named post-boot profile first.

Flag::new currently masks correctly, but Registers::set_af still writes its representation directly with a duplicate mask. Routing raw-byte conversion through the constructor would centralize the invariant. Flag remains in the CPU module, so its private representation is accessible there; moving it into a submodule would enforce encapsulation across CPU code if adopted. CPU::new still uses a fixed F=0x80; the checksum-dependent post-boot profile and read-only CPU snapshot API remain pending.

## Startup profile review — 2026-11-02

The user implemented Registers::dmg_post_boot(header_checksum) and CPU::from_registers, and set_af now routes through Flag::new. Tests independently construct explicit registers, exercise the boot profile for all 256 checksum bytes, and confirm CPU construction preserves explicit state. All 36 tests (34 unit, two integration) and formatting pass.

Visibility remains to resolve: Registers is private while CPU::from_registers is public, preventing future sibling System code from constructing it and triggering private_interfaces under strict Clippy. Recommended internal boundary: Registers, its startup helper, and CPU::from_registers are pub(crate), with register fields private. Unused CPU scaffolding also prevents strict Clippy from passing. System wiring and public read-only snapshots are not implemented yet; startup tests cover the profile and CPU constructor, not an end-to-end machine startup.

## System construction review — 2026-11-02

The user resolved internal visibility and implemented System::new(Cartridge). System reads the validated header checksum before moving the cartridge into Bus and constructs CPU from the named register profile. CPU and Bus remain private owned fields.

Four System tests verify zero and nonzero checksum paths, full ROM preservation through the owned Bus, and separate zero-initialized WRAM. A test-only CPU assertion helper inspects private registers without changing the production API. All 40 tests (38 unit, two integration) and formatting pass. Strict Clippy still fails on unused execution scaffolding; the read-only snapshot API is pending.

This establishes constructor wiring and CPU register startup only. Unimplemented device regions retain the bus fallback policy, and execution/timing are not yet implemented. Resolve the previously identified cycle event ordering before instruction execution.

## Snapshot and register module review — 2026-11-02

CPU::snapshot and System::cpu_snapshot return an owned CpuSnapShot value containing AF, BC, DE, HL, PC, and SP. Public integration tests cover both boot profiles, repeated reads, and mutation of returned snapshots without changing live state. Explicit-state tests verify CPU snapshots beyond the boot profile. The API name currently uses CpuSnapShot; CpuSnapshot would be conventional spelling, but naming is not a behavior blocker.

Registers and Flag now live in cpu/registers.rs. Unit tests moved beneath that module to preserve privacy. System startup assertions now use snapshots; the test-only CPU inspection bridge was removed. All 43 tests (39 unit, four integration) and formatting pass. Strict Clippy fails on unused flags/pair setters and System memory pending execution integration.

Before instruction implementation, settle the internal register mutation interface. Current private byte fields are inaccessible to the parent CPU module. Recommendation: expose ordinary byte registers and PC/SP to the parent using pub(super), retaining private fields outside the CPU subtree. Keep Flag's representation private and expose named flag operations through Registers. Moving Flag to its own child module would additionally prevent raw construction from register code if desired. Do not reinstate a getter/setter for every ordinary byte solely because of the module move.

## Register mutation interface — 2026-11-02

The user implemented pub(super) access for ordinary registers and PC/SP, retained private flag storage, and added Registers::get_flag/set_flag with FlagValue::{Z,N,H,C}. This satisfies the parent CPU's internal mutation needs without exposing the encoded flag byte. CPU can refer to the selector through its private registers child module; external callers do not currently have a re-export of FlagValue, which is acceptable for the internal instruction interface.

Two new tests exercise all flag combinations through Registers, checking getters and set/clear operations while preserving other flags, the unused-bit invariant, and all other registers. All 45 tests (41 unit, four integration) and formatting pass. Strict Clippy remains blocked only by unused pair setters and System memory. The register mutation boundary is ready; cycle event ordering remains the next architecture decision before instruction execution.

## Decoder extraction review

The user introduced a pure internal DecodedOpcode enum with Nop, LoadImmediate(destination), and Unknown, converted from a raw byte. CPU retains execution-state transitions and error-address context; the decoder does not touch Bus, registers, or time. This separates byte interpretation from cycle execution without a per-opcode type hierarchy.

Two decoder tests verify all eight supported mappings and rejection of every other byte. The complete suite has 81 passing tests (65 unit, 16 integration); formatting passes. Strict Clippy still reports existing unread CpuFault fields and unused pair setters. Keeping the raw byte in a local before decoding would avoid the safe-but-unnecessary later read_value.unwrap in the Unknown branch; no behavior bug was found in this refactor. Application logic was left unchanged by the assistant.

## Interrupt-register implementation checkpoint - 2026-11-17

The #38 proposal above is now implemented: Bus owns a private controller, maps FF0F/FFFF, and forwards crate-visible request_interrupt, pending_interrupts and acknowledge_interrupt operations. Interrupt is crate-visible through memory; the controller is confined to memory. The named post_boot constructor stores IF=01 and IE=00; IF reads E1 initially. CPU/System production stepping is unchanged. Acceptance evidence and temporary dead-code allowance removal conditions are in [INTERRUPT_REGISTERS.md](tasks/INTERRUPT_REGISTERS.md). Interrupt entry and device scheduling remain unimplemented and need their own contracts.

## Transition/outcome refactor checkpoint - 2026-11-17

Private CycleTransition and ExecutionState live in cpu/state.rs. Completion handlers return transitions; the public wrapper applies execution-state changes, latches faults and performs instruction-only delayed-EI bookkeeping. Public CycleOutcome carries progress or completed instruction/entry, with a numeric vector for entry. CycleTrace holds a CycleRecord and successful outcome; SystemError::CpuCompletionError holds the failed CycleRecord. Bounded stepping matches the plain outcome and reports InterruptEntered distinctly. This refactor slice passes 341 tests; actual interrupt acceptance/entry remains to be implemented under #39.

## Interrupt-entry implementation - 2026-11-17

The first-stage #39 entry model is implemented. CPU accepts only at enabled FetchOpcode boundaries, captures source/PC/SP, disables IME and runs two waits, two captured stack writes and one vector transfer. System offers Bus pending masks and acknowledges only a newly accepted source before querying the action. Entry completion is distinct from instruction completion; handler execution starts on the next cycle. Immediate source capture/IF acknowledgment remains the documented approximation pending #40. Detailed CPU/System test evidence is in [INTERRUPT_ENTRY.md](tasks/INTERRUPT_ENTRY.md).

## CycleResponse implementation checkpoint - 2026-11-17

CPU re-exports its public cycle protocol from cpu/cycles.rs. complete_cycle accepts Read(u8), WriteDone with a mandatory InterruptSample, or IdleDone; response-kind validation precedes handler mutation. System samples IF before writes and IE afterward. MissingReadValue is replaced by UnexpectedCycleResponse; failure diagnostics still use CycleRecord. This migration passes 352 tests. Completion effects, late selection and cancellation remain unimplemented; current entry still acknowledges at acceptance under #39.

## Read-response sampling implemented - 2026-11-18

CycleResponse::Read now carries { value, pending_interrupts }; System samples pending sources after the read. Existing read_value/kind helpers and non-HALT behavior are preserved; all 362 tests pass. HALT decoding, Halted state, wake-up and the one-shot fetch effect are still future #42 work. Read samples do not acknowledge requests or replace the separate boundary acceptance operation.
