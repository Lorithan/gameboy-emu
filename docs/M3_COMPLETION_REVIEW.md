# M3 completion decision

Reviewed 2026-11-19 at [f0d38ef](https://github.com/Lorithan/gameboy-emu/commit/f0d38efd14f30535ea2d4fa60a9b6c0c26bde333): **M3 meets its declared DMG timer/interrupt integration gate**. [Milestone M3](https://github.com/Lorithan/gameboy-emu/milestone/4) requires timers, interrupt requests/service, HALT and device advancement, verified by focused timing/interrupt tests and relevant bounded ROMs. No remaining in-scope blocker was found; #53 can close with #65-#70 complete.

| Gate | Reviewed evidence |
| --- | --- |
| DIV/TIMA/TMA/TAC ownership, startup and mapping | Timer/Bus source; exhaustive register-byte/mask, divider-state, read-purity and CPU memory-access tests |
| Rates, enable and DIV/TAC falling edges | Independent arithmetic period/bit expectations across counter phases, all control transitions and upper-bit aliases; four rate and four DIV-trigger ROMs plus div_write |
| Overflow/reload and write collisions | Four subsequent ticks, zero window, current-TMA reload, one-shot event, TIMA cancellation/protection, TMA propagation; local phase/byte matrices and three reload/write ROMs |
| Interrupt requests, eligibility and entry | Bus IF independent of IE/IME, preserved existing IF, CPU eligibility/priority, five-cycle stack/vector entry, late IE/IF capture, cancellation, DI/EI/RETI; IF/IE, ie_push, ei_sequence and corrected rapid_toggle ROMs |
| HALT/STOP and time advancement | HALT/entry clocking, stopped zero-cycle polls, counter-only STOP reset, frozen/resumed phases, selected JOYP wake; separate delayed-EI and fully enabled STOP-fetch tests |
| Deterministic integration | Exact actions/ticks/snapshots/full-memory effects, budget pause/resume, zero budgets and request/completion faults; fresh-fetch red-to-green regression and operand/CB/internal-cycle controls |
| External acceptance | Thirteen pinned timer ROMs plus five existing ROMs pass unchanged, with hashes/provenance, first packet/register signatures and finite caps in [testing](TESTING.md) |

The verified full no-fail-fast fixture-inclusive run at this same application checkpoint passed **484 tests**: 346 unit and 138 integration, comprising 466 local and 18 external tests. Formatting, strict Clippy and whitespace checks passed. This review inspected the owning source, test oracles, recorded results and current GitHub issue inventory; it reuses that just-completed run and does not claim another test execution. No application/build/dependency changes were made for the review.

## Remaining limits and separate work

- Four-tick advance-before-access scheduling with post-advance fetch arbitration is a coarse model. Selected ROM passes do not certify every T-state alignment, every game or CGB behavior.
- Startup skips the boot ROM. ABC8 is a documented model-phase inference, not an independent hidden-divider measurement; div_timing resets DIV and does not certify authentic startup.
- Counter-only STOP reset, immediate selected-input wake and physical startup/glitch behavior retain their explicit limits. [#64](https://github.com/Lorithan/gameboy-emu/issues/64) is open and explicitly nonblocking for M3.
- [#52 DAA ROM](https://github.com/Lorithan/gameboy-emu/issues/52) remains unrun and nonblocking. [#1](https://github.com/Lorithan/gameboy-emu/issues/1) retains staged device/startup design work. Neither is silently completed by timer results.
- PPU, DMA, frontend, audio and additional mapper behavior belong to later milestones; serial/PPU fallback reporting in these fixtures is not validation of those devices.

Current behavior lives in [Timer](TIMER.md), [architecture](CORE_ARCHITECTURE.md), [CPU](CPU.md) and [devices](DEVICES.md). The completed task proposal is consolidated into these guides; per-checkpoint history remains in Git and closed issues. Reopen a gate if a demonstrated defect invalidates this scope.

Next: M4 architecture and first independently reviewable PPU contract. Define ownership, register interfaces, timing/events, memory-access policy and headless acceptance before user implementation; agree the ROM-only interactive demo and frontend scope before SDL2 work. This review introduces no new implementation or dependency.
