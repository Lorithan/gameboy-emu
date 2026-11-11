use crate::cpu::{CPU, CpuError, CycleAction, Registers, opcode::DecodedOpcode};
fn machine(sp: u16, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(0x6900 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = sp;
    r.pc = pc;
    CPU::from_registers(r)
}
#[test]
fn store_sp_absolute_mapping_is_explicit() {
    assert_eq!(DecodedOpcode::from(0x08), DecodedOpcode::StoreStackPointer);
}
fn check(sp: u16, target: u16) {
    let mut cpu = machine(sp, ((sp % 16) * 16) as u8, 0x0100);
    let mut expected = cpu.snapshot();
    for byte in [0x08, (target % 256) as u8, (target / 256) as u8] {
        assert!(!cpu.complete_cycle(Some(byte)).unwrap());
        expected.pc += 1;
        assert_eq!(cpu.snapshot(), expected);
    }
    assert!(
        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr, value }) if addr == target && value == (sp % 256) as u8)
    );
    assert!(!cpu.complete_cycle(None).unwrap());
    assert_eq!(cpu.snapshot(), expected);
    let next = ((u32::from(target) + 1) % 65536) as u16;
    assert!(
        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr, value }) if addr == next && value == (sp / 256) as u8)
    );
    assert!(cpu.complete_cycle(None).unwrap());
    assert_eq!(cpu.snapshot(), expected);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0103 })
    ));
    assert!(cpu.complete_cycle(Some(0)).unwrap());
    expected.pc += 1;
    assert_eq!(cpu.snapshot(), expected);
}
#[test]
fn store_sp_absolute_sweeps_all_source_values_and_all_target_addresses() {
    for value in 0u16..=65535 {
        check(value, 0xC123);
        check(0x5AA5, value);
    }
}
#[test]
fn store_sp_absolute_captures_sp_and_low_address_with_atomic_retries_and_pc_wrap() {
    for flags in (0u8..=0xF0).step_by(16) {
        for pc in [0x0100u16, 0xFFFD, 0xFFFE, 0xFFFF] {
            let mut cpu = machine(0x5AA5, flags, pc);
            let mut expected = cpu.snapshot();
            for (phase, byte) in [0x08, 0xFF, 0xFF].into_iter().enumerate() {
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
                    cpu.registers.sp = 0x4321;
                    expected.sp = 0x4321;
                }
            }
            for (phase, addr, value) in [(0, 0xFFFF, 0xA5), (1, 0, 0x5A)] {
                for _ in 0..3 {
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: actual, value: actual_value }) if actual == addr && actual_value == value)
                    );
                    assert_eq!(cpu.snapshot(), expected);
                }
                assert_eq!(cpu.complete_cycle(None).unwrap(), phase == 1);
                assert_eq!(cpu.snapshot(), expected);
            }
            assert!(
                matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pc.wrapping_add(3))
            );
        }
    }
}
