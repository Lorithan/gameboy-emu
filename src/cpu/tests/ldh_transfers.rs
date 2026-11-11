use crate::cpu::{
    CPU, CpuError, CycleAction, Registers,
    opcode::{DecodedOpcode, HighMemoryAddressSource},
};
const CASES: [(u8, bool, bool); 4] = [
    (0xE2, true, false),
    (0xF2, false, false),
    (0xE0, true, true),
    (0xF0, false, true),
];
fn machine(offset: u8, a: u8, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(u16::from(a) * 256 + u16::from(flags));
    r.set_bc(0x1200 + u16::from(offset));
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = 0xDEF0;
    r.pc = pc;
    CPU::from_registers(r)
}
#[test]
fn ldh_mappings_are_independent() {
    for (opcode, store, immediate) in CASES {
        let selector = if immediate {
            HighMemoryAddressSource::Immediate
        } else {
            HighMemoryAddressSource::C
        };
        assert_eq!(
            DecodedOpcode::from(opcode),
            if store {
                DecodedOpcode::StoreHighMemory {
                    address_source: selector,
                }
            } else {
                DecodedOpcode::LoadHighMemory {
                    address_source: selector,
                }
            }
        );
    }
}
#[test]
fn ldh_all_offsets_and_bytes_use_the_high_page_and_preserve_flags() {
    for (opcode, store, immediate) in CASES {
        for offset in 0u8..=255 {
            for value in 0u8..=255 {
                let mut cpu = machine(offset, value, value & 0xF0, 0x0100);
                let mut expected = cpu.snapshot();
                assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                expected.pc += 1;
                assert_eq!(cpu.snapshot(), expected);
                if immediate {
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(CycleAction::Read { addr: 0x0101 })
                    ));
                    assert!(!cpu.complete_cycle(Some(offset)).unwrap());
                    expected.pc += 1;
                    assert_eq!(cpu.snapshot(), expected);
                }
                let address = 0xFF00 + u16::from(offset);
                if store {
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr, value: actual }) if addr == address && actual == value)
                    );
                    assert!(cpu.complete_cycle(None).unwrap());
                } else {
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == address)
                    );
                    assert!(cpu.complete_cycle(Some(255 - value)).unwrap());
                    expected.af = u16::from(255 - value) * 256 + (expected.af & 0xF0);
                }
                assert_eq!(cpu.snapshot(), expected);
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                );
            }
        }
    }
}
#[test]
fn ldh_captured_address_and_store_data_survive_retries_and_pc_wrap() {
    for (opcode, store, immediate) in CASES {
        for flags in (0u8..=0xF0).step_by(16) {
            for pc in [0x0100u16, 0xFFFE, 0xFFFF] {
                let mut cpu = machine(0x80, 0x5A, flags, pc);
                let mut expected = cpu.snapshot();
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pc)
                );
                assert_eq!(cpu.snapshot(), expected);
                assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                expected.pc = expected.pc.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
                cpu.registers.a = 0xA5;
                cpu.registers.c = 0xFE;
                expected.af = 0xA500 + u16::from(flags);
                expected.bc = 0x12FE;
                if immediate {
                    for _ in 0..2 {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == expected.pc)
                        );
                        assert!(
                            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == expected.pc)
                        );
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    assert!(!cpu.complete_cycle(Some(0x80)).unwrap());
                    expected.pc = expected.pc.wrapping_add(1);
                    assert_eq!(cpu.snapshot(), expected);
                }
                for _ in 0..2 {
                    if store {
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Write {
                                addr: 0xFF80,
                                value: 0x5A
                            })
                        ));
                    } else {
                        assert!(matches!(
                            cpu.request_cycle(),
                            Ok(CycleAction::Read { addr: 0xFF80 })
                        ));
                        assert!(matches!(
                            cpu.complete_cycle(None),
                            Err(CpuError::MissingReadValue { addr: 0xFF80 })
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
                assert!(cpu.complete_cycle(Some(0)).unwrap());
                expected.pc = expected.pc.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}
