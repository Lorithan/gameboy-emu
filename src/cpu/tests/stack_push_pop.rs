use crate::cpu::{
    CPU, CpuError, CycleAction, ExecutionState, Registers,
    opcode::{DecodedOpcode, PushOperand},
};

#[test]
fn push_bc_decodes_to_the_stack_pair() {
    assert!(matches!(
        DecodedOpcode::from(0xC5),
        DecodedOpcode::Push {
            operand: PushOperand::BC
        }
    ));
}

#[test]
fn all_stack_opcodes_select_the_independent_expected_pair() {
    for (push, pop, expected) in [
        (0xC5, 0xC1, PushOperand::BC),
        (0xD5, 0xD1, PushOperand::DE),
        (0xE5, 0xE1, PushOperand::HL),
        (0xF5, 0xF1, PushOperand::AF),
    ] {
        match DecodedOpcode::from(push) {
            DecodedOpcode::Push { operand } => assert_eq!(operand, expected),
            _ => panic!("incorrect PUSH mapping {push:02X}"),
        }
        match DecodedOpcode::from(pop) {
            DecodedOpcode::Pop { operand } => assert_eq!(operand, expected),
            _ => panic!("incorrect POP mapping {pop:02X}"),
        }
    }
}

#[test]
fn stack_opcodes_select_every_word_and_capture_push_sources_for_each_pair() {
    for (push, pop, pair) in [
        (0xC5, 0xC1, PushOperand::BC),
        (0xD5, 0xD1, PushOperand::DE),
        (0xE5, 0xE1, PushOperand::HL),
        (0xF5, 0xF1, PushOperand::AF),
    ] {
        for word in 0u16..=u16::MAX {
            let mut registers = Registers::dmg_post_boot(1);
            match pair {
                PushOperand::BC => registers.set_bc(word),
                PushOperand::DE => registers.set_de(word),
                PushOperand::HL => registers.set_hl(word),
                PushOperand::AF => registers.set_af(word),
            }
            let source = if pair == PushOperand::AF {
                word & 0xFFF0
            } else {
                word
            };
            let mut cpu = CPU::from_registers(registers);
            assert!(!cpu.complete_cycle(Some(push)).unwrap());
            // Alter all source pairs after decode to verify captured data.
            cpu.registers.set_bc(!word);
            cpu.registers.set_de(!word);
            cpu.registers.set_hl(!word);
            cpu.registers.set_af(!word);
            let mut expected = cpu.snapshot();
            assert!(!cpu.complete_cycle(None).unwrap());
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr: 0xFFFD, value } if value == (source / 256) as u8)
            );
            assert!(!cpu.complete_cycle(None).unwrap());
            expected.sp = 0xFFFD;
            assert_eq!(cpu.snapshot(), expected);
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr: 0xFFFC, value } if value == source as u8)
            );
            assert!(cpu.complete_cycle(None).unwrap());
            expected.sp = 0xFFFC;
            assert_eq!(cpu.snapshot(), expected);
            assert!(!cpu.complete_cycle(Some(pop)).unwrap());
            expected.pc += 1;
            assert!(!cpu.complete_cycle(Some(word as u8)).unwrap());
            expected.sp = 0xFFFD;
            assert_eq!(cpu.snapshot(), expected);
            assert!(cpu.complete_cycle(Some((word / 256) as u8)).unwrap());
            expected.sp = 0xFFFE;
            match pair {
                PushOperand::BC => expected.bc = word,
                PushOperand::DE => expected.de = word,
                PushOperand::HL => expected.hl = word,
                PushOperand::AF => expected.af = word & 0xFFF0,
            }
            assert_eq!(cpu.snapshot(), expected);
        }
    }
}

#[test]
fn pop_bc_decodes_to_the_stack_pair() {
    assert!(matches!(
        DecodedOpcode::from(0xC1),
        DecodedOpcode::Pop {
            operand: PushOperand::BC
        }
    ));
}

#[test]
fn pop_bc_consumes_every_word_with_all_initial_flags_via_the_decoder() {
    for flags in 0u16..16 {
        for word in 0u16..=u16::MAX {
            let mut registers = Registers::dmg_post_boot(1);
            registers.sp = 0xFFFF;
            registers.pc = 0xFFFF;
            registers.set_af(0x4200 | (flags << 4));
            let mut cpu = CPU::from_registers(registers);
            let mut expected = cpu.snapshot();
            assert!(!cpu.complete_cycle(Some(0xC1)).unwrap());
            expected.pc = 0;
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle().unwrap(),
                CycleAction::Read { addr: 0xFFFF }
            ));
            assert!(!cpu.complete_cycle(Some(word as u8)).unwrap());
            expected.sp = 0;
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle().unwrap(),
                CycleAction::Read { addr: 0 }
            ));
            assert!(cpu.complete_cycle(Some((word / 256) as u8)).unwrap());
            expected.sp = 1;
            expected.bc = word;
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle().unwrap(),
                CycleAction::Read { addr: 0 }
            ));
        }
    }
}

#[test]
fn pop_low_requests_the_next_address_without_assigning_the_pair() {
    for target in [
        PushOperand::BC,
        PushOperand::DE,
        PushOperand::HL,
        PushOperand::AF,
    ] {
        for start in [0u16, 1, 0xC000, 0xFFFC, 0xFFFF] {
            for flags in 0u16..16 {
                let mut registers = Registers::dmg_post_boot(1);
                registers.sp = start;
                registers.set_af(0x4200 | (flags << 4));
                let mut cpu = CPU::from_registers(registers);
                cpu.state = ExecutionState::PopLow {
                    addr: start,
                    target,
                };
                // Live SP must not override the captured read address.
                cpu.registers.sp = 0x8000;
                let mut expected = cpu.snapshot();
                for _ in 0..2 {
                    assert!(
                        matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == start)
                    );
                    assert_eq!(cpu.snapshot(), expected);
                }
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start)
                );
                assert_eq!(cpu.snapshot(), expected);
                assert!(!cpu.complete_cycle(Some(0x3F)).unwrap());
                expected.sp = start.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
                assert!(
                    matches!(cpu.request_cycle().unwrap(), CycleAction::Read { addr } if addr == start.wrapping_add(1)),
                    "high read must use S+1"
                );
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start.wrapping_add(1))
                );
                assert_eq!(cpu.snapshot(), expected);
                cpu.registers.sp = 0x9000;
                assert!(cpu.complete_cycle(Some(0x12)).unwrap());
                expected.sp = start.wrapping_add(2);
                match target {
                    PushOperand::BC => expected.bc = 0x123F,
                    PushOperand::DE => expected.de = 0x123F,
                    PushOperand::HL => expected.hl = 0x123F,
                    PushOperand::AF => expected.af = 0x1230,
                }
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}

#[test]
fn pop_high_assigns_every_word_and_masks_af_without_other_side_effects() {
    for target in [
        PushOperand::BC,
        PushOperand::DE,
        PushOperand::HL,
        PushOperand::AF,
    ] {
        for value in 0u16..=u16::MAX {
            let mut registers = Registers::dmg_post_boot(1);
            registers.sp = 0xFFFF;
            let mut cpu = CPU::from_registers(registers);
            cpu.state = ExecutionState::PopHigh {
                addr: 0xFFFF,
                low: value as u8,
                target,
            };
            let mut expected = cpu.snapshot();
            assert!(matches!(
                cpu.complete_cycle(None),
                Err(CpuError::MissingReadValue { addr: 0xFFFF })
            ));
            assert_eq!(cpu.snapshot(), expected);
            assert!(cpu.complete_cycle(Some((value / 256) as u8)).unwrap());
            expected.sp = 0;
            match target {
                PushOperand::BC => expected.bc = value,
                PushOperand::DE => expected.de = value,
                PushOperand::HL => expected.hl = value,
                PushOperand::AF => expected.af = value & 0xFFF0,
            }
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(
                cpu.request_cycle().unwrap(),
                CycleAction::Read { addr: 0x0100 }
            ));
        }
    }
}

#[test]
fn push_bc_writes_every_word_high_then_low_below_initial_sp() {
    for value in 0u16..=u16::MAX {
        let mut registers = Registers::dmg_post_boot(1);
        registers.set_bc(value);
        let mut cpu = CPU::from_registers(registers);
        let mut expected = cpu.snapshot();
        assert!(!cpu.complete_cycle(Some(0xC5)).unwrap());
        expected.pc += 1;
        assert_eq!(cpu.snapshot(), expected);
        for _ in 0..2 {
            assert!(matches!(cpu.request_cycle().unwrap(), CycleAction::Idle));
            assert_eq!(cpu.snapshot(), expected);
        }
        assert!(!cpu.complete_cycle(None).unwrap());
        for _ in 0..2 {
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr: 0xFFFD, value: high } if high == (value / 256) as u8),
                "high byte must target SP-1 for word {value:04X}"
            );
            assert_eq!(cpu.snapshot(), expected);
        }
        assert!(!cpu.complete_cycle(None).unwrap());
        expected.sp = 0xFFFD;
        assert_eq!(cpu.snapshot(), expected);
        for _ in 0..2 {
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr: 0xFFFC, value: low } if low == value as u8),
                "low byte must target SP-2 for word {value:04X}"
            );
            assert_eq!(cpu.snapshot(), expected);
        }
        assert!(cpu.complete_cycle(None).unwrap());
        expected.sp = 0xFFFC;
        assert_eq!(cpu.snapshot(), expected);
        assert!(matches!(
            cpu.request_cycle().unwrap(),
            CycleAction::Read { addr: 0x0101 }
        ));
    }
}

#[test]
fn push_bc_captures_source_and_assigns_sp_from_pending_addresses() {
    for start in [0u16, 1, 0xC002, 0xFF80, 0xFFFE, 0xFFFF] {
        for flags in 0u16..16 {
            let mut registers = Registers::dmg_post_boot(1);
            registers.pc = 0xFFFF;
            registers.sp = start;
            registers.set_bc(0x1234);
            registers.set_af(0x4200 | (flags << 4));
            let mut cpu = CPU::from_registers(registers);
            let original = cpu.snapshot();
            assert!(matches!(
                cpu.complete_cycle(None),
                Err(CpuError::MissingReadValue { addr: 0xFFFF })
            ));
            assert_eq!(cpu.snapshot(), original);
            assert!(!cpu.complete_cycle(Some(0xC5)).unwrap());
            cpu.registers.set_bc(0xABCD); // Test-only source perturbation.
            cpu.registers.sp = 0x8000;
            assert!(!cpu.complete_cycle(None).unwrap());
            assert_eq!(cpu.snapshot().sp, 0x8000); // Idle does not change SP.
            assert!(!cpu.complete_cycle(None).unwrap());
            assert_eq!(
                cpu.snapshot().sp,
                start.wrapping_sub(1),
                "SP must come from captured address"
            );
            assert!(
                matches!(cpu.request_cycle().unwrap(), CycleAction::Write { addr, value: 0x34 } if addr == start.wrapping_sub(2))
            );
            cpu.registers.sp = 0x9000;
            assert!(cpu.complete_cycle(None).unwrap());
            let mut expected = original;
            expected.pc = 0;
            expected.bc = 0xABCD;
            expected.sp = start.wrapping_sub(2);
            assert_eq!(cpu.snapshot(), expected);
        }
    }
}
