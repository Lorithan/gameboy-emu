# Relative jumps

Issue: [#17](https://github.com/Lorithan/gameboy-emu/issues/17), M2 CPU execution.

## Scope

Implement `0x18 JR e8`, `0x20 JR NZ,e8`, `0x28 JR Z,e8`, `0x30 JR NC,e8`, and `0x38 JR C,e8`. The operand is a signed 8-bit displacement (-128 through 127). Its base is PC immediately after reading the operand; addition wraps at 16 bits. All flags and other registers, SP, and memory remain unchanged.

Taken jumps cost three M-cycles (12 ticks); untaken conditional jumps cost two (8 ticks). Both paths consume the displacement byte. Even a taken displacement of zero needs the internal cycle. See the [RGBDS instruction reference](https://rgbds.gbdev.io/docs/master/gbz80.7#JR_n16).

## Proposed architecture

Keep the existing request/completion interface and System ownership. Suggested type/state names below are proposals; the behavior is the task contract.

- A decoded relative-jump operation carries a condition: Always, NotZero, Zero, NotCarry, or Carry. Decode without reading CPU flags. Resolve the condition at successful opcode completion and capture the boolean decision.
- `ReadRelativeOffset { taken: bool }` requests a read at PC. Successful completion consumes the byte and advances PC once, wrapping. Untaken returns to FetchOpcode and completes the instruction here.
- Taken completion computes the target from the advanced PC and signed displacement, then queues `ApplyRelativeJump { target: u16 }`. Keep PC at the sequential address until the internal cycle completes.
- ApplyRelativeJump requests Idle. Completion with no read data writes the captured target, returns to FetchOpcode, and completes the instruction. There is no bus access or additional PC increment.

Target commitment on the internal cycle is our initial M-cycle observation convention, not a claim about physical edge timing. System already supports Idle; reuse that path.

Repeated requests must be side-effect free. Missing opcode or displacement data must report the requested address and preserve the pending state, PC, flags, and registers so retry works. Budget pauses retain the captured decision/target. Preserve unsupported-opcode fault behavior.

For an instruction starting at 0x0100, `18 FE` jumps back to 0x0100; `18 00` reaches 0x0102 in three cycles. A false conditional branch reaches 0x0102 in two cycles.

## Acceptance evidence

The assistant writes and maintains the tests; the user implements the emulator.

- Independent opcode/condition mapping and updated all-byte supported/unsupported classification.
- Every displacement at representative start addresses, including signed boundaries and wrapping during opcode fetch, operand fetch, and target calculation.
- Every initial flag combination for every condition; no changes to flags or other registers/SP/memory.
- Snapshots and traces after each cycle. Taken completion flags are false/false/true for Read/Read/Idle; untaken false/true for Read/Read. Idle has no read value, and no extra opcode is fetched.
- Missing-read retry, immutable repeated requests, captured decision despite later test-only flag changes, and captured target during budget pauses.
- Zero, partial, exact, and excess instruction budgets; accurate per-call counts and resume without duplicate application.
- A self-loop completes exactly one instruction per bounded step. A finite DEC/JR NZ loop demonstrates both branch outcomes and expected elapsed time.
- Existing regression suite and formatting pass; report Clippy warnings separately. No ROM fixtures or dependencies are needed.

## Implementation sequence

Start with unconditional JR and its displacement-read/internal-cycle states. Review and test that path before adding the four condition mappings. JP, CALL, RET, stack operations, interrupts, and HALT are separate tasks.
