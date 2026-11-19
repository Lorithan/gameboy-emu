use super::divider_oracle::clocked_byte;
use crate::outcome_support::instruction_complete;
use crate::step_support::expect_cycle;

use super::super::System;
use crate::{cpu::CycleAction, memory::Cartridge};
#[path = "../../../tests/support/cb.rs"]
mod oracle;

#[test]
fn cb_memory_variants_change_only_completed_target_and_echo_alias() {
    for opcode in (6u8..=255).step_by(8) {
        for target in [0u16, 0x8000, 0xC123, 0xDFFF, 0xE123, 0xFF80, 0xFFFE, 0xFFFF] {
            let cycles = if (0x40..0x80).contains(&opcode) { 3 } else { 4 };
            for budget in 0..=cycles {
                let mut rom = vec![0; 32768];
                rom[0] = 0x81;
                rom[0x14D] = 0xE7;
                rom[0x100..0x107].copy_from_slice(&[
                    0x21,
                    target as u8,
                    (target / 256) as u8,
                    0xCB,
                    opcode,
                    0,
                    0xD3,
                ]);
                let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
                system.step_instruction(100).unwrap();
                for addr in 0xC000..=0xDFFF {
                    system.memory.write(addr, (addr % 251) as u8);
                }
                for addr in 0xFF80..=0xFFFE {
                    system.memory.write(addr, (addr % 127) as u8);
                }
                let original: Vec<_> = (0u16..=65535)
                    .map(|addr| system.memory.read(addr))
                    .collect();
                let result = oracle::result(opcode, original[usize::from(target)], 0xB0).0;
                system.step_instruction(budget).unwrap();
                for phase in [budget, cycles] {
                    let mut expected = original.clone();
                    if cycles == 4
                        && phase == 4
                        && matches!(target, 0x8000..=0x9FFF | 0xC000..=0xFDFF | 0xFE00..=0xFE9F | 0xFF80..=0xFFFF)
                    {
                        expected[usize::from(target)] = result;
                        if (0xC000..=0xDDFF).contains(&target) {
                            expected[usize::from(target + 0x2000)] = result;
                        }
                        if (0xE000..=0xFDFF).contains(&target) {
                            expected[usize::from(target - 0x2000)] = result;
                        }
                    }
                    for addr in 0u16..=65535 {
                        assert_eq!(
                            system.memory.read(addr),
                            clocked_byte(expected[usize::from(addr)], addr, system.elapsed_ticks()),
                            "CB {opcode:02X}, target {target:04X}, phase {phase}, changed {addr:04X}"
                        );
                    }
                    if phase < cycles {
                        // Trace resumed cycles to prove exactly one write, or zero for BIT.
                        let mut writes = 0;
                        for remaining in phase..cycles {
                            let trace = system.step_machine_cycle().map(expect_cycle).unwrap();
                            if matches!(trace.record.action, CycleAction::Write { .. }) {
                                writes += 1;
                            }
                            assert_eq!(
                                instruction_complete(trace.outcome),
                                remaining + 1 == cycles
                            );
                        }
                        assert_eq!(writes, u8::from(cycles == 4));
                    }
                }
            }
        }
    }
}
