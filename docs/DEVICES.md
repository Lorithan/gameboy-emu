# Devices and startup

Implemented ordinary-DMG skipped-boot profile. [Architecture](CORE_ARCHITECTURE.md) defines scheduling; [programmable timer design](tasks/TIMER.md) remains a proposal.

## Cartridge and memory

Cartridge accepts validated unchanged 32 KiB ROM-only images with no cartridge RAM. Header/checksum/type-size constraints and errors are tested; unsupported configurations are rejected. MBC1/persistence belong to M5. No header patching or fallback mapper is used for fixtures.

| Address/range | Mapping |
| --- | --- |
| 0000-7FFF | Cartridge ROM; writes ignored |
| C000-DFFF | 8 KiB WRAM |
| E000-FDFF | Corresponding WRAM echo |
| FF80-FFFE | 127-byte HRAM |
| FF00 | JOYP |
| FF04 | DIV |
| FF05-FF07 | TIMA/TMA/TAC with tick counting, reload/write phases and DIV/TAC write-edge integration |
| FF0F / FFFF | IF / IE |
| Other | Read FF, ignore writes; unimplemented hardware |

WRAM/HRAM start zero as deterministic project policy, not physical RAM behavior. CPU starts PC=0100, SP=FFFE, A=01, BC=0013, DE=00D8, HL=014D. F=80 for zero header checksum, otherwise B0. Source: [Pan Docs post-boot profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md). Authentic boot and other console profiles are deferred.

## Interrupt registers

Bus owns the controller. Internal IF starts 01 (reads E1); IE starts 00. IF upper bits read one; IE upper bits retain writes but eligibility is `IF & IE & 0x1F`. Requests set one bit; acknowledgments clear one bit; reads/pending queries are pure. Priority is VBlank, LCD STAT, Timer, Serial, Joypad, with vectors 0040/0048/0050/0058/0060. [CPU entry/IME](CPU.md#ime-and-interrupt-entry) remains separate.

## Divider phase

Timer owns a wrapping u16 base-tick counter; DIV reads the high byte. Real M-cycles tick four times, including HALT/entry. DIV writes ignore their value and clear the whole counter; actual StopEntered resets it once. Neither resets monotonic elapsed_ticks. Stopped polling/input consumes no ticks. TIMA/TMA start zero; TAC stores low three bits and reads them OR F8. Tick and DIV/TAC write falling edges share Counting-only TIMA increments; overflow waits four future ticks before current-TMA reload and one-shot IF through Bus. Phase-dependent TIMA cancellation/protection and TMA propagation are locally verified. Writes preserve an existing pending/reload phase; TAC writes never reset the divider. [Detailed contracts](tasks/TIMER.md); the agreed coarse STOP policy below is verified locally, while hardware timer calibration remains pending.

The seed ABC8 at elapsed_ticks=0 is a **model-phase inference**, not an independently measured hidden register. Visible AB follows the [DMG startup profile](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md). Phase derives from pinned [Mooneye boot_div-dmgABCmgb](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/boot_div-dmgABCmgb.s) and our advance-before-access schedule:

| M-cycles at read | Counter from ABC8 | Source DIV |
| ---: | --- | --- |
| 14 | AC00 | AC |
| 78 | AD00 | AD |
| 141 | ADFC | AD |
| 205 | AEFC | AE |
| 269 | AFFC | AF |
| 334 | B100 | B1 |

First visible increment occurs at 56 ticks/14 M-cycles. These source-derived tests are not an executed boot_div ROM pass. Revisit phase if scheduling changes. The existing external div_timing explicitly resets DIV and does not certify startup phase. [DIV reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Timer_and_Divider_Registers.md).

## Joypad

Joypad stores an eight-button mask and selection bits 4-5. Zero in bit 4 selects directions; zero in bit 5 selects actions. Lines 0-3 correspond to Right/A, Left/B, Up/Select, Down/Start. Selected pressed buttons drive shared lines low; both selected rows combine presses. Readback is `0xC0 | selection | input_lines`; writes retain bits 4-5. Startup selects both rows with no held buttons, reading CF, a deterministic no-host-input profile.

Selection/button changes compare old/new lines: a selected high-to-low transition requests Joypad IF. Repetition, releases and already-low shared lines create no falling event. Active selected level is separately `input_lines != 0x0F`. [JOYP reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Joypad_Input.md).

## STOP entry and wake

CPU samples post-read selected JOYP level and eligible pending mask. If STOP is fetched, the coarse model consumes one opcode-read M-cycle, no padding bus read, and performs ordinary fetch increment/suppression before this table:

| Selected input active | Eligible pending | Skip padding | Next state | Outcome / DIV reset |
| --- | --- | --- | --- | --- |
| No | No | Yes | Stopped | StopEntered / reset |
| No | Yes | No | Stopped | StopEntered / reset |
| Yes | No | Yes | Halted | InstructionComplete / no reset |
| Yes | Yes | No | FetchOpcode | InstructionComplete / no reset |

Pending masks ignore upper bits, independent of IME; enabled interrupt acceptance still precedes opcode fetch. All branches share completion EI bookkeeping. HALT-like STOP does not invoke HALT's separate suppression decision.

System::set_button updates Bus/IF, then offers the active level to guarded CPU wake. Only active Stopped becomes ordinary FetchOpcode; PC/registers/IME are preserved. Input clocks nothing, resets no DIV, acknowledges no IF and performs no service. A selected press/release between execution calls leaves CPU awake through its state, without another latch. Pre-entry pulses are not queued; held input uses the entry table. Next real step arbitrates normally; IF alone cannot resume STOP.

The user-approved timer policy #68 retains the current entry reset without manufacturing a TIMA falling edge. All TIMA/TMA/TAC/phase state is preserved except the counter; remaining overflow delay and Reloading freeze while stopped. Wake consumes no ticks; real ticks resume the pending phase. Already-issued timer IF survives entry/pause and normal service follows only after selected-input wake. Tests also verify HALT-like/Continue branches keep clocking and avoid reset. This is explicitly a coarse policy, not sourced physical reset-to-TIMA edge fidelity.

Deterministic #44/#46/#47/#48 integration is complete. Immediate host wake, conditional padding and absent padding bus access are project approximations. Oscillator startup, analog edges/glitches/vector corruption, entry reset-to-TIMA detail and zombie power remain physical assessment [#64](https://github.com/Lorithan/gameboy-emu/issues/64). CGB KEY1 is outside scope. Sources: [STOP reference](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Reducing_Power_Consumption.md), [branch chart](https://raw.githubusercontent.com/gbdev/pandocs/master/src/imgs/stop_diagram.svg), [CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595). Existing tests cover entry/wake/polling, input matrices/pulses, IF/IME, resume PC, clocks and interrupt traces.
