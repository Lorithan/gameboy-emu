use crate::{
    cpu::{CycleAction, CycleOutcome},
    memory::Cartridge,
    step_support::expect_cycle,
    system::System,
};

#[test]
fn timer_registers_round_trip_through_cpu_bus_cycles_with_exact_traces_and_time() {
    for offset in [5u8, 6, 7] {
        for value in 0u8..=255 {
            let mut rom = vec![0; 32768];
            rom[0x14D] = 0xE7;
            rom[0x100..0x106].copy_from_slice(&[0x3E, value, 0xE0, offset, 0xF0, offset]);
            let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
            let mut expected = system.cpu_snapshot();
            let readback = if offset == 7 { 248 + value % 8 } else { value };
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
                CycleAction::Read { addr: target },
            ];
            for (index, action) in actions.into_iter().enumerate() {
                let trace = expect_cycle(system.step_machine_cycle().unwrap());
                assert_eq!(trace.record.action, action);
                assert_eq!(
                    (trace.record.tick_before, trace.record.tick_after),
                    (index as u64 * 4, (index as u64 + 1) * 4)
                );
                assert_eq!(
                    trace.outcome,
                    if [1, 4, 7].contains(&index) {
                        CycleOutcome::InstructionComplete
                    } else {
                        CycleOutcome::InProgress
                    }
                );
                if index == 7 {
                    assert_eq!(trace.record.read_value, Some(readback));
                }
            }
            expected.af = u16::from(readback) * 256 + (expected.af & 0xFF);
            expected.pc = 0x106;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 32);
            assert_eq!(
                (system.memory.read(0xFF0F), system.memory.read(0xFFFF)),
                (0xE1, 0)
            );
        }
    }
}
