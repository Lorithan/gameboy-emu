# CPL, SCF and CCF

Issue: [#25](https://github.com/Lorithan/gameboy-emu/issues/25). Baseline: published cce8c4f, 226 tests passing.

## Contract

2F CPL inverts A, sets N/H to 1, and preserves Z/C. 37 SCF preserves A/Z, clears N/H and sets C. 3F CCF preserves A/Z, clears N/H and inverts the original C. All preserve other registers/SP/memory, consume one opcode byte and one M-cycle (four ticks), and wrap PC advancement once. No operand accesses, writes or idle cycles.

Dedicated decoded operations execute during opcode completion. No new pending states or Bus/System behavior is needed. Existing FetchOpcode state remains active on this entry path; completion reports true.

## Review evidence - 2026-11-06

No correctness findings. All 231 tests pass (161 unit, 70 integration), including:

- Independent three-byte mapping and updated all-byte unsupported classification.
- Every A byte and every initial flag combination at ordinary/wrapping PC (24,576 cases), missing-read atomic retries, immutable requests, unrelated register preservation and double application properties.
- Exact single opcode-fetch traces, zero/exact/excess instruction budgets and no extra fetch.
- Full-address-space preservation including mapped aliases.
- CPL-preserved Z feeding a taken branch; SCF/CCF feeding ADC/SBC and taken/untaken conditions in an 80-tick public program.

Formatting passes. Strict Clippy fails only on uppercase acronym variants CPL/SCF/CCF in opcode.rs. Remaining change: user rename these variants and their production references to Cpl/Scf/Ccf (or descriptive names); assistant will adapt the mapping tests. Do not suppress the lint. Keep issue open until strict Clippy passes. No external ROM validation was performed.

Assistant changed tests, test-module wiring and documentation only. Implementation remains user-owned.
## Completion - 2026-11-06

With user approval, the assistant renamed the enum variants/references to Cpl/Scf/Ccf and adapted the mapping tests. No execution behavior changed. Fresh full run: all 231 tests pass (161 unit, 70 integration); formatting, strict Clippy and diff whitespace checks pass. No correctness findings remain. This supersedes the lint blocker above. Complete locally; publication remains pending.
