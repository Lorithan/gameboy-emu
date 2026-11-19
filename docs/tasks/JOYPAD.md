# Headless joypad and JOYP interrupt edges

Task: [#47](https://github.com/Lorithan/gameboy-emu/issues/47), next prerequisite of [#44](https://github.com/Lorithan/gameboy-emu/issues/44), following divider #46. Status: proposed contract, 2026-11-18. User owns application implementation; assistant owns tests/docs/review. Review this contract before dependent work.

Baseline fa3183f: 390 passing tests (290 unit, 100 integration), formatting/strict Clippy passing. No emulator tests were rerun for this documentation-only specification; no application code, dependencies or ROM fixtures changed.

## Ownership and first scope

Bus owns a private Joypad device in memory/joypad.rs. System exposes host input; CPU does not know button names or borrow Bus. This slice implements FF00, selected input transitions, joypad IF requests and headless button updates. Keep CPU STOP wake behavior unchanged until its next contract under #44.

Proposed types:

- Public Button enum: Right, Left, Up, Down, A, B, Select, Start. Define it in joypad.rs and reexport it from memory, like Interrupt. Derive Debug/Clone/Copy/PartialEq/Eq; no SDL2 key types in the core.
- Joypad visible to its owning memory module, with private selection and pressed-button storage. Proposed selection field contains only bits 4/5; pressed storage uses logical 1 for held buttons, translated to active-low register inputs when read. Do not cache redundant readback/edge state.
- Private-to-memory JoypadEvent::{None, FallingEdge}. It reports an input-line event, not whether any internal state changed. An unselected button press must still be stored even when it returns None.

Exact private names/storage choices remain user-owned. A boolean edge return is also valid if its meaning is documented; the event enum makes that meaning explicit at both callers. No general input framework or separate effects struct is needed.

| Owner | Proposed operation | Meaning |
| --- | --- | --- |
| Joypad | dmg_post_boot() | No held buttons, selection 00, no interrupt event |
| Joypad | read() -> u8 | Pure FF00 readback |
| Joypad | write(value) -> JoypadEvent | Update row selection and report any falling input edge |
| Joypad | set_button(button, pressed) -> JoypadEvent | Store the logical state, then report any falling input edge |
| Bus | set_button(button, pressed) | Update Joypad and consume its event into the interrupt controller |
| System | public set_button(button, pressed) | Forward host input to Bus; return no application-facing hardware event |

Use one private input-line calculation for readback and before/after comparisons. No Joypad tick method, input queue, public mutable device state, extra CPU cycle input or duplicate IF storage is proposed.

## FF00 and startup

Register and mask values in this contract are hexadecimal.

Bits 6/7 read one and ignore writes. Only selection bits 4/5 are writable. Bits 0-3 are read-only selected input lines. The proposed named DMG post-boot constructor uses no held buttons and selection 00, reading CF; construction does not request an interrupt or alter IF/IE.

| FF00 bit | Direction row (selected when bit 4 = 0) | Button row (selected when bit 5 = 0) |
| --- | --- | --- |
| 0 | Right | A |
| 1 | Left | B |
| 2 | Up | Select |
| 3 | Down | Start |

A selected held button pulls its corresponding input low. With both rows selected, a line is low if either corresponding button is held; neither selected yields low nibble F. Opposite directions can coexist as logical input states; do not silently filter them in the device.

Examples for readback:

| Selection bits | Selected rows | All released | Only A held | Only Right held |
| --- | --- | --- | --- | --- |
| 30 | Neither | FF | FF | FF |
| 20 | Directions | EF | EF | EE |
| 10 | Buttons | DF | DE | DF |
| 00 | Both | CF | CE | CE |

Write only the selection bits from the supplied byte; ignored bits cannot change pressed storage. Reads recompute readback without changing storage, time or interrupt state. Do not model switch bounce, electrical settling or SGB multiplexing in this deterministic digital slice.

## Interrupt transitions

For both selection writes and host button updates, compare the old and new selected low nibbles. Emit one FallingEdge event if any input changed from 1 to 0. Multiple falling lines in one update still cause one joypad request. Bus consumes that event through its existing request_interrupt(Interrupt::Joypad) helper; None has no interrupt effect.

This request sets IF bit 4 regardless of IE or CPU IME, preserves other IF bits and all IE bits, and never acknowledges an interrupt. Reads and pure releases cause no new falling edge. Clearing IF while a line remains low does not reassert it on repeated presses/reads; a later genuine falling transition is required.

Important sequences:

- Press A with neither row selected: store the press, read FF, no request. Select buttons next: low nibble F becomes E, requesting joypad.
- Write the same effective selection again, even with different ignored bits: no edge. Deselecting a held row raises its line and does not request.
- Select both, hold A, then press Right: the shared line is already low, so no second request. Releasing one while the other remains held keeps it low.
- Switching rows can raise some lines and lower others. Any newly lowered line requests, even if the numeric nibble value increased.

Keep CPU/Bus completion sampling conventions unchanged. A FF00 write requests through Bus during that write; System's existing IF-before-write/IE-after-write samples retain their meaning. This does not establish physical contact or IF-collision timing.

The existing request_interrupt dead-code allowances become obsolete once these production callers exist. Remove them in the integration slice rather than adding new allowances.

## Input delivery and CPU boundary

System::set_button is a synchronous host operation between execution calls. It consumes no M-cycles/base ticks, does not clock/reset DIV, and does not mutate CPU registers, execution state, IME or the HALT suppression latch. It is usable while CPU is running, HALTed, STOPped or faulted. Host call order determines input order; repeated identical updates are idempotent at the input-line level.

The next ordinary machine step already handles HALT wake and interrupt acceptance. A joypad request can wake HALT when enabled in IE, with normal IME-dependent service. Do not invoke those CPU operations inside set_button in this first slice.

STOP intentionally remains stopped in #47, even if input requests joypad IF. Its next contract must connect a selected-line wake signal separately from persistent IE/IF eligibility, preserve wake during press/release sequences between steps, and address held inputs/pending interrupts at entry. Do not extend wake_if_pending to treat Stopped as Halted, add a public force-resume method, or declare #44 complete merely because a joypad flag exists.

## Implementation slices and assistant acceptance

1. **Device model.** User defines Button, Joypad and the edge result, then the constructor/read/write/button operations. Assistant adds independent unit matrix/edge tests.
2. **Bus and input API.** User adds ownership, FF00 routing, event-to-IF requests, public Button export and System forwarding. Assistant adds end-to-end input/interrupt/HALT/timing tests and migrates old FF00 oracles.
3. **Checkpoint.** Full tests, formatting, strict Clippy and whitespace checks pass for the declared scope. #47 may then close; #44's wake/conditional-entry work remains open.

Acceptance cases:

- Exhaust all 256 held-button masks, four selection settings and all 256 written bytes. Derive expected lines by iterating named button mappings, independently of the implementation's bit packing.
- Exhaust old/new selection combinations for each held mask; verify exactly the intended selected-line falling events. Check each button press/release and repeated update under every mask/selection, including shared-line blocking and stored unselected presses.
- Check CF startup, constant high bits, ignored writes, repeated pure reads and independence of two devices/Bus/System instances.
- Bus requests IF bit 4 independently of IE/IME, preserving all other bits and storage. Verify manual IF clearing, no reassertion without an edge, and both FF00 writes and host updates.
- System input preserves ticks, DIV phase and CPU snapshot/IME across running/HALT/STOP/fault states. Exercise normal HALT wake/entry on the next step; STOP and sticky faults remain unchanged in this slice.
- CPU FF00 access retains correct read/write cycle traces and completion sampling. Update old unmapped/address/data/full-memory oracles for JOYP read masks and selection writes; do not blanket-ignore FF00 or all I/O. Keep existing clock-aware DIV checks.
- Remove now-obsolete request_interrupt allowances; run focused/full tests, formatting, strict Clippy and whitespace checks. No external-ROM conformance claim without executing recorded fixtures.

## Sources consulted 2026-11-18

- [Pan Docs JOYP](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Joypad_Input.md): selection, active-low matrix and read-only inputs.
- [Pan Docs interrupt sources](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Interrupt_Sources.md): selected-line falling edges and both-row shared-line behavior.
- [Pan Docs startup](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Power_Up_Sequence.md): normal DMG P1 = CF at cartridge entry.
- [Gekkio Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf#page=143), revision 199 (2026-11-06), register 11.1: writable/readable P1 fields and ignored/read-one bits 6/7.
- [Mooneye unused_hwio-GS](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/unused_hwio-GS.s): pinned MIT-licensed source includes read-one P1 high-bit expectations after writes. Reference only; no ROM/source fixture copied or run.
- [Pan Docs STOP](https://raw.githubusercontent.com/gbdev/pandocs/master/src/Reducing_Power_Consumption.md): selected input wake and conditional behavior requiring the follow-up contract.
