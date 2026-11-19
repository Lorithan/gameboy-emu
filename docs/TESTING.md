# Testing and ROM evidence

Latest fixture-inclusive regression, #71 on 2026-11-19: **489 passed, zero failed**, with no-fail-fast so all targets ran (351 unit, 138 integration). All 471 local tests and eighteen hash-verified external fixtures pass after the user's PPU storage/Bus mapping and naming/offset cleanup. Formatting and strict Clippy pass. Five new tests verify the [video-memory foundation](DEVICES.md#video-memory-foundation); existing fallback/full-memory/CPU oracles explicitly account for newly writable VRAM/OAM. No PPU timing/rendering hardware result is claimed. The [M3 review](M3_COMPLETION_REVIEW.md) retains its earlier f0d38ef evidence, including the corrected rapid_toggle result in [timer evidence](testing/MOONEYE_TIMER.md).

## Commands

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

An ordinary fixture-free checkout explicitly ignores eighteen external tests; ignored is not passed. With locally supplied pinned fixtures, run checksum-gated scripts:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-reg-f.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-cpu-subset.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/run-mooneye-timer.ps1
```

The execution-policy flag applies only to that process. Scripts reject missing/mismatched hashes before Cargo. After fixture verification, run the full suite including ignored tests:

```powershell
cargo test --all-features --no-fail-fast --quiet -- --include-ignored
```

The timer script and fixture-inclusive suite now pass rapid_toggle without changing its source expectation or budget. A negative invalid-byte probe previously verified that the timer script rejects a hash mismatch before Cargo starts. `cargo test --test cpu_rom_runner` runs synthetic runner acceptance and leaves external fixtures ignored. Unit/ignored/empty runs are not external passes.

## Actual external results

All eighteen artifacts are unchanged official prebuilt images from source revision 31510e12eea6286d36eea060a6adde755e1067aa, using ordinary Cartridge/System and current skipped-boot DMG startup. No binaries are tracked.

| Fixture | Result | M-cycles | Base ticks | Record |
| --- | --- | ---: | ---: | --- |
| reg_f | Passed | 3,946 | 15,784 | [Provenance/results](testing/MOONEYE_REG_F.md), [manifest](../tests/fixtures/mooneye-reg-f.json) |
| if_ie_registers | Passed | 4,272 | 17,088 | [Subset audit](testing/MOONEYE_CPU_SUBSET.md), [manifest](../tests/fixtures/mooneye-cpu-subset.json) |
| interrupts/ie_push | Passed | 852,459 | 3,409,836 | Same subset; bounded PPU-helper timeout documented |
| ei_sequence | Passed | 3,943 | 15,772 | Same subset |
| div_timing | Passed | 4,235 | 16,940 | Same subset; resets DIV explicitly |
| Thirteen timer fixtures | All passed | Per-fixture counts | Per-fixture counts | [Timer audit/results](testing/MOONEYE_TIMER.md), [manifest](../tests/fixtures/mooneye-timer.json) |

Every fixture matched the first FF01 packet and B/C/D/E/H/L decimal signature 3,5,8,13,21,34. Corrected rapid_toggle takes 4605 M-cycles /18420 ticks; its earlier failure is preserved in the timer record. Caps remain 1,000,000 M-cycles and 10 host seconds per run, including a separately bounded diagnostic rerun on rapid_toggle failure. These results are finite checks, not complete timing/serial/PPU accuracy. External DAA [#52](https://github.com/Lorithan/gameboy-emu/issues/52) remains unrun and separate from the completed M3 gate.

## Runner contract

tests/support/rom_runner.rs observes public traces, with no production test special cases. FF01 writes are observable actions although serial is unimplemented. Only use this observer for sources audited to have no earlier SB reports and to select the common fast reporter under current FF44/FF02 fallback reads. Pinned sources: [reg_f](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/reg_f.s), [quit](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s), [PPU probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s), [serial probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_serial_broken.s).

The first six-byte packet plus register signature decides the result. Hex 42 in all six positions/registers is reported failure. Any other complete packet/register mismatch is malformed; partial output at timeout fails. Never scan past bad output for a later match. Other writes/incidental LD B,B bytes are not success.

Outcomes distinguish Passed, ReportedFailure, MalformedReport, TimedOut, UnexpectedStop and ExecutionError. Invalid/missing files fail setup. HALT consumes cycles; unexpected Stopped ends immediately rather than looping without a cycle budget advancing. Zero/exact budget boundaries and wall-time limits are tested. Do not silently increase caps to hide hangs.

Reports retain identity, packet, counts/ticks, snapshot and last 64 completed traces. Completion errors preserve/count the failed real cycle separately; request errors/stopped polls count zero. Host durations are diagnostics, not deterministic timing/performance promises. Eleven synthetic tests cover packet/signature/malformed/partial/wrong-address failures, loops/HALT/STOP, faults, limits, bounded history and file loading.

## Fixture policy

Before adding a fixture test, record source/revision, origin/SHA-256, accepted header, hardware/startup/device prerequisites, build/toolchain identity, component licenses, reporting and finite budgets. Keep ROM/archive binaries ignored under target/test-roms. Do not fetch moving latest data during tests or patch headers. Real bugs get isolated user-owned fixes and assistant regressions.

The [reg_f record](testing/MOONEYE_REG_F.md) preserves archive/build/MIT notices and separately attributed Darkrose font details; the whole ROM is not labeled exclusively MIT. No independent reproduced/signed build is claimed. Retain component provenance before redistribution. Other suites/flat-memory vectors need separate adapter/prerequisite/license audits.
