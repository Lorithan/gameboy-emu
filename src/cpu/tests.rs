use super::{CPU, CpuError, CpuFault, CycleAction, ExecutionState, Registers};

fn unsupported_fault(error: CpuError) -> CpuFault {
    match error {
        CpuError::UnsupportedOpcode(fault) => fault,
        CpuError::MissingReadValue { address } => {
            panic!("expected unsupported opcode, got missing read data at {address:#06X}")
        }
    }
}

#[test]
fn first_cycle_requests_the_current_program_counter() {
    let cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    match cpu.request_cycle().expect("opcode fetch request") {
        CycleAction::Read { addr } => assert_eq!(addr, 0x0100),
        _ => panic!("initial cycle must fetch an opcode"),
    }
}

#[test]
fn repeated_cycle_requests_do_not_change_cpu_state_or_address() {
    let cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    let original = cpu.snapshot();
    for _ in 0..3 {
        match cpu.request_cycle().expect("pending fetch request") {
            CycleAction::Read { addr } => assert_eq!(addr, original.pc),
            _ => panic!("pending opcode fetch must remain a read"),
        }
        assert_eq!(cpu.snapshot(), original);
    }
}

#[test]
fn request_at_last_address_does_not_increment_or_overflow_pc() {
    let mut registers = Registers::dmg_post_boot(0);
    registers.pc = 0xFFFF;
    let cpu = CPU::from_registers(registers);
    match cpu.request_cycle().expect("fetch at address boundary") {
        CycleAction::Read { addr } => assert_eq!(addr, 0xFFFF),
        _ => panic!("opcode fetch must remain a read at the address boundary"),
    }
    assert_eq!(cpu.snapshot().pc, 0xFFFF);
}

#[test]
fn nop_completes_and_only_advances_pc() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    let mut expected = cpu.snapshot();
    expected.pc = 0x0101;
    assert!(cpu.complete_cycle(Some(0x00)).expect("NOP completion"));
    assert_eq!(cpu.snapshot(), expected);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0101 })
    ));
}

#[test]
fn immediate_load_opcode_does_not_complete_the_instruction() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    let mut expected = cpu.snapshot();
    expected.pc = 0x0101;
    let complete = cpu.complete_cycle(Some(0x3E)).expect("LD opcode fetch");
    assert_eq!(cpu.snapshot(), expected);
    assert!(!complete, "LD requires its operand cycle before completion");
}

#[test]
fn immediate_completion_stores_a_advances_pc_and_returns_to_opcode_fetch() {
    for value in [0x00, 0x42, 0x80, 0xFF] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
        cpu.complete_cycle(Some(0x3E)).expect("LD opcode fetch");
        let mut expected = cpu.snapshot();
        expected.pc = 0x0102;
        expected.af = u16::from(value) << 8 | (expected.af & 0x00FF);
        assert!(cpu.complete_cycle(Some(value)).expect("LD operand read"));
        assert_eq!(cpu.snapshot(), expected, "immediate {value:#04X}");
        assert!(matches!(cpu.state, ExecutionState::FetchOpcode));
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Read { addr: 0x0102 })
        ));
    }
}

#[test]
fn operand_completion_returns_to_opcode_fetch_even_when_pc_is_checked_separately() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    cpu.complete_cycle(Some(0x3E)).expect("LD opcode fetch");
    cpu.complete_cycle(Some(0x42)).expect("LD operand read");
    assert!(matches!(cpu.state, ExecutionState::FetchOpcode));
}

#[test]
fn unsupported_fetch_records_original_address_and_consumes_one_byte() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    let mut expected = cpu.snapshot();
    expected.pc = 0x0101;
    let fault = unsupported_fault(
        cpu.complete_cycle(Some(0xD3))
            .expect_err("opcode not implemented"),
    );
    assert_eq!(fault.opcode, 0xD3);
    assert_eq!(fault.addr, 0x0100);
    assert_eq!(cpu.snapshot(), expected);
}

#[test]
fn unsupported_opcode_fault_is_sticky_for_requests_and_completions() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    let first = unsupported_fault(
        cpu.complete_cycle(Some(0xD3))
            .expect_err("opcode not implemented"),
    );
    let state_after_fault = cpu.snapshot();
    for _ in 0..2 {
        let requested = unsupported_fault(
            cpu.request_cycle()
                .expect_err("fault must stop further requests"),
        );
        assert_eq!(
            (requested.opcode, requested.addr),
            (first.opcode, first.addr)
        );
        let completed = unsupported_fault(
            cpu.complete_cycle(Some(0))
                .expect_err("fault must stop further completions"),
        );
        assert_eq!(
            (completed.opcode, completed.addr),
            (first.opcode, first.addr)
        );
        assert_eq!(cpu.snapshot(), state_after_fault);
    }
}

#[test]
fn nop_completion_wraps_pc() {
    let mut registers = Registers::dmg_post_boot(0);
    registers.pc = 0xFFFF;
    let mut cpu = CPU::from_registers(registers);
    assert!(cpu.complete_cycle(Some(0)).expect("NOP at final address"));
    assert_eq!(cpu.snapshot().pc, 0);
}

#[test]
fn immediate_completion_wraps_pc() {
    let mut registers = Registers::dmg_post_boot(0);
    registers.pc = 0xFFFF;
    let mut cpu = CPU::from_registers(registers);
    cpu.state = ExecutionState::FetchImmediateA;
    assert!(
        cpu.complete_cycle(Some(0x42))
            .expect("immediate at final address")
    );
    assert_eq!(cpu.snapshot().pc, 0);
    assert_eq!(cpu.snapshot().af, 0x4280);
}

#[test]
fn missing_read_value_returns_an_error_without_mutating_cpu() {
    for state in [ExecutionState::FetchOpcode, ExecutionState::FetchImmediateA] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
        cpu.state = state;
        let original = cpu.snapshot();
        assert!(matches!(
            cpu.complete_cycle(None),
            Err(CpuError::MissingReadValue { address: 0x0100 })
        ));
        assert_eq!(cpu.snapshot(), original);
        assert_eq!(
            std::mem::discriminant(&cpu.state),
            std::mem::discriminant(&state)
        );
    }
}

#[test]
fn missing_operand_can_be_retried_without_consuming_or_skipping_data() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    cpu.complete_cycle(Some(0x3E)).expect("LD opcode fetch");
    let pending = cpu.snapshot();
    assert!(matches!(
        cpu.complete_cycle(None),
        Err(CpuError::MissingReadValue { address: 0x0101 })
    ));
    assert_eq!(cpu.snapshot(), pending);
    assert!(
        cpu.complete_cycle(Some(0x42))
            .expect("retry operand completion")
    );
    assert_eq!(cpu.snapshot().pc, 0x0102);
    assert_eq!(cpu.snapshot().af, 0x4280);
}

#[test]
fn unsupported_fetch_wraps_pc_and_retains_the_original_fault_address() {
    let mut registers = Registers::dmg_post_boot(0);
    registers.pc = 0xFFFF;
    let mut cpu = CPU::from_registers(registers);
    let fault = unsupported_fault(
        cpu.complete_cycle(Some(0xD3))
            .expect_err("unsupported fetch"),
    );
    assert_eq!((fault.opcode, fault.addr), (0xD3, 0xFFFF));
    assert_eq!(cpu.snapshot().pc, 0);
}
