use super::registers::{Register, Register16};
use super::{CPU, CpuError, CpuFault, CycleAction, Destination, ExecutionState, Registers};

mod absolute_jumps;
mod adc_sbc;
mod byte_transfers;
mod call_ret;
mod compare;
mod conditional_returns;
mod memory_inc_dec;
mod operand_add_sub;
mod register_add_sub;
mod register_inc_dec;
mod relative_jumps;
mod stack_push_pop;

fn unsupported_fault(error: CpuError) -> CpuFault {
    match error {
        CpuError::UnsupportedOpcode(fault) => fault,
        CpuError::MissingReadValue { addr: address } => {
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
    cpu.state = ExecutionState::ReadImmediate8 {
        target: Destination::Register(Register::A),
    };
    assert!(
        cpu.complete_cycle(Some(0x42))
            .expect("immediate at final address")
    );
    assert_eq!(cpu.snapshot().pc, 0);
    assert_eq!(cpu.snapshot().af, 0x4280);
}

#[test]
fn missing_read_value_returns_an_error_without_mutating_cpu() {
    for state in [
        ExecutionState::FetchOpcode,
        ExecutionState::ReadImmediate8 {
            target: Destination::Register(Register::A),
        },
    ] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
        cpu.state = state;
        let original = cpu.snapshot();
        assert!(matches!(
            cpu.complete_cycle(None),
            Err(CpuError::MissingReadValue { addr: 0x0100 })
        ));
        assert_eq!(cpu.snapshot(), original);
        assert_eq!(
            std::mem::discriminant(&cpu.state),
            std::mem::discriminant(&state)
        );
    }
}

#[test]
fn all_immediate_register_loads_preserve_every_valid_flag_combination() {
    for opcode in [0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E, 0x3E] {
        for combination in 0u16..16 {
            let flags = combination << 4;
            let mut registers = Registers::dmg_post_boot(0);
            registers.set_af(0x0100 | flags);
            let mut cpu = CPU::from_registers(registers);
            assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode"));
            assert_eq!(cpu.snapshot().af & 0xFF, flags);
            assert!(cpu.complete_cycle(Some(0x80)).expect("operand"));
            assert_eq!(cpu.snapshot().af & 0xFF, flags, "opcode {opcode:#04X}");
        }
    }
}

#[test]
fn all_immediate_register_loads_wrap_pc_and_retain_destination_after_missing_data() {
    for opcode in [0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E, 0x3E] {
        let mut registers = Registers::dmg_post_boot(0);
        registers.pc = 0xFFFF;
        let mut cpu = CPU::from_registers(registers);
        assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode at FFFF"));
        assert_eq!(cpu.snapshot().pc, 0);
        let pending = cpu.snapshot();
        assert!(matches!(
            cpu.complete_cycle(None),
            Err(CpuError::MissingReadValue { addr: 0 })
        ));
        assert_eq!(cpu.snapshot(), pending);
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Read { addr: 0 })
        ));
        assert!(cpu.complete_cycle(Some(0x42)).expect("retry operand"));
        assert_eq!(cpu.snapshot().pc, 1);
    }
}

#[test]
fn missing_operand_can_be_retried_without_consuming_or_skipping_data() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    cpu.complete_cycle(Some(0x3E)).expect("LD opcode fetch");
    let pending = cpu.snapshot();
    assert!(matches!(
        cpu.complete_cycle(None),
        Err(CpuError::MissingReadValue { addr: 0x0101 })
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

// Isolate operand handling from decoding so a decoder omission cannot hide
// bugs in the new execution states. Public tests exercise the complete path.
#[test]
fn word_operand_states_load_every_value_without_partial_register_updates() {
    for destination in [
        Register16::BC,
        Register16::DE,
        Register16::HL,
        Register16::SP,
    ] {
        for value in 0u16..=u16::MAX {
            let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
            cpu.state = ExecutionState::ReadImmediate16Low { destination };
            let mut expected = cpu.snapshot();
            assert!(!cpu.complete_cycle(Some(value as u8)).expect("low byte"));
            expected.pc += 1;
            assert_eq!(cpu.snapshot(), expected);
            assert!(
                cpu.complete_cycle(Some((value >> 8) as u8))
                    .expect("high byte")
            );
            expected.pc += 1;
            match destination {
                Register16::BC => expected.bc = value,
                Register16::DE => expected.de = value,
                Register16::HL => expected.hl = value,
                Register16::SP => expected.sp = value,
            }
            assert_eq!(cpu.snapshot(), expected);
            assert!(matches!(cpu.state, ExecutionState::FetchOpcode));
        }
    }
}

#[test]
fn word_operand_states_preserve_pending_data_on_missing_reads_and_wrap_pc() {
    for destination in [
        Register16::BC,
        Register16::DE,
        Register16::HL,
        Register16::SP,
    ] {
        for start in [0xFFFEu16, 0xFFFF] {
            for flags in 0u16..16 {
                let mut registers = Registers::dmg_post_boot(1);
                registers.pc = start;
                registers.set_af(0x0100 | flags << 4);
                let mut cpu = CPU::from_registers(registers);
                cpu.state = ExecutionState::ReadImmediate16Low { destination };
                for byte in [0x34, 0x12] {
                    let pending = cpu.snapshot();
                    assert!(
                        matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pending.pc)
                    );
                    assert_eq!(cpu.snapshot(), pending);
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pending.pc)
                    );
                    cpu.complete_cycle(Some(byte)).expect("retry operand");
                }
                let final_state = cpu.snapshot();
                let mut expected = CPU::from_registers(registers).snapshot();
                expected.pc = start.wrapping_add(2);
                match destination {
                    Register16::BC => expected.bc = 0x1234,
                    Register16::DE => expected.de = 0x1234,
                    Register16::HL => expected.hl = 0x1234,
                    Register16::SP => expected.sp = 0x1234,
                }
                assert_eq!(final_state, expected);
            }
        }
    }
}

#[test]
fn word_loads_wrap_at_each_fetch_stage_and_retry_without_losing_data() {
    for opcode in [0x01, 0x11, 0x21, 0x31] {
        for start in [0xFFFDu16, 0xFFFE, 0xFFFF] {
            let mut registers = Registers::dmg_post_boot(1);
            registers.pc = start;
            let mut cpu = CPU::from_registers(registers);
            let mut expected = cpu.snapshot();
            for (index, byte) in [opcode, 0x34, 0x12].into_iter().enumerate() {
                let pending = cpu.snapshot();
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == pending.pc)
                );
                assert_eq!(cpu.snapshot(), pending);
                assert!(
                    matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pending.pc)
                );
                assert_eq!(
                    cpu.complete_cycle(Some(byte)).expect("retry fetch"),
                    index == 2
                );
                expected.pc = expected.pc.wrapping_add(1);
                if index == 2 {
                    match opcode {
                        0x01 => expected.bc = 0x1234,
                        0x11 => expected.de = 0x1234,
                        0x21 => expected.hl = 0x1234,
                        0x31 => expected.sp = 0x1234,
                        _ => unreachable!(),
                    }
                }
                assert_eq!(cpu.snapshot(), expected);
            }
        }
    }
}

#[test]
fn missing_data_memory_read_reports_the_requested_address_and_can_be_retried() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    assert!(!cpu.complete_cycle(Some(0x7E)).expect("LD A,(HL) opcode"));
    let pending = cpu.snapshot();
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x014D })
    ));
    assert!(matches!(
        cpu.complete_cycle(None),
        Err(CpuError::MissingReadValue { addr: 0x014D })
    ));
    assert_eq!(cpu.snapshot(), pending);
    assert!(cpu.complete_cycle(Some(0x42)).expect("retry data read"));
    let mut expected = pending;
    expected.af = 0x4200 | (pending.af & 0xFF);
    assert_eq!(cpu.snapshot(), expected);
}

#[test]
fn missing_immediate_store_operand_reports_pc_and_preserves_pending_state() {
    // Isolate operand handling; public tests cover the opcode transition.
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    cpu.state = ExecutionState::ReadImmediate8 {
        target: Destination::Memory(0x014D),
    };
    let pending = cpu.snapshot();
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0100 })
    ));
    assert!(matches!(
        cpu.complete_cycle(None),
        Err(CpuError::MissingReadValue { addr: 0x0100 })
    ));
    assert_eq!(cpu.snapshot(), pending);
    assert!(!cpu.complete_cycle(Some(0x42)).expect("retry immediate"));
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Write {
            addr: 0x014D,
            value: 0x42
        })
    ));
    assert!(cpu.complete_cycle(None).expect("write completion"));
    let mut expected = pending;
    expected.pc += 1;
    assert_eq!(cpu.snapshot(), expected);
    assert!(matches!(
        cpu.request_cycle(),
        Ok(CycleAction::Read { addr: 0x0101 })
    ));
}

#[test]
fn pending_memory_read_uses_its_captured_address_for_requests_and_errors() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
    // Exercise the resolved-address state independently of HL, as required
    // for future non-HL addressing. No current instruction changes HL here.
    cpu.state = ExecutionState::ReadMemory {
        addr: 0xC123,
        destination: Register::A,
    };
    let pending = cpu.snapshot();
    for _ in 0..2 {
        assert!(matches!(
            cpu.request_cycle(),
            Ok(CycleAction::Read { addr: 0xC123 })
        ));
        assert!(matches!(
            cpu.complete_cycle(None),
            Err(CpuError::MissingReadValue { addr: 0xC123 })
        ));
        assert_eq!(cpu.snapshot(), pending);
    }
    assert!(cpu.complete_cycle(Some(0x42)).expect("retry captured read"));
    let mut expected = pending;
    expected.af = 0x4200 | (pending.af & 0xFF);
    assert_eq!(cpu.snapshot(), expected);
}

#[test]
fn hl_loads_preserve_flags_and_unrelated_registers_across_wraps_and_read_retries() {
    for opcode in [0x36, 0x77, 0x7E] {
        for start in [0xFFFEu16, 0xFFFF] {
            for flags in 0u16..16 {
                let mut registers = Registers::dmg_post_boot(1);
                registers.pc = start;
                registers.set_hl(0xC123);
                registers.set_af(0x9900 | flags << 4);
                let mut cpu = CPU::from_registers(registers);
                let mut expected = cpu.snapshot();
                assert!(
                    matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == start)
                );
                assert_eq!(cpu.snapshot(), expected);
                assert!(!cpu.complete_cycle(Some(opcode)).expect("opcode"));
                expected.pc = expected.pc.wrapping_add(1);
                assert_eq!(cpu.snapshot(), expected);
                if opcode == 0x36 || opcode == 0x7E {
                    let read_address = if opcode == 0x36 { expected.pc } else { 0xC123 };
                    for _ in 0..2 {
                        assert!(
                            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == read_address)
                        );
                        assert!(
                            matches!(cpu.complete_cycle(None), Err(CpuError::MissingReadValue { addr }) if addr == read_address)
                        );
                        assert_eq!(cpu.snapshot(), expected);
                    }
                    assert_eq!(
                        cpu.complete_cycle(Some(0x42)).expect("read"),
                        opcode == 0x7E
                    );
                    if opcode == 0x36 {
                        expected.pc = expected.pc.wrapping_add(1);
                    } else {
                        expected.af = 0x4200 | flags << 4;
                    }
                }
                if opcode != 0x7E {
                    let expected_value = if opcode == 0x77 { 0x99 } else { 0x42 };
                    assert!(
                        matches!(cpu.request_cycle(), Ok(CycleAction::Write { addr: 0xC123, value }) if value == expected_value)
                    );
                    assert!(cpu.complete_cycle(None).expect("write"));
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
fn pending_writes_capture_address_and_value_and_preserve_pc_on_completion() {
    for opcode in [0x77, 0x36] {
        let mut cpu = CPU::from_registers(Registers::dmg_post_boot(1));
        cpu.registers.set_hl(0xC123);
        cpu.registers.a = 0x42;
        assert!(!cpu.complete_cycle(Some(opcode)).expect("store opcode"));
        // Deliberately change registers to check that resolved address/data
        // belong to the pending state, rather than being re-read on request.
        cpu.registers.set_hl(0xD456);
        cpu.registers.a = 0x99;
        if opcode == 0x36 {
            assert!(!cpu.complete_cycle(Some(0x42)).expect("immediate byte"));
        }
        let pending = cpu.snapshot();
        for _ in 0..2 {
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Write {
                    addr: 0xC123,
                    value: 0x42
                })
            ));
            assert_eq!(cpu.snapshot(), pending);
        }
        assert!(cpu.complete_cycle(None).expect("write completion"));
        assert_eq!(cpu.snapshot(), pending);
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pending.pc)
        );
    }
}
