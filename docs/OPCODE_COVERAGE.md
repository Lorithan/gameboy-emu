# Opcode coverage
## Core STOP checkpoint - 2026-11-18

STOP (10) now passes eight focused core-standby tests and the full 379-test suite (279 unit, 100 integration). Coverage is 244/244 legal non-prefix base plus 256/256 CB = 500/500 encodings. The CB dispatch byte is not a standalone instruction; eleven invalid base slots remain excluded. Decoder/protocol/EI/fault regressions retain their oracles with STOP's new completion event. See [completion evidence](tasks/STOP.md#core-standby-completion-verified---2026-11-18).

This is implemented/tested encoding coverage for the declared scope. STOP uses the documented core approximation and has no emulated wake path yet. Divider reset/pause (#46) and headless JOYP/input (#47) are verified, with the latest full suite at 401 tests. Selected JOYP STOP wake and conditional/edge behavior remain #44. No external ROM suite ran and M2's ROM-validation gate remains open. Earlier remaining-STOP/HALT counts below are historical.

## Current HALT checkpoint - 2026-11-18

HALT (76) now passes focused waiting/wake/bug/EI/CB/RST acceptance tests and the full 371-test suite. Coverage is 243/244 non-prefix base plus 256/256 CB = 499/500 encodings. Only STOP (10) remains. This is implemented/tested coverage, not a claim of complete CPU/hardware accuracy; no external ROM suite ran. See [HALT evidence](tasks/HALT.md). Older unsupported-HALT statements below are historical.
## Current RETI checkpoint - 2026-11-18

RETI (D9) is implemented using shared typed return states and enables IME at final instruction completion. Focused acceptance includes immediate pending interrupt entry before the returned-to opcode. Coverage is 242/244 legal non-prefix base instructions plus 256/256 CB = 498/500 encodings. Remaining base instructions: HALT (76), STOP (10). CB prefix dispatch is not a standalone instruction. IF/IE and late interrupt entry are implemented; older device/interrupt-absence statements below are historical. See [RETI evidence](tasks/RETI.md); no external ROM suite has been run.

Audited current decoder at 68e8cfe by compiling a temporary executable that imports the actual opcode and register modules and enumerates all 256 byte inputs. Compared unknown bytes with gbdev Opcodes.json. This counts implemented encodings, not hardware accuracy or completion of CPU/device milestone gates.

- 221 implemented base instructions out of 244 legal non-prefix base instructions.
- 23 legal base instructions remain.
- 0 of 256 CB-prefixed instructions implemented; all 256 remain.
- Total: 221 of 500 instruction encodings implemented, 279 remaining.
- The CB prefix dispatcher is also missing. It is not counted as a separate executable instruction in the 500 total.
- Eleven invalid base byte slots are excluded, rather than recorded as missing instructions.

## Remaining base instructions

| Category | Count | Bytes |
| --- | --- | --- |
| Loads/addressing modes | 16 | 02 08 0A 12 1A 22 2A 32 3A E0 E2 EA F0 F2 F9 FA |
| Signed SP arithmetic | 2 | E8 F8 |
| Interrupt/control | 5 | 10 STOP, 76 HALT, D9 RETI, F3 DI, FB EI |

CB families: 64 rotate/shift/swap, 64 BIT, 64 RES, 64 SET. These regular groups should be developed in focused family issues, not one implementation per byte. A raw opcode count does not measure remaining engineering effort; timing, interrupts, external ROM validation and device interactions are substantial separate work.

Reference retrieved 2026-11-08: [gbdev opcode data](https://raw.githubusercontent.com/gbdev/gb-opcodes/master/Opcodes.json). No ROM fixtures or dependencies introduced. Temporary audit source/executable live under ignored target/ and are not repository deliverables.
## Update - 2026-11-10

F8 and E8 now pass acceptance tests. Current implementation: 223/244 non-prefix base instructions, 0/256 CB instructions. Remaining: 21 base instructions (16 loads, five interrupt/control) plus 256 CB instructions = 277. CB dispatch is still missing and counted separately. The initial audit above is retained as the pre-task checkpoint.
## After BC/DE memory transfers - 2026-11-10

02/0A/12/1A now pass acceptance tests. Current implementation: 227 base instructions and zero CB instructions. Remaining: 17 base (12 loads, five interrupt/control) plus 256 CB = 273 instruction encodings; CB dispatch remains separate.

## After HL auto-update transfers - 2026-11-10

22/2A/32/3A now pass acceptance tests. Current implementation: 231 base instructions and zero CB instructions. Remaining: 13 base (eight loads, five interrupt/control) plus 256 CB = 269 instruction encodings; CB dispatch remains separate.

## After absolute A transfers - 2026-11-10

EA/FA now pass acceptance tests. Current implementation: 233 base instructions and zero CB instructions. Remaining: 11 base (six loads, five interrupt/control) plus 256 CB = 267 instruction encodings; CB dispatch remains separate.

## After LDH transfers - 2026-11-11

E0/E2/F0/F2 now pass acceptance tests. Current implementation: 237 base instructions and zero CB instructions. Remaining: seven base (08/F9 loads and five interrupt/control) plus 256 CB = 263 instruction encodings; CB dispatch remains separate.

## After LD SP,HL - 2026-11-11

F9 now passes acceptance tests. Current implementation: 238 base instructions and zero CB instructions. Remaining: six base (08 load and five interrupt/control) plus 256 CB = 262 instruction encodings; CB dispatch remains separate.

## After LD (a16),SP - 2026-11-11

08 now passes acceptance tests; all legal base load/store instructions are implemented. Current implementation: 239/244 non-prefix base instructions and 0/256 CB instructions. Remaining: five base interrupt/control instructions (STOP, HALT, RETI, DI, EI) plus 256 CB = 261 encodings; CB dispatch remains separate. Device and mapper semantics and external ROM validation remain outside this count.

## After CB rotates - 2026-11-13

Commit 3ff50d9 implements CB dispatch and CB 00-1F (RLC/RRC/RL/RR, eight targets each). Current coverage: 239/244 non-prefix base instructions and 32/256 CB instructions. Remaining: five base interrupt/control and 224 CB = 229 instruction encodings. All nine CB unit tests pass; no CB-specific System integration checks or external ROM validation have been run. Next: CB 20-3F shifts and nibble swap; see tasks/CB_SHIFTS_SWAP.md.

## After CB shifts and SWAP - 2026-11-13

CB 20-3F is implemented, bringing coverage to 239/244 non-prefix base instructions and 64/256 CB instructions. Remaining: five base plus 192 CB = 197 instruction encodings. All 319 tests pass (225 unit, 94 integration), with formatting and strict Clippy passing. Added checks exhaust A/(HL) shift values and initial flags, and all eight SWAP targets with values/flags, register preservation and cycle completion. CB-specific System integration and remaining task acceptance gaps are recorded in tasks/CB_SHIFTS_SWAP.md. Next implementation family: BIT.

## After BIT, RES and SET - 2026-11-14

All 256 CB instruction encodings are implemented and decode successfully. Combined implementation coverage is 495/500 instruction encodings: 239/244 non-prefix base plus 256/256 CB. The five remaining base instructions are STOP (10), HALT (76), RETI (D9), DI (F3), EI (FB). The CB prefix itself is dispatch, not an additional standalone instruction. All 321 tests pass (227 unit, 94 integration), with formatting and strict Clippy passing. Exhaustive BIT/RES/SET values/flags/targets and CPU cycle completion checks pass. CB System integration, remaining transform-target validation and external ROM validation are not demonstrated by this count. Changes are currently uncommitted.

## CB System/Bus validation - 2026-11-14

Focused integration validation now passes for all 256 CB encodings: exact traces/ticks and independent result/flag checks, every instruction-budget boundary, full-memory checks for all 32 memory variants across mapped/unmapped addresses and WRAM echo, mixed branch execution, and PC wrapping. All 326 tests pass (228 unit, 98 integration); formatting and strict Clippy pass. See tasks/CB_INTEGRATION.md for evidence and limitations. No external ROM suite was run. Coverage remains 495/500 implemented instruction encodings. Changes remain uncommitted.

## After DI/EI - 2026-11-14

DI (F3) and EI (FB) are locally verified. Coverage is now 241/244 non-prefix base plus 256/256 CB = 497/500 instruction encodings. Remaining: RETI (D9), HALT (76), STOP (10). The last full run passed 334 tests; after a visibility-only correction, all eight DI/EI tests, formatting and strict Clippy pass. IME lifecycle is covered; actual interrupt dispatch and hardware IF/IE behavior remain unimplemented. See tasks/DI_EI.md. Changes are uncommitted.
