use crate::cpu::{
    CPU, CpuError, CycleAction, Registers,
    opcode::{DecodedOpcode, IndirectPair},
};
const CASES: [(u8, IndirectPair, bool); 4] = [
    (0x02, IndirectPair::BC, true),
    (0x0A, IndirectPair::BC, false),
    (0x12, IndirectPair::DE, true),
    (0x1A, IndirectPair::DE, false),
];
fn machine(pair: IndirectPair, addr: u16, a: u8, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(u16::from(a) * 256 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = 0xDEF0;
    r.pc = pc;
    match pair {
        IndirectPair::BC => r.set_bc(addr),
        IndirectPair::DE => r.set_de(addr),
    }
    CPU::from_registers(r)
}
#[test]
fn indirect_pair_mappings_are_explicit() {
    for (opcode, pair, store) in CASES {
        assert_eq!(
            DecodedOpcode::from(opcode),
            if store {
                DecodedOpcode::StoreAToIndirect { destination: pair }
            } else {
                DecodedOpcode::LoadAFromIndirect { source: pair }
            }
        );
    }
}
#[test]
fn indirect_pair_all_addresses_preserve_flags_and_unrelated_registers() {
    for (opcode, pair, store) in CASES {
        for addr in 0u16..=65535 {
            let value = (addr % 256) as u8;
            let mut cpu = machine(pair, addr, value, ((addr % 16) * 16) as u8, 0x0100);
            let mut expected = cpu.snapshot();
            expected.pc += 1;
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
                let read = 255 - value;
                assert!(cpu.complete_cycle(Some(read)).unwrap());
                expected.af = u16::from(read) * 256 + (expected.af & 0xF0);
            }
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0x0101 })
            ));
        }
    }
}
#[test]
fn indirect_pair_captures_address_and_store_data_with_atomic_retries_and_pc_wrap() {
    for (opcode, pair, store) in CASES {
        for flags in (0u8..=0xF0).step_by(16) {
            let mut cpu = machine(pair, 0xC123, 0x5A, flags, 0xFFFF);
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
            match pair {
                IndirectPair::BC => cpu.registers.set_bc(0xD234),
                IndirectPair::DE => cpu.registers.set_de(0xD234),
            }
            cpu.registers.a = 0xA5;
            let changed = cpu.snapshot();
            for _ in 0..3 {
                if store {
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(CycleAction::Write {
                            addr: 0xC123,
                            value: 0x5A
                        })
                    ));
                } else {
                    assert!(matches!(
                        cpu.request_cycle(),
                        Ok(CycleAction::Read { addr: 0xC123 })
                    ));
                    assert!(matches!(
                        cpu.complete_cycle(None),
                        Err(CpuError::MissingReadValue { addr: 0xC123 })
                    ));
                }
                assert_eq!(cpu.snapshot(), changed);
            }
            let mut expected = changed;
            if store {
                assert!(cpu.complete_cycle(None).unwrap());
            } else {
                assert!(cpu.complete_cycle(Some(0x69)).unwrap());
                expected.af = 0x6900 + u16::from(flags);
            }
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
