use crate::cpu::{CPU, CpuError, CycleAction, Registers};

fn reference(old: u8, flags: u8, increment: bool) -> (u8, u8) {
    let delta = if increment { 1i16 } else { -1 };
    let value = (i16::from(old) + delta).rem_euclid(256) as u8;
    let half = !(0..16).contains(&(i16::from(old % 16) + delta));
    let flags = (flags & 0x10)
        | if value == 0 { 0x80 } else { 0 }
        | if increment { 0 } else { 0x40 }
        | if half { 0x20 } else { 0 };
    (value, flags)
}

#[test]
fn memory_inc_dec_exhaustively_preserve_registers_and_update_flags_at_the_read() {
    for (opcode, increment) in [(0x34, true), (0x35, false)] {
        for old in 0u8..=255 {
            for combination in 0u8..16 {
                let flags = combination << 4;
                let mut registers = Registers::dmg_post_boot(1);
                registers.set_af(0xA700 | u16::from(flags));
                registers.set_bc(0x1234);
                registers.set_de(0x5678);
                registers.set_hl(0xC123);
                registers.sp = 0xDEF0;
                let mut cpu = CPU::from_registers(registers);
                let mut expected = cpu.snapshot();
                assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode"));
                expected.pc += 1;
                assert_eq!(cpu.snapshot(), expected);
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0xC123 })
                ));
                assert!(!cpu.complete_cycle(Some(old)).expect("data read"));
                let (value, final_flags) = reference(old, flags, increment);
                expected.af = 0xA700 | u16::from(final_flags);
                assert_eq!(
                    cpu.snapshot(),
                    expected,
                    "opcode {opcode:02X}, old {old:02X}, flags {flags:02X}"
                );
                for _ in 0..2 {
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value: actual }) if actual == value)
                    );
                    assert_eq!(cpu.snapshot(), expected);
                }
                assert!(cpu.complete_cycle(None).expect("write completion"));
                assert_eq!(cpu.snapshot(), expected);
                assert!(matches!(
                    cpu.request_cycle(),
                    Ok(CycleAction::Read { addr: 0x0101 })
                ));
            }
        }
    }
}

#[test]
fn modification_read_retry_retains_operation_address_and_flags_after_pc_wrap() {
    for (opcode, increment) in [(0x34, true), (0x35, false)] {
        let mut registers = Registers::dmg_post_boot(1);
        registers.pc = 0xFFFF;
        registers.set_hl(0xC123);
        let mut cpu = CPU::from_registers(registers);
        let before = cpu.snapshot();
        assert!(matches!(
            cpu.complete_cycle(None),
            Err(CpuError::MissingReadValue { address: 0xFFFF })
        ));
        assert_eq!(cpu.snapshot(), before);
        assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode"));
        assert_eq!(cpu.snapshot().pc, 0);
        // Check the stored address independently of live HL. Current normal
        // execution does not modify HL while a data read is pending.
        cpu.registers.set_hl(0xD456);
        let mut expected = cpu.snapshot();
        for _ in 0..2 {
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0xC123 })
            ));
            assert!(matches!(
                cpu.complete_cycle(None),
                Err(CpuError::MissingReadValue { address: 0xC123 })
            ));
            assert_eq!(cpu.snapshot(), expected);
        }
        assert!(!cpu.complete_cycle(Some(0x0F)).expect("retry read"));
        let (value, flags) = reference(0x0F, 0xB0, increment);
        expected.af = 0x0100 | u16::from(flags);
        assert_eq!(cpu.snapshot(), expected);
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value: actual }) if actual == value)
        );
        assert!(cpu.complete_cycle(None).expect("write"));
        assert_eq!(cpu.snapshot(), expected);
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Read { addr: 0 })
        ));
    }
}
