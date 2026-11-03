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
fn every_memory_operand_has_three_exact_cycles_and_can_be_reloaded() {
    for (opcode, delta) in [(0x34, 1i16), (0x35, -1)] {
        for old in 0u8..=255 {
            let mut system = program(&[0x21, 0x23, 0xC1, 0x36, old, opcode, 0x7E, 0xD3]);
            system.step_instruction(3).expect("HL setup");
            system.step_instruction(3).expect("memory setup");
            let mut expected = system.cpu_snapshot();
            let value = (i16::from(old) + delta).rem_euclid(256) as u8;
            let half = !(0..16).contains(&(i16::from(old % 16) + delta));
            let flags = 0x10u16
                | if value == 0 { 0x80 } else { 0 }
                | if delta < 0 { 0x40 } else { 0 }
                | if half { 0x20 } else { 0 };
            let fetch = system.step_machine_cycle().expect("opcode read");
            assert!(matches!(fetch.action, CycleAction::Read { addr: 0x0105 }));
            assert_eq!(fetch.read_value, Some(opcode));
            assert_eq!((fetch.tick_before, fetch.tick_after), (24, 28));
            assert!(!fetch.complete);
            expected.pc += 1;
            assert_eq!(system.cpu_snapshot(), expected);
            let read = system.step_machine_cycle().expect("data read");
            assert!(matches!(read.action, CycleAction::Read { addr: 0xC123 }));
            assert_eq!(read.read_value, Some(old));
            assert_eq!((read.tick_before, read.tick_after), (28, 32));
            assert!(!read.complete);
            expected.af = (expected.af & 0xFF00) | flags;
            assert_eq!(system.cpu_snapshot(), expected);
            let write = system.step_machine_cycle().expect("data write");
            assert!(
                matches!(write.action, CycleAction::Write { addr: 0xC123, value: actual } if actual == value)
            );
            assert_eq!(write.read_value, None);
            assert_eq!((write.tick_before, write.tick_after), (32, 36));
            assert!(write.complete);
            assert_eq!(system.cpu_snapshot(), expected);
            assert!(matches!(
                system.step_instruction(100),
                Ok(InstructionStep::Complete {
                    mcycles: 2,
                    ticks: 8
                })
            ));
            expected.af = u16::from(value) << 8 | flags;
            expected.pc += 1;
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 44);
        }
    }
}

#[test]
fn mixed_register_and_memory_arithmetic_keep_flags_through_transfers() {
    let mut system = program(&[
        0x21, 0x23, 0xC1, // HL=C123
        0x36, 0x0F, // (HL)=0F
        0x34, // INC (HL) -> 10, F=30
        0x7E, // A=10, preserve F
        0x35, // DEC (HL) -> 0F, F=70
        0x7E, // A=0F, preserve F
        0x04, // INC B -> 01, F=10
        0x77, // store A, preserve F
        0x00, 0xD3,
    ]);
    for (cycles, flags) in [
        (3, 0xB0),
        (3, 0xB0),
        (3, 0x30),
        (2, 0x30),
        (3, 0x70),
        (2, 0x70),
        (1, 0x10),
        (2, 0x10),
        (1, 0x10),
    ] {
        assert!(
            matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles && ticks == cycles * 4)
        );
        assert_eq!(system.cpu_snapshot().af & 0xFF, flags);
    }
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: 0x0F10,
            bc: 0x0113,
            de: 0x00D8,
            hl: 0xC123,
            pc: 0x010C,
            sp: 0xFFFE
        }
    );
    assert_eq!(system.elapsed_ticks(), 80);
}
