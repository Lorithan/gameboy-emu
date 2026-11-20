use crate::{
    cpu::{CycleAction, CycleOutcome},
    memory::Cartridge,
    step_support::expect_cycle,
    system::System,
};

#[test]
fn passive_lcd_cpu_round_trips_all_bytes_with_exact_actions_time_and_snapshots() {
    let registers = [
        (0x42u8, 0),
        (0x43, 0),
        (0x45, 0),
        (0x47, 0xFC),
        (0x48, 0xFF),
        (0x49, 0xFF),
        (0x4A, 0),
        (0x4B, 0),
    ];
    for (offset, _) in registers {
        for value in 0u8..=u8::MAX {
            // Set A, store, clear A, then load: readback must actually replace A.
            let bytes = [0x3E, value, 0xE0, offset, 0x3E, 0, 0xF0, offset];
            let mut rom = vec![0; 32768];
            rom[0x14D] = 0xE7;
            rom[0x100..0x108].copy_from_slice(&bytes);
            let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
            let initial = system.cpu_snapshot();
            let target = 0xFF00 + u16::from(offset);
            let actions = [
                CycleAction::Read { addr: 0x100 },
                CycleAction::Read { addr: 0x101 },
                CycleAction::Read { addr: 0x102 },
                CycleAction::Read { addr: 0x103 },
                CycleAction::Write {
                    addr: target,
                    value,
                },
                CycleAction::Read { addr: 0x104 },
                CycleAction::Read { addr: 0x105 },
                CycleAction::Read { addr: 0x106 },
                CycleAction::Read { addr: 0x107 },
                CycleAction::Read { addr: target },
            ];
            let read_values = [
                Some(0x3E),
                Some(value),
                Some(0xE0),
                Some(offset),
                None,
                Some(0x3E),
                Some(0),
                Some(0xF0),
                Some(offset),
                Some(value),
            ];
            let pc_offsets = [1, 2, 3, 4, 4, 5, 6, 7, 8, 8];
            for (index, action) in actions.into_iter().enumerate() {
                let trace = expect_cycle(system.step_machine_cycle().unwrap());
                assert_eq!(trace.record.action, action);
                assert_eq!(trace.record.read_value, read_values[index]);
                assert_eq!(
                    (trace.record.tick_before, trace.record.tick_after),
                    (index as u64 * 4, (index as u64 + 1) * 4)
                );
                assert_eq!(
                    trace.outcome,
                    if [1, 4, 6, 9].contains(&index) {
                        CycleOutcome::InstructionComplete
                    } else {
                        CycleOutcome::InProgress
                    }
                );
                let mut expected = initial;
                expected.pc += pc_offsets[index];
                let accumulator = match index {
                    0 => (initial.af >> 8) as u8,
                    1..=5 | 9 => value,
                    _ => 0,
                };
                expected.af = u16::from(accumulator) * 256 + (initial.af & 0xFF);
                assert_eq!(system.cpu_snapshot(), expected);
                for (probe, startup) in registers {
                    assert_eq!(
                        system.memory.read(0xFF00 + u16::from(probe)),
                        if probe == offset && index >= 4 {
                            value
                        } else {
                            startup
                        }
                    );
                }
                assert_eq!(
                    (system.memory.read(0xFF0F), system.memory.read(0xFFFF)),
                    (0xE1, 0)
                );
            }
            assert_eq!(system.elapsed_ticks(), 40);
        }
    }
}
