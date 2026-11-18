#[path = "support/responses.rs"]
mod response_support;
use gameboy_emu::cpu::{CycleResponse, InterruptSample};
use gameboy_emu::{
    cpu::{CPU, CpuError, CycleAction, CycleOutcome, OpcodeEncoding, Registers},
    memory::Cartridge,
    system::{CycleRecord, InstructionStep, System, SystemError},
};
use response_support::response;

#[test]
fn public_cpu_outcomes_preserve_retry_and_next_instruction_boundaries() {
    let mut cpu = CPU::from_registers(Registers::dmg_post_boot(0));
    assert_eq!(
        cpu.complete_cycle(response(cpu.request_cycle(), Some(0x3E)))
            .unwrap(),
        CycleOutcome::InProgress
    );
    let pending = cpu.snapshot();
    for _ in 0..2 {
        assert_eq!(
            cpu.request_cycle().unwrap(),
            CycleAction::Read { addr: 0x101 }
        );
        assert!(matches!(
            cpu.complete_cycle(response(cpu.request_cycle(), None)),
            Err(CpuError::UnexpectedCycleResponse(_)) if cpu.request_cycle().unwrap() == (CycleAction::Read { addr: 0x101 })
        ));
        assert_eq!(cpu.snapshot(), pending);
    }
    assert_eq!(
        cpu.complete_cycle(response(cpu.request_cycle(), Some(0x42)))
            .unwrap(),
        CycleOutcome::InstructionComplete
    );
    assert_eq!(cpu.snapshot().af, 0x4280);
    assert_eq!(
        cpu.request_cycle().unwrap(),
        CycleAction::Read { addr: 0x102 }
    );
    assert_eq!(
        cpu.complete_cycle(response(cpu.request_cycle(), Some(0)))
            .unwrap(),
        CycleOutcome::InstructionComplete
    );
    let first = cpu
        .complete_cycle(response(cpu.request_cycle(), Some(0xD3)))
        .unwrap_err();
    let faulted = cpu.snapshot();
    for error in [
        first,
        cpu.request_cycle().unwrap_err(),
        cpu.complete_cycle(response(cpu.request_cycle(), None))
            .unwrap_err(),
    ] {
        match error {
            CpuError::UnsupportedOpcode(fault) => {
                assert_eq!(fault.opcode, OpcodeEncoding::Base(0xD3));
                assert_eq!(fault.addr, 0x103);
            }
            other => panic!("unexpected error {other:?}"),
        }
    }
    assert_eq!(cpu.snapshot(), faulted);
}

#[test]
fn successful_trace_and_failed_record_preserve_full_cycle_diagnostics() {
    let mut rom = vec![0; 32768];
    rom[0x14D] = 0xE7;
    rom[0x100..0x104].copy_from_slice(&[0x3E, 0x42, 0, 0xD3]);
    let mut s = System::new(Cartridge::from_bytes(&rom).unwrap());
    for (index, byte, outcome) in [
        (0, 0x3E, CycleOutcome::InProgress),
        (1, 0x42, CycleOutcome::InstructionComplete),
        (2, 0, CycleOutcome::InstructionComplete),
    ] {
        let trace = s.step_machine_cycle().unwrap();
        assert_eq!(trace.outcome, outcome);
        assert_eq!(
            trace.record,
            CycleRecord {
                tick_before: index * 4,
                tick_after: index * 4 + 4,
                action: CycleAction::Read {
                    addr: 0x100 + index as u16
                },
                read_value: Some(byte),
            }
        );
    }
    match s.step_instruction(100).unwrap_err() {
        SystemError::CpuCompletionError(CpuError::UnsupportedOpcode(fault), record) => {
            assert_eq!(fault.opcode, OpcodeEncoding::Base(0xD3));
            assert_eq!(fault.addr, 0x103);
            assert_eq!(
                record,
                CycleRecord {
                    tick_before: 12,
                    tick_after: 16,
                    action: CycleAction::Read { addr: 0x103 },
                    read_value: Some(0xD3),
                }
            );
        }
        other => panic!("unexpected error {other:?}"),
    }
    let snapshot = s.cpu_snapshot();
    assert!(matches!(
        s.step_instruction(0).unwrap(),
        InstructionStep::BudgetExhausted {
            mcycles: 0,
            ticks: 0
        }
    ));
    assert!(matches!(
        s.step_instruction(1),
        Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
    ));
    assert_eq!(s.elapsed_ticks(), 16);
    assert_eq!(s.cpu_snapshot(), snapshot);
}
