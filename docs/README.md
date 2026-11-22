# Documentation

Start with the [project plan](PROJECT_PLAN.md). M3 is complete within its [reviewed scope](M3_COMPLETION_REVIEW.md). M4 has verified [video memory](DEVICES.md#video-memory-foundation), [passive LCD registers](DEVICES.md#passive-lcd-data-registers), [LCDC and the dot clock](DEVICES.md#lcdc-and-internal-dot-clock), [live LY/STAT](DEVICES.md#live-ly-and-stat-readback), [STAT/VBlank interrupts](DEVICES.md#stat-and-vblank-interrupts), [VRAM/OAM access restrictions](DEVICES.md#vram-and-oam-access-restrictions) and [OAM DMA](DEVICES.md#oam-dma) ([contract](tasks/OAM_DMA.md), five Mooneye ROMs), all on a provisional model ([contracts](tasks/LCD_REGISTERS.md)). PPU timing is calibrated against ten Mooneye ROMs within the limits in the [calibration record](tasks/PPU_CALIBRATION.md). Next is [rendering into a framebuffer](tasks/RENDERING.md) (proposed).

| Guide | Contents |
| --- | --- |
| [Architecture](CORE_ARCHITECTURE.md) | Ownership, timing/order, cycle protocol, stepping and errors |
| [CPU](CPU.md) | Invariants, continuations, interrupts and HALT |
| [Devices/startup](DEVICES.md) | Memory, cartridge, DIV phase, JOYP and STOP limits |
| [Timer](TIMER.md) | Implemented registers, falling edges, reload/write phases and clock/interrupt integration |
| [Testing](TESTING.md) | Commands, runner, fixture requirements and current evidence |
| [Opcode coverage](OPCODE_COVERAGE.md) | Encoding counts and their limits |
| [M2 decision](M2_COMPLETION_REVIEW.md) | Completed milestone gate and follow-ups |
| [M3 decision](M3_COMPLETION_REVIEW.md) | Timer/interrupt gate, evidence and remaining limits |
| [reg_f evidence](testing/MOONEYE_REG_F.md) | Artifact/build/license audit and measured result |
| [CPU subset evidence](testing/MOONEYE_CPU_SUBSET.md) | Four additional fixture audits/results |
| [Timer ROM evidence](testing/MOONEYE_TIMER.md) | Thirteen audited fixtures pass after the fetch-boundary correction |

`tasks` holds active proposals; `testing` retains provenance and measurements. Completed specs, superseded reviews and checkpoint logs were removed from the working tree on 2026-11-19. They remain in Git history and [closed issues](https://github.com/Lorithan/gameboy-emu/issues?q=is%3Aissue+is%3Aclosed). Old issue/commit links to deleted task paths are historical references; current guides replace them.

Update guides in place, clearly label proposals/approximations, and preserve source/license/startup/accuracy evidence during future consolidation.
