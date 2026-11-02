# Task #3 — Initial memory bus

## Goal
Route byte reads and writes for the initial DMG address space. Milestone: **M1 — Cartridge and memory foundation**.

**Ready to start.** #2 is complete and published in checkpoint d03788d. The agreed CPU/Bus/System ownership is sufficient for this task; remaining timing and startup work in #1 is not a blocker.

## Scope and behavior
Bus owns a validated Cartridge and **8 KiB of work RAM**. Expose byte reads/writes with a 16-bit address and 8-bit value. The user chooses Rust names and module layout.

| CPU address | Behavior for this task |
| --- | --- |
| `0x0000–0x7FFF` | Read the corresponding ROM byte; ignore writes for our ROM-only cartridge |
| `0xC000–0xDFFF` | Read/write work RAM |
| `0xE000–0xFDFF` | Read/write aliases of `0xC000–0xDDFF`, respectively |
| All other addresses | Temporary policy: reads return `0xFF`; writes have no effect |

Echo RAM shares storage with work RAM; it is not a separate copy. The echo ends at FDFF: DE00–DFFF have no echo in this range. Keep WRAM separate from cartridge ROM and from future cartridge RAM.

Initialize work RAM to zero for deterministic construction/tests. This is a project convenience, **not a claim about physical power-on or post-boot RAM contents**.

The fallback policy explicitly includes VRAM, absent external cartridge RAM, OAM, unusable memory, device registers, HRAM, and IE for now. It is not accurate device emulation. Do not allocate writable backing memory for those regions yet.

Reads/writes in this task do not advance time. Device scheduling, boot ROM mapping, CPU, HRAM, interrupts, DMA, PPU, mappers, SDL2, and generic hardware abstractions remain separate work. When devices arrive, replace their fallback regions with explicit routing.

## Acceptance criteria
- [ ] Construct a Bus owning a validated ROM-only Cartridge and isolated 8 KiB WRAM.
- [ ] Expose byte read/write access using u16 addresses and u8 values.
- [ ] Implement the map above, including bidirectional echo aliasing.
- [ ] ROM writes never change any ROM/header byte.
- [ ] Fresh RAM is zero under the declared deterministic policy; writes to one non-aliased location do not change another.
- [ ] All other addresses follow the explicit fallback behavior.
- [ ] Every u16 address can be accessed without panicking.
- [ ] Document implemented regions and temporary behavior.
- [ ] Assistant-written tests and existing cartridge tests pass; formatting and strict Clippy pass.

## Planned tests (assistant owns implementation)
- Read distinct fixture values at ROM offsets 0000, 3FFF, 4000, and 7FFF, plus header checksum at 014D.
- Write multiple values into ROM locations; verify original data remains.
- Read/write C000, CFFF, D000, DDFF, DE00, and DFFF with nonzero/non-FF sentinels; verify independence.
- Write C000 and read E000; write E000 and read C000. Repeat at DDFF/FDFF and an interior address.
- Verify writes at DFFF do not change FDFF's alias DDFF.
- Check boundaries around 7FFF/8000, BFFF/C000, DFFF/E000, FDFF/FE00, FF7F/FF80, and FFFE/FFFF.
- Access every unimplemented address: reads remain FF after writes; no aliasing into ROM or WRAM.
- Verify two Bus instances do not share writable RAM.
- Sweep the u16 address space without overflow in the test iterator; verify mapping against independent expected regions.
- Re-run all existing cartridge tests.

## References
[Pan Docs: Memory Map](https://github.com/gbdev/pandocs/blob/master/src/Memory_Map.md), especially the region table and Echo RAM. Consulted 2026-11-02. Fallback and zero-initialization rules above are project policies.

Start with ROM reads and ignored ROM writes. Add work RAM, then echo aliases and fallback behavior. Leave implementation to the user; the assistant will add tests when the interface exists.

## First implementation review

Added seven bus-routing tests and one ROM-write regression test; updated the cartridge API test import after the move into memory. No emulator logic was changed.

`cargo test` is currently blocked by two compilation errors in memory.rs: the bare cart reference needs field access, and Bus::write takes self by value instead of the trait's mutable reference. Formatting and diff whitespace checks pass.

Inspection also finds that Cartridge's new Memory::write mutates ROM, violating the ROM-only contract. A public Bus constructor is missing; current unit fixtures construct backing storage directly, so zero-initialization through a public constructor is not yet tested. Tests will be executed after the compilation fixes.

## Completion — 2026-11-02

The user fixed field access, write borrowing, ROM write protection, and added Bus::new. Test fixtures now use that constructor; a new test checks zero-initialized WRAM and echo across their entire ranges.

`cargo test`: 28 passed, zero failed. `cargo fmt --check` and strict Clippy passed. Routing tests sweep the full u16 address space, check alias boundaries and ignored writes, and verify separate buses have independent RAM. All cartridge tests also pass. No emulator logic was changed by the assistant.

Issue #3 is complete on local evidence; the new implementation/test changes still need committing and publication. Direct cartridge reads outside its ROM range remain outside the current bus routing contract. The device fallback is temporary behavior, not a hardware-accuracy claim.

## Architecture refactor checkpoint

Removed the shared Memory trait. Bus exposes inherent public read/write methods; cartridge ROM is accessed through its read-only slice. Cartridge no longer exposes address-space read/write operations. The redundant direct-cartridge-write test was removed; Bus ROM-write protection remains covered.

Final checks for this checkpoint: 27 tests passed, formatting passed, and strict Clippy passed. The checkpoint includes the module move under memory, bus implementation, tests, and architecture/verification documentation. GitHub publication remains pending.
