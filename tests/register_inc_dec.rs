use gameboy_emu::{
    cpu::{CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"))
}

fn with_byte(mut snapshot: CpuSnapShot, index: usize, value: u8) -> CpuSnapShot {
    let value = u16::from(value);
    match index {
        0 => snapshot.bc = value << 8 | (snapshot.bc & 0xFF),
        1 => snapshot.bc = (snapshot.bc & 0xFF00) | value,
        2 => snapshot.de = value << 8 | (snapshot.de & 0xFF),
        3 => snapshot.de = (snapshot.de & 0xFF00) | value,
        4 => snapshot.hl = value << 8 | (snapshot.hl & 0xFF),
        5 => snapshot.hl = (snapshot.hl & 0xFF00) | value,
        6 => snapshot.af = value << 8 | (snapshot.af & 0xFF),
        _ => unreachable!(),
    }
    snapshot
}

#[test]
fn arithmetic_boundaries_have_exact_public_traces_flags_and_budgets() {
    let opcodes = [
        (0x06, 0x04, 0x05),
        (0x0E, 0x0C, 0x0D),
        (0x16, 0x14, 0x15),
        (0x1E, 0x1C, 0x1D),
        (0x26, 0x24, 0x25),
        (0x2E, 0x2C, 0x2D),
        (0x3E, 0x3C, 0x3D),
    ];
    // Independent boundary oracle: old, INC result/ZNH, DEC result/ZNH.
    let values: [(u8, u8, u8, u8, u8); 8] = [
        (0x00, 0x01, 0x00, 0xFF, 0x60),
        (0x01, 0x02, 0x00, 0x00, 0xC0),
        (0x0F, 0x10, 0x20, 0x0E, 0x40),
        (0x10, 0x11, 0x00, 0x0F, 0x60),
        (0x7F, 0x80, 0x20, 0x7E, 0x40),
        (0x80, 0x81, 0x00, 0x7F, 0x60),
        (0xFE, 0xFF, 0x00, 0xFD, 0x40),
        (0xFF, 0x00, 0xA0, 0xFE, 0x40),
    ];
    for (target, (load, inc, dec)) in opcodes.into_iter().enumerate() {
        for (old, inc_result, inc_flags, dec_result, dec_flags) in values {
            for (opcode, result, flags) in
                [(inc, inc_result, inc_flags), (dec, dec_result, dec_flags)]
            {
                let mut system = program(&[load, old, opcode, 0xD3]);
                system.step_instruction(2).expect("load input");
                let before = system.cpu_snapshot();
                let ticks = system.elapsed_ticks();
                assert!(matches!(
                    system.step_instruction(0),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    })
                ));
                assert_eq!(system.cpu_snapshot(), before);
                assert_eq!(system.elapsed_ticks(), ticks);
                let mut expected = with_byte(before, target, result);
                expected.af = (expected.af & 0xFF00) | u16::from(flags) | 0x10;
                expected.pc += 1;
                let trace = system.step_machine_cycle().expect("arithmetic opcode");
                assert!(matches!(trace.action, CycleAction::Read { addr: 0x0102 }));
                assert_eq!(trace.read_value, Some(opcode));
                assert_eq!((trace.tick_before, trace.tick_after), (ticks, ticks + 4));
                assert!(trace.complete);
                assert_eq!(system.cpu_snapshot(), expected);
                for budget in [1, 100] {
                    let mut system = program(&[load, old, opcode, 0xD3]);
                    system.step_instruction(2).expect("load input");
                    assert!(matches!(
                        system.step_instruction(budget),
                        Ok(InstructionStep::Complete {
                            mcycles: 1,
                            ticks: 4
                        })
                    ));
                    assert_eq!(system.cpu_snapshot(), expected);
                    assert_eq!(system.elapsed_ticks(), 12);
                }
            }
        }
    }
}

#[test]
fn mixed_arithmetic_and_transfers_preserve_the_latest_flags() {
    let mut system = program(&[
        0x21, 0x00, 0xC0, // HL=C000
        0x3E, 0x0F, // A=0F
        0x3C, // INC A -> 10, F=30
        0x77, // (HL)=10, preserve F
        0x47, // B=A, preserve F
        0x3D, // DEC A -> 0F, F=70
        0x7E, // A=(HL) -> 10, preserve F
        0x00, 0xD3,
    ]);
    for (cycles, flags) in [
        (3, 0xB0),
        (2, 0xB0),
        (1, 0x30),
        (2, 0x30),
        (1, 0x30),
        (1, 0x70),
        (2, 0x70),
        (1, 0x70),
    ] {
        assert!(
            matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
        );
        assert_eq!(system.cpu_snapshot().af & 0xFF, flags);
    }
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: 0x1070,
            bc: 0x1013,
            de: 0x00D8,
            hl: 0xC000,
            pc: 0x010B,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 52);
}
