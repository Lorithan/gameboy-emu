use gameboy_emu::{
    cpu::{CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn machine(opcode: u8, value: u16) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0104].copy_from_slice(&[opcode, value as u8, (value >> 8) as u8, 0xD3]);
    System::new(Cartridge::from_bytes(&rom).expect("valid synthetic ROM"))
}

fn check_load(opcode: u8, set_expected: fn(&mut CpuSnapShot, u16)) {
    for value in [0x0000, 0x00FF, 0xFF00, 0xFFFF, 0x1234, 0x8001] {
        let mut system = machine(opcode, value);
        let mut expected = system.cpu_snapshot();
        for (index, byte) in [opcode, value as u8, (value >> 8) as u8]
            .into_iter()
            .enumerate()
        {
            let trace = system.step_machine_cycle().expect("word-load read cycle");
            assert!(
                matches!(trace.action, CycleAction::Read { addr } if addr == 0x0100 + index as u16)
            );
            assert_eq!(trace.read_value, Some(byte));
            assert_eq!(
                (trace.tick_before, trace.tick_after),
                (index as u64 * 4, (index as u64 + 1) * 4)
            );
            assert_eq!(trace.complete, index == 2);
            expected.pc += 1;
            if index == 2 {
                set_expected(&mut expected, value);
            }
            assert_eq!(system.cpu_snapshot(), expected);
        }

        // A larger budget must stop before the following unsupported opcode.
        let mut system = machine(opcode, value);
        assert!(matches!(
            system.step_instruction(100),
            Ok(InstructionStep::Complete {
                mcycles: 3,
                ticks: 12
            })
        ));
        assert_eq!(system.cpu_snapshot(), expected);

        for prefix in [1, 2] {
            let mut system = machine(opcode, value);
            assert!(
                matches!(system.step_instruction(prefix), Ok(InstructionStep::BudgetExhausted { mcycles, ticks }) if mcycles == prefix && ticks == prefix * 4)
            );
            let pending = system.cpu_snapshot();
            assert!(matches!(
                system.step_instruction(0),
                Ok(InstructionStep::BudgetExhausted {
                    mcycles: 0,
                    ticks: 0
                })
            ));
            assert_eq!(system.cpu_snapshot(), pending);
            assert!(
                matches!(system.step_instruction(3), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == 3 - prefix && ticks == (3 - prefix) * 4)
            );
            assert_eq!(system.cpu_snapshot(), expected);
        }
    }
}

#[test]
fn ld_bc_immediate_word() {
    check_load(0x01, |snapshot, value| snapshot.bc = value);
}

#[test]
fn ld_de_immediate_word() {
    check_load(0x11, |snapshot, value| snapshot.de = value);
}

#[test]
fn ld_hl_immediate_word() {
    check_load(0x21, |snapshot, value| snapshot.hl = value);
}

#[test]
fn ld_sp_immediate_word() {
    check_load(0x31, |snapshot, value| snapshot.sp = value);
}

#[test]
fn exact_budget_and_mixed_loads_preserve_instruction_boundaries() {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    let program = [
        0x01, 0x34, 0x12, 0x06, 0xAB, 0x00, 0x11, 0x78, 0x56, 0x1E, 0xCD, 0x21, 0xBC, 0x9A, 0x2E,
        0xEF, 0x31, 0x00, 0xC0, 0xD3,
    ];
    rom[0x0100..0x0100 + program.len()].copy_from_slice(&program);
    let mut system = System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"));
    let mut expected = system.cpu_snapshot();
    assert!(matches!(
        system.step_instruction(0),
        Ok(InstructionStep::BudgetExhausted {
            mcycles: 0,
            ticks: 0
        })
    ));
    assert_eq!(system.elapsed_ticks(), 0);
    assert_eq!(system.cpu_snapshot(), expected);
    for (cycles, pc, bc, de, hl, sp) in [
        (3, 0x0103, 0x1234, 0x00D8, 0x014D, 0xFFFE),
        (2, 0x0105, 0xAB34, 0x00D8, 0x014D, 0xFFFE),
        (1, 0x0106, 0xAB34, 0x00D8, 0x014D, 0xFFFE),
        (3, 0x0109, 0xAB34, 0x5678, 0x014D, 0xFFFE),
        (2, 0x010B, 0xAB34, 0x56CD, 0x014D, 0xFFFE),
        (3, 0x010E, 0xAB34, 0x56CD, 0x9ABC, 0xFFFE),
        (2, 0x0110, 0xAB34, 0x56CD, 0x9AEF, 0xFFFE),
        (3, 0x0113, 0xAB34, 0x56CD, 0x9AEF, 0xC000),
    ] {
        assert!(
            matches!(system.step_instruction(cycles), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
        );
        expected.pc = pc;
        expected.bc = bc;
        expected.de = de;
        expected.hl = hl;
        expected.sp = sp;
        assert_eq!(system.cpu_snapshot(), expected);
    }
    assert_eq!(system.elapsed_ticks(), 76);
}
