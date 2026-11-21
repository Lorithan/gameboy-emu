# Testing and ROM evidence

Latest fixture-inclusive regression, #76 on 2026-11-20: **533 passed, zero failed**, release build, no-fail-fast with `--include-ignored` (395 unit, 138 integration), after the user's STAT/VBlank interrupts. Formatting and strict Clippy pass; the three checksum-gated fixture scripts pass.

## STAT and VBlank interrupts (#76)

Thirteen new tests verify the [slice B contract](DEVICES.md#stat-and-vblank-interrupts):

- `memory::ppu::tests::interrupts` (10), against an independent position/line/edge model that shares no production code:
  - `Ppu::tick` effects at every tick for two frames, from the seed and from enable, for all 16 select combinations × LYC 0, 1, 143, 144, 153, FF (with a non-vacuity check on request counts);
  - hand-derived per-frame counts: HBlank 144, OAM 145, VBlank 1, LYC 1 (0 for FF), HBlank+VBlank 144, OAM+VBlank 144;
  - the VBlank effect exactly once per frame at line 144 / dot 0 for every select combination;
  - Bus IF at every M-cycle for two frames matching the model's OR of four ticks' effects;
  - no request at construction or before the first VBlank;
  - LYC writes while on: immediate flag, request only on a rising edge, no request without the select;
  - the STAT-write quirk in OAM scan / HBlank / VBlank / with the flag set (one request per write), not in Drawing with the flag clear, and not when the line is already high;
  - stat_irq_blocking rounds 1 and 2 reproduced through Bus by M-cycle polling, asserting that mode 3 was passed on every LYC 0-143 iteration;
  - no requests while the LCD is off for every STAT/LYC byte, and none at either LCDC edge;
  - the four stat_lyc_onoff rounds through IF bit 1 (request only for held false then true).
- `system::tests::ppu_interrupts` (3): VBlank and STAT HBlank requests wake HALT at M-cycles 16,529 and 176 from the seed and dispatch to 0040 / 0048 with the same request-to-entry delay as a Timer control; a same-tick VBlank and line-144 OAM pulse dispatch 0040 then 0048; with IME off only IF changes; a STAT-write request is dispatched at exactly the same step as a direct IF store at the same program position.

Migrations: the slice A every-M-cycle readback test and the System LDH-read test no longer assert "no PPU interrupts"; the System test now compares IF with its reference Bus. Two snapshot tests (passive LCD all-byte and LDH store mapping) now expect the STAT flag to follow LYC writes immediately (STAT 85 → 81 after LYC 5A). One hand count (OAM+VBlank) was wrong in the first draft, 145 rather than 144, because line 0's OAM is blocked behind VBlank; the independent model and production agreed. No mutation check was run; detection of the review-found defects (STAT effect clearing instead of requesting IF; LCD-off writes evaluating the flag/line) rests on the Bus IF model, off-write and stat_lyc_onoff tests.

**Fixtures.** All eighteen hash-verified fixtures pass with M-cycle counts identical to #75: 16 at 183,194-183,522, ie_push 179,867, div_write 899,476. None enable PPU interrupt sources in IE, so the new IF bits do not alter their paths. stat_irq_blocking, stat_lyc_onoff and vblank_stat_intr are reproduced locally, not run as fixtures (#73). Tests verify the provisional model, not hardware timing.

## Live LY/STAT (#75)

Twelve new tests verify the [slice A contract](DEVICES.md#live-ly-and-stat-readback):

- `memory::ppu::timing::tests` (4): `stat_bits` for each mode; `visible_ly()` and `mode()` at every dot for two frames from enable and from the seed; line 153 shows 153 for exactly dots 0-3; LCD-off state reads LY 0 / HBlank.
- `memory::ppu::tests::status` (7), black-box through FF41/FF44:
  - startup LY 00 / STAT 85 with pure reads;
  - every STAT byte write (selects only);
  - STAT bit 7 and select persistence while the LCD is off;
  - every FF44 write ignored;
  - LY/STAT at every M-cycle for two frames, for LYC 0, 1, 143, 144, 153 and FF, with all selects set and IF unchanged, including exactly one LY 153 sample per frame;
  - the four stat_lyc_onoff held-flag states across LCD off/on.
- `system::tests::ppu_timing` (2, plus extended mirroring): every CPU LDH read of FF41/FF44 equals a reference Bus at that M-cycle over three frames, observing LY 0-152 and both flag states; budget pauses leave LY/STAT reads unchanged. The 9-M-cycle poll loop cannot sample the single LY 153 phase, so that case stays with the Bus-level tests.

The stat-bit-7 test was added during review after the original six missed a defect that a stray stored bit was masking. Existing tests migrated FF41/FF44 from fallback (startup 85/00), and two full-memory snapshots now exclude LY/STAT where time or LCDC edges legitimately change them. FF46 stays fallback.

**Fixture reporter path.** Live LY makes `is_ppu_broken` report a working PPU, so all eighteen fixtures now take Mooneye's on-screen result path. All still pass. Measured M-cycles:

| Fixture | Before #75 | After #75 |
| --- | --- | --- |
| 16 assertion fixtures | 3,943-4,677 | 183,194-183,522 |
| ie_push | 852,459 (LY-wait timeout) | 179,867 |
| div_write | 721,229 | 899,476 |

Each fixture added about 179,000 M-cycles. That is more than the contract's rough 53,000-cycle wait estimate, because screen setup costs more than estimated. `wait_ly_with_timeout` timeouts are silent and cost 851,968 M-cycles, so these counts show that every LY wait completed: all non-div_write fixtures stay far below one timeout, and a div_write timeout would exceed the unchanged 1,000,000 cap. Its margin is now about 100,000 M-cycles. Tests verify the provisional model, not hardware timing; calibration is #73.

## LCDC and dot clock (#74)

At 49b8dd4: 508 passed (370 unit, 138 integration). Fifteen new tests verify the [#74 contract](DEVICES.md#lcdc-and-internal-dot-clock):

- `memory::ppu::timing::tests` (5): constructor states; every dot for three frames after enable against an independent range-table model, which is itself spot-checked at each boundary; exact 70,224-dot frame period; the 452-dot post-boot tail.
- `memory::ppu::tests` (6): FF40 all-byte readback and read purity; 6 starting values × 256 writes for enable-edge effects; disabled freeze across all disabled bytes with an identical Timer/IF to an enabled bus, then restart from the enable write; four dots per Bus M-cycle and frame return to the seed; full-memory/Timer/instance isolation; STOP-entry phase preservation.
- `system::tests::ppu_timing` (4): per-M-cycle comparison with a reference Bus (LCDC stores applied after the cycle's four ticks), including zero elapsed dots after an enabling LDH store; budget-pause equivalence; HALT idles and interrupt entries (asserted counts); stopped polls add no dots and phase resumes after wake. They use a `#[cfg(test)]` timing probe because System tests cannot name private PPU types.

Four existing tests migrated FF40 from fallback to LCDC (startup 91). Tests verify the agreed provisional schedule, not hardware timing. A mutation check reintroducing the two review-found defects (VBlank only on line 144; enable-edge test never true) was not run, so their detection rests on reasoning about the every-dot and all-byte tests. The [passive LCD](DEVICES.md#passive-lcd-data-registers) and [video-memory](DEVICES.md#video-memory-foundation) tests remain. No PPU rendering or hardware timing result is claimed. The [M3 review](M3_COMPLETION_REVIEW.md) retains its earlier f0d38ef evidence, including the corrected rapid_toggle result in [timer evidence](testing/MOONEYE_TIMER.md).

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
| reg_f | Passed | 183,194 | 732,776 | [Provenance/results](testing/MOONEYE_REG_F.md), [manifest](../tests/fixtures/mooneye-reg-f.json) |
| if_ie_registers | Passed | 183,364 | 733,456 | [Subset audit](testing/MOONEYE_CPU_SUBSET.md), [manifest](../tests/fixtures/mooneye-cpu-subset.json) |
| interrupts/ie_push | Passed | 179,867 | 719,468 | Same subset; LY waits complete since #75 |
| ei_sequence | Passed | 183,204 | 732,816 | Same subset |
| div_timing | Passed | 183,275 | 733,100 | Same subset; resets DIV explicitly |
| Thirteen timer fixtures | All passed | Per-fixture counts | Per-fixture counts | [Timer audit/results](testing/MOONEYE_TIMER.md), [manifest](../tests/fixtures/mooneye-timer.json) |

Every fixture matched the first FF01 packet and B/C/D/E/H/L decimal signature 3,5,8,13,21,34. Corrected rapid_toggle took 4605 M-cycles / 18420 ticks at M3 (183,203 since the #75 on-screen reporter); its earlier failure is preserved in the timer record. Caps remain 1,000,000 M-cycles and 10 host seconds per run, including a separately bounded diagnostic rerun on rapid_toggle failure. These results are finite checks, not complete timing/serial/PPU accuracy. External DAA [#52](https://github.com/Lorithan/gameboy-emu/issues/52) remains unrun and separate from the completed M3 gate.

## Runner contract

tests/support/rom_runner.rs observes public traces, with no production test special cases. FF01 writes are observable actions although serial is unimplemented. Only use this observer for sources audited to have no earlier SB reports and to select the common fast reporter under current FF44/FF02 fallback reads. Pinned sources: [reg_f](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/acceptance/bits/reg_f.s), [quit](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/quit.s), [PPU probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_ppu_broken.s), [serial probe](https://github.com/Gekkio/mooneye-test-suite/blob/31510e12eea6286d36eea060a6adde755e1067aa/common/lib/is_serial_broken.s).

The first six-byte packet plus register signature decides the result. Hex 42 in all six positions/registers is reported failure. Any other complete packet/register mismatch is malformed; partial output at timeout fails. Never scan past bad output for a later match. Other writes/incidental LD B,B bytes are not success.

Outcomes distinguish Passed, ReportedFailure, MalformedReport, TimedOut, UnexpectedStop and ExecutionError. Invalid/missing files fail setup. HALT consumes cycles; unexpected Stopped ends immediately rather than looping without a cycle budget advancing. Zero/exact budget boundaries and wall-time limits are tested. Do not silently increase caps to hide hangs.

Reports retain identity, packet, counts/ticks, snapshot and last 64 completed traces. Completion errors preserve/count the failed real cycle separately; request errors/stopped polls count zero. Host durations are diagnostics, not deterministic timing/performance promises. Eleven synthetic tests cover packet/signature/malformed/partial/wrong-address failures, loops/HALT/STOP, faults, limits, bounded history and file loading.

## Fixture policy

Before adding a fixture test, record source/revision, origin/SHA-256, accepted header, hardware/startup/device prerequisites, build/toolchain identity, component licenses, reporting and finite budgets. Keep ROM/archive binaries ignored under target/test-roms. Do not fetch moving latest data during tests or patch headers. Real bugs get isolated user-owned fixes and assistant regressions.

The [reg_f record](testing/MOONEYE_REG_F.md) preserves archive/build/MIT notices and separately attributed Darkrose font details; the whole ROM is not labeled exclusively MIT. No independent reproduced/signed build is claimed. Retain component provenance before redistribution. Other suites/flat-memory vectors need separate adapter/prerequisite/license audits.
