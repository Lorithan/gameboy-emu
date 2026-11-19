use std::error::Error;

use gameboy_emu::{
    cpu::{CpuError, CpuFault, CycleAction, CycleKind, OpcodeEncoding, UnexpectedCycleResponse},
    system::{CycleRecord, SystemError},
};

fn record() -> CycleRecord {
    CycleRecord {
        tick_before: 124,
        tick_after: 128,
        action: CycleAction::Read { addr: 0xC000 },
        read_value: Some(0xD3),
    }
}

#[test]
fn completion_message_reports_the_completion_tick_and_relevant_bus_action_details() {
    for (action, details) in [
        (CycleAction::Idle, &["idle"][..]),
        (CycleAction::Read { addr: 0xC000 }, &["read", "0xC000"][..]),
        (
            CycleAction::Write {
                addr: 0xC000,
                value: 0x42,
            },
            &["write", "0x42", "0xC000"][..],
        ),
    ] {
        let error = SystemError::CpuCompletionError(
            CpuError::NoCycleRequested,
            CycleRecord { action, ..record() },
        );
        let message = error.to_string();
        assert!(
            message.contains("128"),
            "missing completion tick: {message}"
        );
        for detail in details {
            assert!(message.contains(detail), "missing {detail}: {message}");
        }
    }
}

#[test]
fn both_system_error_phases_expose_cpu_and_original_fault_sources() {
    let fault = CpuFault {
        opcode: OpcodeEncoding::Base(0xD3),
        addr: 0xFFFF,
    };
    let cpu = CpuError::UnsupportedOpcode(fault);
    for error in [
        SystemError::CpuRequestError(cpu),
        SystemError::CpuCompletionError(cpu, record()),
    ] {
        let standard: &dyn Error = &error;
        assert!(!standard.to_string().is_empty());
        let cpu = standard
            .source()
            .expect("system error must retain CPU cause");
        assert!(cpu.downcast_ref::<CpuError>().is_some());
        let cause = cpu.source().expect("CPU cause must retain original fault");
        let original = cause.downcast_ref::<CpuFault>().unwrap();
        assert_eq!(original.opcode, fault.opcode);
        assert_eq!(original.addr, fault.addr);
        assert!(cause.source().is_none());
    }
}

#[test]
fn system_source_chains_preserve_protocol_details_and_stop_at_leaf_cpu_errors() {
    let protocol = UnexpectedCycleResponse {
        expected: CycleKind::Write,
        received: CycleKind::Idle,
    };
    for cpu in [CpuError::from(protocol), CpuError::NoCycleRequested] {
        for error in [
            SystemError::CpuRequestError(cpu),
            SystemError::CpuCompletionError(cpu, record()),
        ] {
            let inner = error.source().unwrap().downcast_ref::<CpuError>().unwrap();
            match cpu {
                CpuError::UnexpectedCycleResponse(_) => {
                    let cause = inner.source().unwrap();
                    assert_eq!(
                        cause.downcast_ref::<UnexpectedCycleResponse>(),
                        Some(&protocol)
                    );
                    assert!(cause.source().is_none());
                }
                CpuError::NoCycleRequested => assert!(inner.source().is_none()),
                _ => unreachable!("only the two explicitly constructed cases are used"),
            }
        }
    }
}
