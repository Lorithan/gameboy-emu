use crate::cpu::{CPU, CpuError, CycleAction, Registers, opcode::DecodedOpcode};
fn machine(hl: u16, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(0x5A00 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(hl);
    r.sp = 0xDEF0;
    r.pc = pc;
    CPU::from_registers(r)
}
#[test]
fn load_sp_from_hl_mapping_is_explicit() {
    assert_eq!(DecodedOpcode::from(0xF9), DecodedOpcode::TransferHLtoSP);
}
#[test]
fn load_sp_from_hl_all_values_commit_on_idle_and_continue_to_next_opcode() {
    for hl in 0u16..=65535 {
        let mut cpu = machine(hl, ((hl % 16) * 16) as u8, 0x0100);
        let mut expected = cpu.snapshot();
        expected.pc += 1;
        assert!(!cpu.complete_cycle(Some(0xF9)).unwrap());
        assert_eq!(cpu.snapshot(), expected);
        assert!(matches!(cpu.request_cycle(), Ok(CycleAction::Idle)));
        assert!(cpu.complete_cycle(None).unwrap());
        expected.sp = hl;
        assert_eq!(cpu.snapshot(), expected);
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Read { addr: 0x0101 })
        ));
        assert!(cpu.complete_cycle(Some(0)).unwrap());
        expected.pc += 1;
        assert_eq!(cpu.snapshot(), expected);
    }
}
#[test]
fn load_sp_from_hl_captures_source_with_all_flags_retries_and_pc_wrap() {
    for hl in [0u16, 1, 0x00FF, 0x0100, 0x7FFF, 0xFFFE, 0xFFFF] {
        for flags in (0u8..=0xF0).step_by(16) {
            let mut cpu = machine(hl, flags, 0xFFFF);
            let before = cpu.snapshot();
            assert!(matches!(
                cpu.complete_cycle(None),
                Err(CpuError::MissingReadValue { addr: 0xFFFF })
            ));
            assert_eq!(cpu.snapshot(), before);
            assert!(!cpu.complete_cycle(Some(0xF9)).unwrap());
            let mut pending = before;
            pending.pc = 0;
            assert_eq!(cpu.snapshot(), pending);
            cpu.registers.set_hl(0x4321);
            let changed = cpu.snapshot();
            for _ in 0..3 {
                assert!(matches!(cpu.request_cycle(), Ok(CycleAction::Idle)));
                assert_eq!(cpu.snapshot(), changed);
            }
            assert!(cpu.complete_cycle(None).unwrap());
            let mut expected = changed;
            expected.sp = hl;
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0 })
            ));
        }
    }
}
