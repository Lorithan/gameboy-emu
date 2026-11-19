use super::divider_oracle::clocked_byte;
use crate::{
    cpu::{CycleAction, CycleOutcome},
    memory::Cartridge,
    step_support::expect_cycle,
    system::{InstructionStep, System},
};

#[test]
fn word_stores_cross_video_memory_boundaries_with_exact_traces_and_pause_budgets() {
    // Independent expected physical bytes: ROM/unmapped writes are omitted;
    // FDFF also writes its DDFF echo partner. No oracle calls production routing.
    type ExpectedWrite = (u64, u16, u8); // completion phase, address, byte
    let cases: &[(u16, &[ExpectedWrite])] = &[
        (0x7FFF, &[(5, 0x8000, 0xAB)]),
        (0x9FFF, &[(4, 0x9FFF, 0xCD)]),
        (
            0xFDFF,
            &[(4, 0xFDFF, 0xCD), (4, 0xDDFF, 0xCD), (5, 0xFE00, 0xAB)],
        ),
        (0xFE9F, &[(4, 0xFE9F, 0xCD)]),
    ];
    for &(addr, changes) in cases {
        for budget in 0..=5 {
            let mut rom = vec![0; 32768];
            rom[0x14D] = 0xE7;
            rom[0x100..0x107].copy_from_slice(&[
                0x31,
                0xCD,
                0xAB, // LD SP,ABCD
                0x08,
                addr as u8,
                (addr / 256) as u8, // LD (a16),SP
                0xD3,
            ]);
            let mut s = System::new(Cartridge::from_bytes(&rom).unwrap());
            assert!(matches!(
                s.step_instruction(3).unwrap(),
                InstructionStep::Complete {
                    mcycles: 3,
                    ticks: 12
                }
            ));
            let before = s.cpu_snapshot();
            let original: Vec<_> = (0u16..=u16::MAX).map(|a| s.memory.read(a)).collect();
            let result = s.step_instruction(budget).unwrap();
            if budget < 5 {
                assert!(
                    matches!(result, InstructionStep::BudgetExhausted { mcycles, ticks }
                    if mcycles == budget && ticks == budget * 4)
                );
            } else {
                assert!(matches!(
                    result,
                    InstructionStep::Complete {
                        mcycles: 5,
                        ticks: 20
                    }
                ));
            }
            for phase in budget..=5 {
                let mut expected = original.clone();
                for &(write_phase, target, value) in changes {
                    if phase >= write_phase {
                        expected[usize::from(target)] = value;
                    }
                }
                for address in 0u16..=u16::MAX {
                    assert_eq!(
                        s.memory.read(address),
                        clocked_byte(expected[usize::from(address)], address, s.elapsed_ticks()),
                        "word at {addr:04X}, phase {phase}, memory {address:04X}"
                    );
                }
                let mut expected_cpu = before;
                expected_cpu.pc += phase.min(3) as u16;
                assert_eq!(s.cpu_snapshot(), expected_cpu);
                assert_eq!(s.elapsed_ticks(), 12 + phase * 4);
                s.step_instruction(0).unwrap();
                assert_eq!(s.cpu_snapshot(), expected_cpu);
                assert_eq!(s.elapsed_ticks(), 12 + phase * 4);
                if phase == 5 {
                    break;
                }
                let trace = expect_cycle(s.step_machine_cycle().unwrap());
                let (action, read_value) = match phase {
                    0..=2 => (
                        CycleAction::Read {
                            addr: 0x103 + phase as u16,
                        },
                        Some([0x08, addr as u8, (addr / 256) as u8][phase as usize]),
                    ),
                    3 => (CycleAction::Write { addr, value: 0xCD }, None),
                    _ => (
                        CycleAction::Write {
                            addr: addr + 1,
                            value: 0xAB,
                        },
                        None,
                    ),
                };
                assert_eq!(trace.record.action, action);
                assert_eq!(trace.record.read_value, read_value);
                assert_eq!(
                    (trace.record.tick_before, trace.record.tick_after),
                    (12 + phase * 4, 16 + phase * 4)
                );
                assert_eq!(
                    trace.outcome,
                    if phase == 4 {
                        CycleOutcome::InstructionComplete
                    } else {
                        CycleOutcome::InProgress
                    }
                );
            }
        }
    }
}
