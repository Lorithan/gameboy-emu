# Opcode coverage checkpoint - 2026-11-08

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
