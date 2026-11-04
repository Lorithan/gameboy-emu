# Unconditional CALL and RET

Issue: [#21](https://github.com/Lorithan/gameboy-emu/issues/21), M2 CPU execution. PUSH/POP is committed locally as 9f6e47e. Baseline: all 192 tests pass (135 unit, 57 integration), formatting and Clippy pass without warnings.

## Required behavior

CD CALL a16 is a three-byte instruction taking six M-cycles (24 ticks). Read its unsigned little-endian target, push the address immediately after the operand bytes, and transfer PC to the target. C9 RET is one byte and takes four M-cycles (16 ticks): pop a little-endian target into PC. Neither changes flags or general register pairs. SP changes as for PUSH/POP. No host-language call stack or special stack memory is involved.

Example: CD 34 12 at 0x0100, with SP=0xFFFE, calls 0x1234. The return address is 0x0103. Write 0x01 to 0xFFFD and 0x03 to 0xFFFC, ending at SP=0xFFFC. RET reads 0x03 then 0x01, restores SP=0xFFFE and resumes at 0x0103.

All address arithmetic wraps at 16 bits, including operand fetch, return address, and stack addresses. Accesses outside writable memory retain ordinary Bus semantics; no stack range checks or errors. A RET without a prior CALL simply reads the bytes at SP.

Sources: [RGBDS instruction reference](https://github.com/gbdev/rgbds/blob/master/man/gbz80.7) for semantics/timing and [Pan Docs instruction encoding](https://github.com/gbdev/pandocs/blob/master/src/CPU_Instruction_Set.md) for CD/C9 mappings.

## Cycle observation contract

| CALL cycle | Action | Completion effect |
| --- | --- | --- |
| 1 | Opcode read | Advance PC once; capture initial SP S |
| 2 | Target low-byte read | Buffer low byte; advance PC |
| 3 | Target high-byte read | Advance PC; capture this PC as return address R and assemble target T |
| 4 | Idle | Queue high-byte write at S-1; no SP/PC change |
| 5 | Write R high at S-1 | Assign SP=S-1; queue low-byte write |
| 6 | Write R low at S-2 | Assign SP=S-2 and PC=T; instruction complete |

PC remains at sequential R from cycle 3 through cycle 5. CALL does not add the separate JP ApplyJump Idle after its writes; that would make seven cycles.

| RET cycle | Action | Completion effect |
| --- | --- | --- |
| 1 | Opcode read | Advance PC once; capture stack address S |
| 2 | Read low at S | Buffer low byte; assign SP=S+1 |
| 3 | Read high at S+1 | Assemble target T; assign SP=S+2; queue ApplyJump |
| 4 | Idle | Assign PC=T; instruction complete |

RET leaves sequential PC unchanged until its final Idle. Do not assign PC on the high read or complete there: ordinary POP has three cycles, but RET has four. These PC/SP commitment points follow the project's initial M-cycle observation convention, not a claim of physical internal register timing.

## Architecture proposal

Use distinct decoded CALL/RET operations. For CALL, add low/high operand states retaining captured initial SP and the low target byte where needed. Capture return PC only after successfully consuming the high operand byte. Both return address and target must remain available through later write cycles without rereading live PC or registers.

Now there is a concrete second use for stack writing. Extend the existing three PUSH states with an explicit completion selector, such as FinishInstruction versus JumpTo(target), carried through all three states. Ordinary PUSH queues FinishInstruction. CALL queues the captured return word with JumpTo(target). The last write completes either path; only the latter assigns PC. Keep the existing source register selector unchanged; PC is a return address value, not a new PUSH register.

Likewise, a POP continuation/destination can distinguish RegisterPair(existing selector) from ReturnAddress. Preserve existing ordinary POP behavior. After RET's high read, queue existing ApplyJump with the assembled address. Keep buffered bytes and captured stack addresses in payloads. No PC member is needed in PushOperand or Register16. Type/state names are implementation choices within this contract.

This is a focused extension of the existing state machine. Do not introduce a general microcode framework or speculative interrupt machinery. System and Bus APIs need no changes.

All requests remain immutable and repeatable. Missing opcode/operand/stack-read data reports the captured requested address and leaves pending state, PC, SP and registers unchanged for retry. Writes/Idle require no read data. Budget pauses preserve continuation, return address, target, and buffered byte, without duplicate writes or SP changes.

## Assistant-owned acceptance evidence

- Independent CD/C9 mappings and updated all-byte classification; other call/return forms stay unsupported.
- Every immediate CALL target and popped RET target, independent byte assembly, all initial flag combinations at relevant boundaries, and no general-register/flag changes.
- Return address equals opcode address+3 with wrapping; wrap at every fetch stage and SP boundary.
- Exact six/four-cycle traces, data values, completion markers, PC/SP snapshots, no extra Idle or read-ahead, and no early target assignment.
- Captured return address/target/stack address, missing-read retries at every read, repeated requests, and zero/partial/exact/excess budgets with resumes at every phase.
- Full memory preservation except CALL's two mapped writes; normal ROM/echo/HRAM/unimplemented bus behavior, including RET reads without a prior call.
- Nested subroutines in HRAM and WRAM return in LIFO order, restore SP and execute caller continuations while skipping unsupported sentinels. Calls may target themselves; one bounded instruction step still completes only one instruction.
- All prior PUSH/POP, JR/JP, arithmetic, bus and fault regressions pass; formatting and Clippy pass. Synthetic programs only, no dependencies or ROM fixtures.

## First slice and exclusions

Implement CD CALL a16 first and review its complete six-cycle path, then add C9 RET and nested-call tests. Conditional CALL/RET, RST, RETI, interrupts, and other instruction families are separate tasks. The user writes emulator implementation; the assistant owns tests and architecture review.
