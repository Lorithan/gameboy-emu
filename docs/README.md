# Documentation

Start with the [project plan](PROJECT_PLAN.md). Current work: [M3 timer design](tasks/TIMER.md).

| Guide | Contents |
| --- | --- |
| [Architecture](CORE_ARCHITECTURE.md) | Ownership, timing/order, cycle protocol, stepping and errors |
| [CPU](CPU.md) | Invariants, continuations, interrupts and HALT |
| [Devices/startup](DEVICES.md) | Memory, cartridge, DIV phase, JOYP and STOP limits |
| [Testing](TESTING.md) | Commands, runner, fixture requirements and current evidence |
| [Opcode coverage](OPCODE_COVERAGE.md) | Encoding counts and their limits |
| [M2 decision](M2_COMPLETION_REVIEW.md) | Completed milestone gate and follow-ups |
| [reg_f evidence](testing/MOONEYE_REG_F.md) | Artifact/build/license audit and measured result |
| [CPU subset evidence](testing/MOONEYE_CPU_SUBSET.md) | Four additional fixture audits/results |
| [Timer ROM evidence](testing/MOONEYE_TIMER.md) | Thirteen audited fixtures pass after the fetch-boundary correction |

`tasks` holds active proposals; `testing` retains provenance and measurements. Completed specs, superseded reviews and checkpoint logs were removed from the working tree on 2026-11-19. They remain in Git history and [closed issues](https://github.com/Lorithan/gameboy-emu/issues?q=is%3Aissue+is%3Aclosed). Old issue/commit links to deleted task paths are historical references; current guides replace them.

Update guides in place, clearly label proposals/approximations, and preserve source/license/startup/accuracy evidence during future consolidation.
