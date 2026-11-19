# First external CPU result: Mooneye reg_f

Task [#49](https://github.com/Lorithan/gameboy-emu/issues/49), completed 2026-11-18. Contract: [CPU_ROM_VALIDATION.md](CPU_ROM_VALIDATION.md). Fixture metadata: [mooneye-reg-f.json](../../tests/fixtures/mooneye-reg-f.json). User supplied the extracted ROMs; assistant verified the fixture and added the explicit test invocation. No application logic, dependencies, configuration or ROM headers were changed.

## Artifact and provenance

- Official archive: [mts-20260714-0944-31510e1.zip](https://gekkio.fi/files/mooneye-test-suite/mts-20260714-0944-31510e1/mts-20260714-0944-31510e1.zip).
- Source pin: [31510e12eea6286d36eea060a6adde755e1067aa](https://github.com/Gekkio/mooneye-test-suite/tree/31510e12eea6286d36eea060a6adde755e1067aa), selected [acceptance/bits/reg_f.s](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/reg_f.s).
- Archive SHA-256: `18aa29462dfe1fcd32a2cb3621733abdc72410074918b9245aa99a6920f7d3f2`.
- ROM SHA-256: `4b193e887ee3ac82b38b796729e1503e9a78da3e1140f8bd5600d0884f2e2627`.
- ROM is 32,768 bytes, cartridge type/ROM-size/RAM-size codes all zero, header checksum 2D. The normal Cartridge loader accepts it unchanged.
- Local ROM, reg_f.sym and LICENSE hashes match their entries in a separately downloaded official archive. Symbols describe WLA format version 3, main at 0150 and quit@halt at 4AB3. Full hashes are in the manifest.
- The pinned [CI workflow](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/.github/workflows/ci.yml) pins WLA-DX 89a90a56be5c2b8cf19a9afa3e1b32384ddb1a97 and builds with make clean all. The Makefile uses wla-gb with the common include path and wlalink -d -S. The release script packages .gb/.sym plus LICENSE. This is official-prebuilt provenance, not an independently reproduced build or signed build attestation.

The suite notice attributes Joonas Javanainen, 2014-2022, under MIT; LICENSE remains in the cache. The separately attributed Darkrose font is identified in common/common.s. Its 2032-byte pinned common/font.bin exactly matches the ROM at file offsets 4000-47EF. The manifest records its SHA-256 and [OGA attribution/license page](https://opengameart.org/content/8x8-ascii-bitmap-font-with-c-source), which distinguishes font.c and font.png licensing. The exact conversion from those assets is not independently established, so the whole ROM is not labeled exclusively MIT. No ROM/archive binaries are redistributed in this repository; both remain under ignored target/test-roms.

## Actual result

| Observation | Recorded result |
| --- | --- |
| Hardware/startup scope | Current documented DMG post-boot model; boot ROM skipped |
| Outcome | Passed |
| FF01 write packet | Decimal 3, 5, 8, 13, 21, 34 |
| B/C/D/E/H/L at report completion | Same six values |
| Consumed M-cycles | 3,946 |
| Consumed base ticks | 15,784 |
| PC / SP at packet completion | 4AB3 / E000 |
| AF / BC / DE / HL | 2210 / 0305 / 080D / 1522 |
| First explicit run host duration | 1.8046 ms, including file load; environment-specific |
| Caps retained | 1,000,000 M-cycles and 10 seconds; neither raised |

The pinned source checks F's lower four bits through PUSH/POP/register operations. This pass independently validates that assembled path. It does not validate general arithmetic, all cycle timing, serial transfer, PPU behavior, or complete CPU accuracy. The ordinary unmapped FF44/FF02 reads enable the source-audited fast reporting path; no production overrides were added.

## Reproduction and checks

Keep the unmodified extracted ROM at target/test-roms/mooneye/acceptance/bits/reg_f.gb, or pass -RomPath to this command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-reg-f.ps1
```

The script checks SHA-256 against the committed manifest before invoking the exact external test. Missing paths or mismatched bytes fail setup. This machine disallows direct .ps1 execution; the option applies to the launched process only, without changing persistent execution policy. The test's trace observer never changes production device behavior. A negative check supplying reg_f.sym as the ROM path failed at the hash guard before Cargo ran.

Ordinary cargo test leaves the external test explicitly ignored, so a checkout without locally supplied fixtures stays usable. That ignored result is not external validation. The command above executes it explicitly and requires a pass. After fixture verification, the full regression was also run with `cargo test --all-features --quiet -- --include-ignored`: all 426 tests pass (312 unit, 114 integration), including the actual external test, with no ignored tests in that run. Formatting, strict Clippy and whitespace checks pass.

## Next scope

#49 is complete. M2 stays open for broader independent validation. [#50](https://github.com/Lorithan/gameboy-emu/issues/50) audits a small next Mooneye subset and its device/startup/reporting dependencies before more execution. No other external ROM result is claimed. The archive's availability does not imply all tests are runnable on the current core.
