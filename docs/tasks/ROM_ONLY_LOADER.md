# Task #2 — ROM-only cartridge loader

## Goal
Load a supported ROM-only cartridge, preserve its bytes, and return useful errors for invalid or unsupported inputs. This is the first implementation task in **M1 — Cartridge and memory foundation**.

**Ready to start.** Completion of #1 is not a prerequisite for this isolated loader. Timing, post-boot state, CPU execution, and bus mapping remain separate work.

## Supported input and project policy
These are deliberately narrow loader policies, not a claim to reproduce the boot ROM's validation behavior.

- Accept exactly **32,768 bytes (32 KiB)**.
- Require cartridge type at `0x0147` to be `0x00` (ROM ONLY), ROM-size code at `0x0148` to be `0x00`, and RAM-size code at `0x0149` to be `0x00`.
- Accept conventional DMG and DMG-compatible CGB-enhanced headers (`0x0143 = 0x80`). Reject `0xC0` with an unsupported-CGB-only error. Other values are not rejected solely for this field; on older cartridges it belongs to the title. No GBC emulation is introduced.
- Validate the header checksum at `0x014D`, covering `0x0134..=0x014C` with 8-bit wrapping arithmetic. Reject a mismatch.
- Do not validate the Nintendo logo or global checksum in this task. Preserve those bytes unchanged. This intentionally accepts some images an authentic boot ROM would reject.
- Reject length mismatches rather than padding, truncating, or guessing a mapper. No archive extraction.
- Retain enough parsed metadata to inspect cartridge type, declared ROM size, and declared RAM size. Title/publisher decoding is deferred.
- Metadata size fields use decoded byte counts, not raw header codes: rom_size is 32,768 and ram_size is zero for supported input. The cartridge_type_code field retains the raw type code, zero. Header code 0x0148=0x00 encodes 32 KiB; it does not mean zero bytes.

## Acceptance criteria
- [ ] Validate an in-memory byte input independently of filesystem access.
- [ ] Load a raw file through a thin entry point; retain the underlying I/O failure context and path.
- [ ] Valid input yields inspectable metadata and unchanged ROM data, usable by the future bus.
- [ ] Bad input returns a recoverable error without panicking or yielding a usable cartridge.
- [ ] Errors distinguish I/O, incomplete header, unsupported hardware/size codes, actual-length mismatch, and header-checksum mismatch. Include relevant actual/expected values. Exact Rust API and error type design are the user's choice.
- [ ] Test the cases below with synthetic data, and document the loader policy.
- [ ] Keep SDL2, CPU/System scaffolding, memory writes, mappers, external RAM, and save files outside this task.

For inputs with multiple faults, no particular error precedence is required except that unavailable header bytes must not be indexed.

## Test cases
Begin with a **32,768-byte zero-filled buffer** and set `0x014D = 0xE7`. This is a valid input under our loader policy, not a bootable program. Its covered checksum bytes are all zero; the independently known checksum is `0xE7`.

| Case | Expected result |
| --- | --- |
| Baseline fixture | Accepted; type/ROM/RAM codes are zero; all bytes preserved |
| Distinct markers at offsets `0x0000`, `0x3FFF`, `0x4000`, `0x7FFF` | Accepted; each marker retained at its original offset |
| Lengths 0 and `0x014F` | Incomplete-header error; no panic |
| Length `0x0150` (full header), 32,767, 32,769, or 65,536; otherwise baseline header | Actual-length error; no silent resizing |
| Type `0x01`, `0x08`, or `0xFF`; other fields consistent | Unsupported cartridge error |
| ROM-size code `0x01` or `0xFF` in a 32 KiB buffer | Unsupported declared-ROM-size error |
| RAM-size code `0x02` or `0xFF` | Unsupported/inconsistent cartridge-RAM error |
| CGB flag `0x80`, checksum `0x67` | Accepted for DMG use |
| CGB flag `0xC0`, checksum `0x27` | Unsupported CGB-only error |
| Baseline checksum byte changed from `0xE7` to `0xE6` | Header-checksum error |
| Baseline byte `0x0134` changed from zero to one without updating checksum | Header-checksum error |
| Baseline fixture with an arbitrary logo and arbitrary global-checksum bytes | Accepted if all enforced checks still pass |
| Valid temporary raw file | Same data/metadata as in-memory loading |
| Nonexistent path | Recoverable I/O error with path/context |

When changing a covered header field, update its checksum unless the test specifically exercises checksum failure. Keep explicit checksum known-answer tests; do not derive every expectation using the production checksum routine. An unreadable-file check can be manual where reliable permission setup is impractical; record the result rather than assuming it passed.

## Review evidence
Run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`. Summarize observed results and any unverified I/O case. The user writes the emulator implementation; the assistant writes and maintains tests, reviews behavior, and runs checks.

## Reference
[Pan Docs: The Cartridge Header](https://github.com/gbdev/pandocs/blob/master/src/The_Cartridge_Header.md), especially CGB flag, cartridge type, ROM/RAM sizes, and checksums. Reference consulted 2026-11-02.

Choose the Rust API and module layout yourself. Start with the valid in-memory case, then rejection cases, then file loading.

## Initial test run — 2026-11-02

- `cargo test`: 15 passed, zero failed. Tests live in src/cartridge/tests.rs and use synthetic buffers and temporary files; no ROM downloads or new dependencies.
- `cargo fmt --check`: passed.
- Strict Clippy: fails on dead-code warnings in the normal binary because main does not yet use the cartridge API. No warning suppression or application logic changes were made.
- Short inputs are verified to return the current LengthError safely, with accurate lengths. The requested separate incomplete-header diagnostic is still missing; add its exact assertions when that API is implemented.
- Private ROM preservation is verified from unit tests. Public read-only ROM/metadata access is still missing and will need tests when introduced.
- Missing-file and directory-input I/O errors retain path and cause. On Windows the directory case may fail during open; an error after a successful open was not independently forced. Permission-specific read failure remains unverified.
- Issue #2 remains open pending these implementation/interface requirements and lint resolution.

## Metadata review — 2026-11-02

- Updated short-input tests to require the new IncompleteHeader diagnostic and added tests for rom() and metadata(). `cargo test`: 17 passed, one failed (18 total).
- The failing metadata test expects rom_size in bytes: 32,768. The implementation currently reports the header code, zero. RAM size zero happens to equal both the code and decoded size for the supported cartridge.
- Incomplete-header boundaries, accurate error lengths, and rom() preserving the full input including the header pass.
- Metadata fields are private, so consumers outside the cartridge module cannot inspect them; provide read-only accessors or public fields on the metadata value. Also correct the public naming typos minimun and cartidge_type_code; tests currently use the existing names and will be updated with the API.
- Formatting passes. Strict Clippy fails on unread private metadata fields in the normal binary.
- Emulator logic was left unchanged. The metadata test remains failing to preserve the expected behavior for the user to implement. Prior I/O coverage limitations still apply.

## Final local verification — 2026-11-02

- The user fixed decoded ROM size, naming, and metadata visibility. All acceptance criteria are met for the supported ROM-only scope.
- `cargo test`: 19 passed, zero failed. A sibling-module test confirms public metadata access and verifies that changing a snapshot does not change later metadata or ROM contents.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`: passed.
- The assistant added only tests and test-module wiring; emulator implementation remains the user's work.
- Issue #2 is complete on local evidence. Source/tests/docs remain uncommitted and unpublished at the time of this verification.
- Permission-specific read errors after a successful open were not independently forced on Windows. Existing missing-path/directory tests and inspection of both error mappings cover the current I/O contract; this limitation is retained explicitly.

## Published checkpoint

Implementation, tests, and documentation were committed in d03788d (Implement cartridge loading and verification). GitHub main was verified to match that checkpoint. The publication-pending status in the earlier local verification is resolved.
