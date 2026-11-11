use crate::cpu::{CPU, CpuError, CycleAction, Registers, opcode::DecodedOpcode};
fn machine(flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(0x5A00 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = 0xDEF0;
    r.pc = pc;
    CPU::from_registers(r)
}
#[test]
fn absolute_a_mappings_are_independent() {
    assert_eq!(DecodedOpcode::from(0xEA), DecodedOpcode::StoreAToAbsolute);
    assert_eq!(DecodedOpcode::from(0xFA), DecodedOpcode::LoadAFromAbsolute);
}
#[test]
fn absolute_a_every_target_address_uses_little_endian_without_early_effects() {
    for (opcode, store) in [(0xEA, true), (0xFA, false)] {
        for addr in 0u16..=65535 {
            let mut cpu = machine(((addr % 16) * 16) as u8, 0x0100);
            let mut expected = cpu.snapshot();
            for byte in [opcode, (addr % 256) as u8, (addr / 256) as u8] {
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                );
                assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                expected.pc += 1;
                assert_eq!(cpu.snapshot(), expected);
            }
            if store {
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: actual, value: 0x5A }) if actual == addr)
                );
                assert!(cpu.complete_cycle(None).unwrap());
            } else {
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr: actual }) if actual == addr)
                );
                let value = (addr % 256) as u8;
                assert!(cpu.complete_cycle(Some(value)).unwrap());
                expected.af = u16::from(value) * 256 + (expected.af & 0xF0);
            }
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0x0103 })
            ));
        }
    }
}
#[test]
fn absolute_a_read_retries_capture_store_data_and_low_byte_across_pc_wrap() {
    for (opcode, store) in [(0xEA, true), (0xFA, false)] {
        for flags in (0u8..=0xF0).step_by(16) {
            for pc in [0x0100u16, 0xFFFD, 0xFFFE, 0xFFFF] {
                let mut cpu = machine(flags, pc);
                let mut expected = cpu.snapshot();
                for (phase, byte) in [opcode, 0x34, 0xC1].into_iter().enumerate() {
                    for _ in 0..2 {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                        );
                        assert!(
                            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == expected.pc)
                        );
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    assert!(!cpu.complete_cycle(Some(byte)).unwrap());
                    expected.pc = expected.pc.wrapping_add(1);
                    assert_eq!(cpu.snapshot(), expected);
                    if phase == 0 {
                        cpu.registers.a = 0xA5;
                        expected.af = 0xA500 + u16::from(flags);
                    }
                }
                for _ in 0..2 {
                    if store {
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Write {
                                addr: 0xC134,
                                value: 0x5A
                            })
                        ));
                    } else {
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Read { addr: 0xC134 })
                        ));
                        assert!(matches!(
                            cpu.complete_cycle(None),
                            Err(CpuError::MissingReadValue { addr: 0xC134 })
                        ));
                    }
                    assert_eq!(cpu.snapshot(), expected);
                }
                if store {
                    assert!(cpu.complete_cycle(None).unwrap());
                } else {
                    assert!(cpu.complete_cycle(Some(0x69)).unwrap());
                    expected.af = 0x6900 + u16::from(flags);
                }
                assert_eq!(cpu.snapshot(), expected);
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pc.wrapping_add(3))
                );
                assert!(cpu.complete_cycle(Some(0)).unwrap());
                expected.pc = expected.pc.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}
