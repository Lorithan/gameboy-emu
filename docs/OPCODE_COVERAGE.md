# Opcode coverage

All **500 legal instruction encodings** are implemented and covered by focused tests: 244 legal non-prefix base encodings plus 256 CB encodings. CB is a base dispatch byte, not another instruction in that count.

Base TryFrom rejects exactly `D3 DB DD E3 E4 EB EC ED F4 FC FD`; CB decoding is infallible. CPU uses sticky diagnostic faults with original byte/address metadata for illegal fetches, not physical lockup emulation.

Tests cover loads, arithmetic/logical/DAA/flags, all CB register/memory families, stack/control flow, DI/EI/RETI/HALT/STOP, captured operands, wrapping, retries, cycle actions and bounded stepping. Encoding coverage does not establish every physical timing or future device interaction.

[M2 decision](M2_COMPLETION_REVIEW.md) is for the declared functional scope. [Five actual ROM passes](TESTING.md) add narrow independent evidence. External DAA #52 is unrun; authentic boot, physical STOP, PPU/DMA and other devices remain separate.

Owning source: src/cpu/opcode.rs, dispatch.rs, execution.rs, alu.rs. Independent decode and family tests live under src/cpu and tests, including tests/support/cb.rs. Reference: [gbdev opcode data](https://raw.githubusercontent.com/gbdev/gb-opcodes/master/Opcodes.json).
