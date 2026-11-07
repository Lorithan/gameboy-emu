# Game Boy emulator project plan

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

AND/XOR/OR redo is now complete: 226 tests pass (158 unit, 68 integration), formatting and strict Clippy pass, with no correctness findings. See [acceptance evidence](tasks/ACCUMULATOR_LOGIC.md). Next active task: #25 CPL/SCF/CCF, beginning with CPL; preserve Z/C and set N/H for CPL. Retain CPU/System/Bus boundaries. Specify interrupt sampling, device event ordering and startup before peripheral work. SDL2 remains deferred to M4.

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
