# M5: MBC1 and persistent RAM

**Agreed 2026-11-26:** the user accepted the five recommended decisions below. The `Header` type is proposed for C1 and open to the user's changes. M5's gate ([plan](../PROJECT_PLAN.md)): "bank/mode/boundary tests, save/load checks, selected supported software". This document proposes the architecture, the rules, the slices and the evidence. Items marked **Decision** need the user's choice; each has a recommendation.

## Sources

- **Behaviour:** Pan Docs [MBC1](https://raw.githubusercontent.com/gbdev/pandocs/master/src/MBC1.md) and [cartridge header](https://raw.githubusercontent.com/gbdev/pandocs/master/src/The_Cartridge_Header.md), read 2026-11-26 from gbdev/pandocs master (the pages carry no revision; pin a commit when the contract is agreed).
- **Test ROMs:** Mooneye `emulator-only/mbc1`, 13 ROMs, already inside the pinned archive `mts-20260714-0944-31510e1.zip` ([manifest](../../tests/fixtures/mooneye-reg-f.json)), so no new download is needed:

| ROM | Size | Observes |
| --- | --- | --- |
| bits_bank1 | 64 KiB | the 5-bit ROM bank register: which bits are kept, the 0-to-1 rule |
| bits_bank2 | 32 KiB | the 2-bit register |
| bits_mode | 32 KiB | the mode bit |
| bits_ramg | 64 KiB | the RAM-enable rule (low nibble 0xA) |
| ram_64kb | 64 KiB | 8 KiB RAM (64 kilobits) |
| ram_256kb | 64 KiB | 32 KiB RAM in four banks, mode 1 |
| rom_512kb, rom_1Mb, rom_2Mb, rom_4Mb | 64-512 KiB | every bank reachable, masking to the ROM size |
| rom_8Mb, rom_16Mb | 1-2 MiB | the large-ROM wiring: the 2-bit register as ROM bits 5-6, including 0000-3FFF in mode 1 |
| multicart_rom_8Mb | 1 MiB | MBC1M multicart wiring (see decision 1) |

Sizes in the names are in bits (512 kb = 64 KiB). Each ROM reports through the same Fibonacci-register convention the existing runner reads.

## Rules (from Pan Docs)

1. **Registers.** All four start at 0. Writes to 0000-7FFF go to the MBC; ROM is never written.
   - **0000-1FFF, RAM enable:** a value whose low four bits are `0xA` enables RAM; any other value disables it.
   - **2000-3FFF, BANK1:** five bits; higher bits are discarded.
   - **4000-5FFF, BANK2:** two bits.
   - **6000-7FFF, MODE:** one bit.
2. **The 0-to-1 rule.** If BANK1 is 0 it acts as 1. The check uses the full five bits, before masking to the ROM size: on a 256 KiB cartridge, BANK1 = `0x10` selects bank 0 at 4000-7FFF. BANK2 is not part of the check, so banks `0x20`, `0x40` and `0x60` are never reachable at 4000-7FFF.
3. **ROM mapping.** Bank numbers are masked to the cartridge's bank count.
   - 4000-7FFF: bank `(BANK2 << 5) | BANK1'`, where `BANK1'` is BANK1 after rule 2.
   - 0000-3FFF: bank 0 in mode 0; bank `BANK2 << 5` in mode 1.
   - BANK2 only matters when the ROM has more than 32 banks (1 MiB or more); masking removes it otherwise.
4. **RAM mapping.** A000-BFFF.
   - Disabled, or no RAM: reads return `0xFF` and writes are ignored (hardware returns open bus; `0xFF` is project policy).
   - 8 KiB: one bank.
   - 32 KiB: bank BANK2 in mode 1, bank 0 in mode 0. The byte is at `bank × 0x2000 + (addr - 0xA000)`.
5. **Large ROM or large RAM, never both.** 32 KiB RAM exists only with ROM up to 512 KiB; ROM of 1 MiB or more has at most 8 KiB RAM. So BANK2 drives either RAM banking or ROM bits 5-6, never both.
6. **No timing.** Register writes and bank switches take effect immediately. The cartridge has no clock.

## Accepted cartridges

| Header byte | Accepted | Rejected |
| --- | --- | --- |
| 0147 type | `00` ROM only, `01` MBC1, `02` MBC1+RAM, `03` MBC1+RAM+BATTERY | everything else, including `08`/`09` (no licensed cartridge; behaviour unknown) |
| 0148 ROM size | `00`-`06` (32 KiB-2 MiB, MBC1's limit); `00` only for ROM-only | `07` and above, `52`-`54` |
| 0149 RAM size | `00`; `02` (8 KiB) and `03` (32 KiB) for types `02`/`03` | `01` (see decision 2), `04`, `05`; RAM with types `00`/`01`; no RAM with `02`/`03`; 32 KiB with ROM over 512 KiB |

- The file length must equal the declared ROM size exactly. No padding or trimming.
- The header checksum is still required; CGB-only (`0143 = C0`) is still rejected; the logo and global checksum are still not checked.
- Each rejection keeps a specific `CartridgeError` naming the field and value, as now.

## Proposed architecture

- **The `Cartridge` owns the mapper.** It holds the ROM (`Box<[u8]>`, sized by the header), the RAM (`Box<[u8]>`, possibly empty), the parsed header, and a `Mapper` enum: `RomOnly` or `Mbc1(Mbc1)`, where `Mbc1` holds the four registers.
  - An **enum, not a trait object**: the set is closed and small, it stays `Debug` and `Clone`, and every case is visible in one `match`. MBC2/3/5 would be new variants.
  - Keep bank arithmetic in small functions of the registers (for example "which ROM bank is at 4000-7FFF"), so tests can check them without a Bus.
- **Bus delegates, nothing more:**
  - reads of 0000-7FFF and A000-BFFF go to the cartridge;
  - writes to 0000-7FFF and A000-BFFF go to the cartridge.
  - The OAM DMA source path reads ROM and A000-BFFF through the cartridge too (today it returns `FF` for A000-BFFF). Cartridge reads are pure, so DMA needs no special case.
- **The core does no save-file IO.** The cartridge exposes:
  - `has_battery()`;
  - `ram() -> &[u8]`, for the frontend to write out;
  - `load_ram(&[u8]) -> Result<(), CartridgeError>`, which requires the exact RAM size;
  - a counter that changes whenever RAM is written, so the frontend can tell whether there is anything new to save.
  This keeps the core portable to a web frontend later, where saves would go to browser storage.
- **Header (C1, proposed).** Parsed once into `Header { title: Option<String>, cgb: CgbSupport, kind: CartridgeKind, rom_size: RomSize }`, with `CartridgeKind` = `RomOnly | Mbc1 { ram: RamSize, battery: bool }` and `RamSize` = `None | Kib8 | Kib32`, so invalid combinations cannot be represented. `RomSize::from_code` and `RamSize::from_code` decode one byte each. `Header` replaces `CartridgeMetadata`; `metadata()` becomes `header() -> &Header`; `rom()` stays. Licensee codes, the SGB flag, the mask ROM version and the global checksum are left out until something uses them.
- **Header title.** The title is printable ASCII from 0134 up to the first `00`, stopping before 0143 when that byte is a CGB flag (`80`/`C0`). The frontend uses it in the window title when present, otherwise the file name.
- **Initial RAM content:** `0x00`, matching the WRAM/HRAM policy (decision 3).

## C1 test contract (agreed 2026-11-27, [#90](https://github.com/Lorithan/gameboy-emu/issues/90))

**Names.** `gameboy_emu::memory` exports `Cartridge`, `CartridgeError`, `Header`, `CartridgeKind`, `RomSize`, `RamSize` and `CgbSupport`.
- `Header::parse(&[u8]) -> Result<Header, CartridgeError>` checks the header only, from a slice of at least 0x150 bytes: the type, the ROM and RAM codes, their combination, the CGB flag and the checksum.
- `Cartridge::from_bytes` parses the header, then requires the length to equal `rom_size.bytes()` (`LengthError { expected, actual }`). `Cartridge::header() -> &Header`; `rom()` returns every byte. `metadata()` and `CartridgeMetadata` are removed.
- `RomSize::from_code` accepts 00-06, with `banks()` and `bytes()`. `RamSize::from_code` accepts 00, 02 and 03, with `bytes()`. All of these, `CgbSupport` and `CartridgeKind` derive `Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`; `Header` the same except `Copy`. `RomSize` and `RamSize` also derive `PartialOrd` and `Ord`, ordered by size (pinned by a test, since derived order follows declaration order).
- **Errors:** `UnsupportedCartridgeType { code }`, `UnsupportedRomSize { code }`, `UnsupportedRamSize { code }` and `InvalidHeaderCombination { cartridge_type, rom_size, ram_size }`, all `u8`, replace `VerificationFailure`. Messages show the codes as two hex digits.
- **CGB flag:** `80` is `CgbCompatible`, `C0` is rejected, and bytes without bit 7 are `DmgOnly`. Other values with bit 7 set are left unspecified.

**Tests,** written ahead of implementation: 30 in `src/memory/cartridge/tests.rs` (12 new, the rest migrated to `rom()` and `header()`) and the public-API test in `tests/core_api.rs`. Against a throwaway reference in a scratch worktree all pass, the whole core suite passes (577), and 23 of 24 deliberate breaks fail at least one test. The survivor treats every flag from 80 to BF as CGB-compatible, which is the unspecified case above.

**Until C2,** an accepted MBC1 cartridge still maps only its first 32 KiB, as ROM-only does.

## C1 result (complete 2026-11-27)

- **Implemented by the user** in `src/memory/cartridge.rs`, as agreed: `Header::parse` checks the length, the codes (through a private `CartridgeKind::new` that reads like the table above), the CGB flag, the title and the checksum; `Cartridge::from_bytes` adds the exact-length check and keeps the ROM as `Box<[u8]>`; the types are exported from `memory`. `VerificationFailure` is removed.
- **Tests:** 32 in `src/memory/cartridge/tests.rs` (13 new; `size_ordering_agrees_with_byte_counts` was added when the size types gained `Ord`, and fails when two variants are swapped) and the public-API test. During review they caught the RAM code `00`/`01` mix-up, 1028-byte kilobytes, the ROM sizes `07`/`08`, MBC1 decoded as ROM-only, title bytes from the manufacturer code, control characters and an all-space title accepted, the file length checked in `parse` (with RAM added), and two out-of-bounds panics on short input.
- **Regression:** `cargo test --workspace --all-features --no-fail-fast -- --include-ignored` passes **672, zero failed, zero ignored** (624 core, 48 frontend). fmt and workspace-wide strict Clippy pass; all eight checksum-gated fixture scripts pass.
- **Kept as is, by choice:** `from_bytes(&[u8])` copies the ROM once at load (at most 2 MiB); the checksum is checked after the fields, so a corrupted code byte is reported as that code. Neither is covered by the contract.

## Saves (frontend)

- **Where:** next to the ROM, same stem, `.sav`: raw RAM bytes, the format other emulators use, so saves move between them (decision 4).
- **Loading:** when a battery cartridge loads, read its `.sav` if one exists.
  - Wrong size: do not load it, show a warning, and **never overwrite that file** for the rest of the session. Losing a player's save is the worst failure here.
  - Missing: start with fresh RAM.
- **Writing:** write to `name.sav.tmp`, then rename over `name.sav`, so a crash mid-write cannot leave a half-written save.
- **When:** at the end of every session (another ROM, Esc, closing the window), and while running when RAM has changed and nothing has been written for a few seconds (decision 5).
- **Errors** (read-only folder, disk full) show in the window and are retried at the next save point; the game keeps running.

## Decisions (agreed 2026-11-26: all recommendations)

1. **MBC1M multicarts.** Recommended: **out of M5.** Detection is a heuristic (a Nintendo logo in bank `0x10`), few real cartridges use it, and it needs a second wiring mode. multicart_rom_8Mb is then expected to fail and is recorded as a limit. Alternative: add it as a final slice.
2. **RAM size code `01`.** Pan Docs: no cartridge ever had 2 KiB RAM, but some public-domain ROMs use `01` by mistake. Recommended: **reject** for now, with its own error; revisit if a ROM we care about needs it.
3. **Initial RAM content.** Recommended: **`0x00`**, as for WRAM. Real SRAM powers up with arbitrary data and games that care format it; `0xFF` is the other common choice.
4. **Save location.** Recommended: **next to the ROM**. Simple and conventional; it fails in read-only folders, which the error path covers. Alternative: a per-user data folder, which needs a naming scheme to keep ROMs with the same name apart.
5. **When to save.** Recommended: **session end plus a few seconds after the last RAM change**. Session end alone loses progress if the program crashes or the machine loses power.

## Slices (proposed issues)

| Slice | Scope | Evidence |
| --- | --- | --- |
| C1 header [#90](https://github.com/Lorithan/gameboy-emu/issues/90) | Parse and validate types `00`-`03`, ROM and RAM sizes and their combinations; ROM of any accepted size; `title()` | Header tests: every accepted combination, every rejection with its error, exact-length files, titles (11/15/16 characters, padding, CGB flag) |
| C2 MBC1 [#91](https://github.com/Lorithan/gameboy-emu/issues/91) | The mapper: registers, rules 1-6; Bus and DMA delegation | Local tests against an independent model of rules 2-4 (every register value, both modes, every ROM size); Bus tests for A000-BFFF and DMA from cartridge RAM; 12 Mooneye MBC1 ROMs through a new hash-checked script |
| C3 saves [#92](https://github.com/Lorithan/gameboy-emu/issues/92) | Core RAM API; frontend `.sav` loading and atomic writing, the mismatch rule, save timing; header title in the window | Core API tests; frontend tests for pure path and timing functions; manual: a battery game's save survives quitting and relaunching |
| C4 software [#93](https://github.com/Lorithan/gameboy-emu/issues/93) | Pin MBC1 homebrew, at least one with RAM and battery (Tobu Tobu Girl is MBC1+RAM+BATTERY; Porklike, PostBot, JP and Domination are candidates); smoke tests; play-through; M5 review | Hash-pinned manifests with licences; headless smoke tests; the user's play-through |

Existing ROM-only fixtures must keep identical M-cycle counts throughout.
