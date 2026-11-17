use crate::cpu::{
    CPU, CpuError, CycleAction, CycleOutcome, ExecutionState, InterruptState, Registers,
};

#[test]
fn interrupt_acceptance_masks_prioritizes_and_rejects_without_mutation() {
    for pending in 0u8..=255 {
        for ime in [
            InterruptState::Disabled,
            InterruptState::Delayed,
            InterruptState::Enabled,
        ] {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
            cpu.interrupt_state = ime;
            let before = cpu.snapshot();
            let accepted = cpu.try_accept_interrupt(pending);
            if ime == InterruptState::Enabled && pending % 32 != 0 {
                let index = (0..5).find(|i| pending & (1 << i) != 0).unwrap();
                assert_eq!(accepted.unwrap().vector_address(), 0x40 + index * 8);
                assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
                assert!(matches!(cpu.state, ExecutionState::InterruptEntry { .. }));
                assert!(cpu.try_accept_interrupt(31).is_none());
            } else {
                assert!(accepted.is_none());
                assert!(matches!(cpu.state, ExecutionState::FetchOpcode));
                assert_eq!(cpu.interrupt_state, ime);
            }
            assert_eq!(cpu.snapshot(), before);
        }
    }
    // Reject while Enabled even in the middle of an instruction or a fault.
    for opcode in [0x3E, 0xCB, 0xCD, 0xC5, 0xD3] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
        let _ = cpu.complete_cycle(Some(opcode));
        cpu.interrupt_state = InterruptState::Enabled;
        let snapshot = cpu.snapshot();
        let state = format!("{:?}", cpu.state);
        assert!(cpu.try_accept_interrupt(31).is_none());
        assert_eq!(cpu.snapshot(), snapshot);
        assert_eq!(format!("{:?}", cpu.state), state);
        assert_eq!(cpu.interrupt_state, InterruptState::Enabled);
        if opcode == 0xD3 {
            assert!(matches!(
                cpu.request_cycle(),
                Err(CpuError::UnsupportedOpcode(_))
            ));
        }
    }
}

fn check_entry(pc: u16, sp: u16, mask: u8, initial_flags: u8) {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0xE7));
    for (flag, bit) in [
        (crate::cpu::registers::Flag::Z, 0x80),
        (crate::cpu::registers::Flag::N, 0x40),
        (crate::cpu::registers::Flag::H, 0x20),
        (crate::cpu::registers::Flag::C, 0x10),
    ] {
        cpu.registers.set_flag(flag, initial_flags & bit != 0);
    }
    cpu.registers.pc = pc;
    cpu.registers.sp = sp;
    cpu.interrupt_state = InterruptState::Enabled;
    let original = cpu.snapshot();
    let source = cpu.try_accept_interrupt(mask).unwrap();
    let vector = source.vector_address();
    let actions = [
        CycleAction::Idle,
        CycleAction::Idle,
        CycleAction::Write {
            addr: sp.wrapping_sub(1),
            value: (pc / 256) as u8,
        },
        CycleAction::Write {
            addr: sp.wrapping_sub(2),
            value: pc as u8,
        },
        CycleAction::Idle,
    ];
    for (cycle, action) in actions.into_iter().enumerate() {
        let before = cpu.snapshot();
        for _ in 0..2 {
            assert_eq!(cpu.request_cycle().unwrap(), action);
            assert_eq!(cpu.snapshot(), before);
            assert!(cpu.try_accept_interrupt(31).is_none());
        }
        let outcome = cpu.complete_cycle(None).unwrap();
        assert_eq!(
            outcome,
            if cycle == 4 {
                CycleOutcome::InterruptEntryComplete { vector }
            } else {
                CycleOutcome::InProgress
            }
        );
        let mut expected = original;
        if cycle >= 2 {
            expected.sp = sp.wrapping_sub(if cycle == 2 { 1 } else { 2 });
        }
        if cycle == 4 {
            expected.pc = vector;
        }
        assert_eq!(cpu.snapshot(), expected);
        assert_eq!(cpu.interrupt_state, InterruptState::Disabled);
    }
    assert_eq!(
        cpu.request_cycle().unwrap(),
        CycleAction::Read { addr: vector }
    );
    assert_eq!(
        cpu.complete_cycle(Some(0)).unwrap(),
        CycleOutcome::InstructionComplete
    );
    assert_eq!(cpu.snapshot().pc, vector + 1);
}

#[test]
fn interrupt_entry_exhausts_return_addresses_and_stack_wrapping_separately() {
    for value in 0u16..=65535 {
        check_entry(value, 0xFFFE, 4, 0xE7);
        check_entry(0xA53C, value, 4, 0);
    }
    for mask in [1, 2, 4, 8, 16] {
        for flags in 0..16 {
            check_entry(0x1234, 0xFFFE, mask, flags * 16);
        }
        for sp in [0, 1, 0x8000, 0xC002, 0xE002, 0xFF10, 0xFF80, 0xFFFF] {
            check_entry(0x1234, sp, mask, 0xE7);
        }
    }
}

#[test]
fn interrupt_entry_uses_captured_pc_and_stack_addresses() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    cpu.registers.pc = 0xA53C;
    cpu.registers.sp = 0;
    cpu.interrupt_state = InterruptState::Enabled;
    cpu.try_accept_interrupt(8).unwrap();
    cpu.registers.pc = 0xDEAD;
    cpu.registers.sp = 0xBEEF;
    for _ in 0..2 {
        assert_eq!(cpu.complete_cycle(None).unwrap(), CycleOutcome::InProgress);
    }
    assert_eq!(
        cpu.request_cycle().unwrap(),
        CycleAction::Write {
            addr: 0xFFFF,
            value: 0xA5
        }
    );
    cpu.complete_cycle(None).unwrap();
    cpu.registers.sp = 0x4321;
    assert_eq!(
        cpu.request_cycle().unwrap(),
        CycleAction::Write {
            addr: 0xFFFE,
            value: 0x3C
        }
    );
    cpu.complete_cycle(None).unwrap();
    assert_eq!(cpu.snapshot().sp, 0xFFFE);
    assert_eq!(cpu.snapshot().pc, 0xDEAD);
    assert_eq!(
        cpu.complete_cycle(None).unwrap(),
        CycleOutcome::InterruptEntryComplete { vector: 0x58 }
    );
    assert_eq!(cpu.snapshot().pc, 0x58);
}
