use crate::cpu::{
    CPU, CpuError, CycleAction, Registers,
    opcode::{DecodedOpcode, UnaryOperation},
};
const CASES: [(u8, bool, bool); 4] = [
    (0x22, true, true),
    (0x2A, false, true),
    (0x32, true, false),
    (0x3A, false, false),
];
fn machine(addr: u16, a: u8, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(u16::from(a) * 256 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(addr);
    r.sp = 0xDEF0;
    r.pc = pc;
    CPU::from_registers(r)
}
fn next(addr: u16, increment: bool) -> u16 {
    (i32::from(addr) + if increment { 1 } else { -1 }).rem_euclid(65536) as u16
}
#[test]
fn hl_auto_update_mappings_are_explicit() {
    for (byte, store, increment) in CASES {
        let operation = if increment {
            UnaryOperation::Increment
        } else {
            UnaryOperation::Decrement
        };
        assert_eq!(
            DecodedOpcode::from(byte),
            if store {
                DecodedOpcode::StoreAToHL { operation }
            } else {
                DecodedOpcode::LoadAFromHL { operation }
            }
        );
    }
}
#[test]
fn hl_auto_update_all_addresses_use_original_hl_and_preserve_flags() {
    for (opcode, store, increment) in CASES {
        for addr in 0u16..=65535 {
            let value = (addr % 256) as u8;
            let mut cpu = machine(addr, value, ((addr % 16) * 16) as u8, 0xFFFF);
            let before = cpu.snapshot();
            let mut expected = before;
            expected.pc = 0;
            assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
            assert_eq!(cpu.snapshot(), expected);
            if store {
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: actual, value: actual_value }) if actual == addr && actual_value == value)
                );
                assert!(cpu.complete_cycle(None).unwrap());
            } else {
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr: actual }) if actual == addr)
                );
                assert!(cpu.complete_cycle(Some(255 - value)).unwrap());
                expected.af = u16::from(255 - value) * 256 + (before.af & 0xF0);
            }
            expected.hl = next(addr, increment);
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0 })
            ));
            assert!(cpu.complete_cycle(Some(0)).unwrap());
            expected.pc = 1;
            assert_eq!(cpu.snapshot(), expected);
        }
    }
}
#[test]
fn hl_auto_update_captures_address_data_and_update_through_atomic_retries() {
    for (opcode, store, increment) in CASES {
        for flags in (0u8..=0xF0).step_by(16) {
            for addr in [0u16, 0x00FF, 0xC123, 0xFFFF] {
                let mut cpu = machine(addr, 0x5A, flags, 0xFFFF);
                let before = cpu.snapshot();
                assert!(matches!(
                    cpu.complete_cycle(None),
                    Err(CpuError::MissingReadValue { addr: 0xFFFF })
                ));
                assert_eq!(cpu.snapshot(), before);
                assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                let mut pending = before;
                pending.pc = 0;
                assert_eq!(cpu.snapshot(), pending);
                cpu.registers.set_hl(0xD234);
                cpu.registers.a = 0xA5;
                let changed = cpu.snapshot();
                for _ in 0..3 {
                    if store {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: actual, value: 0x5A }) if actual == addr)
                        );
                    } else {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr: actual }) if actual == addr)
                        );
                        assert!(
                            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr: actual }) if actual == addr)
                        );
                    }
                    assert_eq!(cpu.snapshot(), changed);
                }
                let mut expected = changed;
                expected.hl = next(addr, increment);
                if store {
                    assert!(cpu.complete_cycle(None).unwrap());
                } else {
                    assert!(cpu.complete_cycle(Some(0x69)).unwrap());
                    expected.af = 0x6900 + u16::from(flags);
                }
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}
