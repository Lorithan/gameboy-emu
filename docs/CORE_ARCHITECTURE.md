# DMG core architecture

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

The current Bus is not a full post-boot machine: device registers and HRAM still use fallback behavior. Before running software that uses the stack, implement HRAM. Before adding timer/interrupt or PPU behavior, specify device startup state and internal timing phase from relevant references. Observable register values alone do not specify complete device state. Continue deterministic zero WRAM as the documented project approximation.

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
