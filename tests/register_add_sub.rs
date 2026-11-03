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

#[test]
fn add_sub_boundaries_have_one_public_cycle_and_stop_before_the_next_opcode() {
    let opcodes = [
        (0x06, 0x80, 0x90),
        (0x0E, 0x81, 0x91),
        (0x16, 0x82, 0x92),
        (0x1E, 0x83, 0x93),
        (0x26, 0x84, 0x94),
        (0x2E, 0x85, 0x95),
        (0x3E, 0x87, 0x97),
    ];
    // left, right, ADD result/flags, SUB result/flags: independent examples.
    let examples: [(u8, u8, u8, u8, u8, u8); 7] = [
        (0xFF, 0x01, 0x00, 0xB0, 0xFE, 0x40),
        (0x0F, 0x01, 0x10, 0x20, 0x0E, 0x40),
        (0x10, 0x01, 0x11, 0x00, 0x0F, 0x60),
        (0x00, 0x01, 0x01, 0x00, 0xFF, 0x70),
        (0x42, 0x42, 0x84, 0x00, 0x00, 0xC0),
        (0x80, 0x80, 0x00, 0x90, 0x00, 0xC0),
        (0xFF, 0xFF, 0xFE, 0x30, 0x00, 0xC0),
    ];
    for (load, add, sub) in opcodes {
        for (left, right, add_result, add_flags, sub_result, sub_flags) in examples {
            if load == 0x3E && left != right {
                continue;
            }
            for (opcode, result, flags) in
                [(add, add_result, add_flags), (sub, sub_result, sub_flags)]
            {
                for budget in [0, 1, 100] {
                    let mut system = program(&[0x3E, left, load, right, opcode, 0xD3]);
                    system.step_instruction(2).expect("left operand");
                    system.step_instruction(2).expect("right operand");
                    let before = system.cpu_snapshot();
                    let mut expected = before;
                    expected.af = u16::from(result) << 8 | u16::from(flags);
                    expected.pc += 1;
                    if budget == 0 {
                        assert!(matches!(
                            system.step_instruction(0),
                            Ok(InstructionStep::BudgetExhausted {
                                mcycles: 0,
                                ticks: 0
                            })
                        ));
                        assert_eq!(system.cpu_snapshot(), before);
                        assert_eq!(system.elapsed_ticks(), 16);
                        let trace = system.step_machine_cycle().expect("arithmetic opcode");
                        assert!(matches!(trace.action, CycleAction::Read { addr: 0x0104 }));
                        assert_eq!(trace.read_value, Some(opcode));
                        assert_eq!((trace.tick_before, trace.tick_after), (16, 20));
                        assert!(trace.complete);
                    } else {
                        assert!(matches!(
                            system.step_instruction(budget),
                            Ok(InstructionStep::Complete {
                                mcycles: 1,
                                ticks: 4
                            })
                        ));
                    }
                    assert_eq!(system.cpu_snapshot(), expected);
                    assert_eq!(system.elapsed_ticks(), 20);
                }
            }
        }
    }
}

#[test]
fn mixed_binary_arithmetic_and_inc_dec_preserve_flags_across_loads_and_stores() {
    let mut system = program(&[
        0x21, 0x23, 0xC1, // HL=C123
        0x36, 0x0F, // (HL)=0F
        0x34, // INC (HL) -> 10, F=30
        0x06, 0xF0, // B=F0
        0x7E, // A=10
        0x80, // ADD A,B -> 00, F=90
        0x90, // SUB A,B -> 10, F=50
        0x77, // (HL)=10
        0x05, // DEC B -> EF, F=70
        0x35, // DEC (HL) -> 0F, F=70
        0x4E, // C=(HL) -> 0F
        0x00, 0xD3,
    ]);
    for (cycles, flags) in [
        (3, 0xB0),
        (3, 0xB0),
        (3, 0x30),
        (2, 0x30),
        (2, 0x30),
        (1, 0x90),
        (1, 0x50),
        (2, 0x50),
        (1, 0x70),
        (3, 0x70),
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
            bc: 0xEF0F,
            de: 0x00D8,
            hl: 0xC123,
            pc: 0x0110,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 96);
}
