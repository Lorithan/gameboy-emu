# CPU contract

Functional DMG core. [Architecture](CORE_ARCHITECTURE.md) defines protocol/ownership; [coverage](OPCODE_COVERAGE.md) describes encoding/test scope.

## Registers and decoding

Registers owns byte and BC/DE/HL/SP selectors, pairs and named flag operations. StackRegister describes BC/DE/HL/AF for PUSH/POP. F is private and its low nibble stays zero, including POP AF. CpuSnapshot is an owned observation.

Decoding is pure: base TryFrom rejects eleven illegal bytes; all CB bytes decode infallibly. A validated fetch captures its original address, applies wrapping increment or HALT suppression, then handles decode success/failure. Illegal bytes return Faulted and the wrapper installs a sticky diagnostic fault. Rejected response kinds mutate nothing; faults do not mature delayed EI. Physical illegal-opcode lockup is not claimed.

## States and continuations

Pending states retain captured data across retries. Handlers return CycleTransition; the wrapper applies state/fault/completion and delayed EI. Infallible storage/finish helpers return transitions directly.

ReadImm16Low/High consume one validated PC byte each, increment once and assemble little-endian words. Imm16Use selects register load, JP/CALL, absolute A load/store or SP store. Conditions, A/SP values and CALL's first stack address are captured during opcode handling; CALL's return PC is captured after high-read increment. Untaken JP/CALL still consume both operands.

PUSH, taken CALL and RST share idle/high-write/low-write states with PushThen::Finish/Jump. SP uses captured wrapping addresses; CALL/RST set PC and finish on the low-write completion, with no extra jump cycle. RET/RETI share typed return states; RETI enables IME on final completion. Interrupt entry retains separate sampling/selection states.

## IME and interrupt entry

IF/IE eligibility is separate from IME. Acceptance occurs only at an enabled fetch boundary with an eligible source, captures PC/SP and disables IME. EI is delayed through the next instruction completion; DI cancels it. Consecutive EI does not postpone an already pending enable. HALT/STOP completions participate in shared EI handling.

Entry takes five M-cycles: idle, idle, high stack write, low stack write, vector transfer. The high write captures IE after its access; low completion selects priority from captured IE and IF-before-low-write. System acknowledges the selected source on low completion. Cancellation vectors to zero without acknowledging IF. Entry completion is distinct from the first handler instruction. This is the cycle-level model, not full physical collision behavior.

## HALT and suppression

HALT sleeps unless IME is not enabled and the opcode's sampled eligible mask is nonzero. That case returns FetchOpcode with skip_increment=true. The fetch state owns suppression: validated completion consumes it through its transition; retries preserve it; repeated HALT can rearm it. Ordinary fetch construction/wake/completion uses false.

Halted idle cycles clock devices. Eligible pending requests wake HALT regardless of IME; IME controls service. Accepting at an armed suppressed-fetch boundary preserves the wrapping return-PC rewind before entry capture. Rejected arbitration preserves state/PC. [STOP](DEVICES.md#stop-entry-and-wake) has a separate no-cycle state and JOYP level wake. **Wake timing ([#79](https://github.com/Lorithan/gameboy-emu/issues/79)):** System checks for a wake both before and after each M-cycle's device advance. A request raised during the advance wakes the CPU in that same step, regardless of IME. `wake_if_pending` returns whether it woke, and System re-requests the cycle action if the wake or `try_accept_interrupt` changed state. So with IME=1 the step becomes the first entry cycle, and with IME=0 it becomes the next opcode fetch, matching Mooneye halt_ime1_timing2-GS and halt_ime0_nointr_timing (NOP-wait timing).

## Evidence

Owning tests are in src/cpu/tests, opcode/tests.rs, registers/tests.rs and public tests. They retain independent result/flag/capture/wrap/retry/fault/action/budget oracles. [External results](TESTING.md) are narrow, not certification of all pin timing, DMA/PPU restrictions, authentic boot or analog STOP behavior.
