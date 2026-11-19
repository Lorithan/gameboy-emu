use std::error::Error;

use gameboy_emu::cpu::{CpuError, CpuFault, CycleKind, OpcodeEncoding, UnexpectedCycleResponse};

#[test]
fn protocol_error_is_a_standard_leaf_error_with_readable_kind_details() {
    let error = UnexpectedCycleResponse {
        expected: CycleKind::Read,
        received: CycleKind::Write,
    };
    let standard: &dyn Error = &error;
    assert!(standard.source().is_none());
    let message = standard.to_string();
    assert!(message.contains("read"), "{message}");
    assert!(message.contains("write"), "{message}");
}

#[test]
fn fault_error_preserves_opcode_encoding_and_address_in_leaf_diagnostics() {
    for (opcode, marker) in [
        (OpcodeEncoding::Base(0xD3), "d3"),
        (OpcodeEncoding::Cb(0xAB), "ab"),
    ] {
        let fault = CpuFault {
            opcode,
            addr: 0xC123,
        };
        let standard: &dyn Error = &fault;
        assert!(standard.source().is_none());
        let message = standard.to_string().to_ascii_lowercase();
        assert!(message.contains(marker), "{message}");
        assert!(message.contains("c123"), "{message}");
        if matches!(opcode, OpcodeEncoding::Cb(_)) {
            assert!(message.contains("cb"), "{message}");
        }
    }
}

#[test]
fn cpu_error_exposes_typed_causes_and_retains_the_original_details() {
    let protocol = UnexpectedCycleResponse {
        expected: CycleKind::Idle,
        received: CycleKind::Read,
    };
    let error = CpuError::from(protocol);
    let cause = Error::source(&error).expect("wrapped protocol error must remain accessible");
    assert_eq!(
        cause.downcast_ref::<UnexpectedCycleResponse>(),
        Some(&protocol)
    );
    assert!(cause.source().is_none());
    assert!(!error.to_string().is_empty());

    let fault = CpuFault {
        opcode: OpcodeEncoding::Base(0xD3),
        addr: 0xFFFF,
    };
    let error = CpuError::UnsupportedOpcode(fault);
    let cause = Error::source(&error).expect("wrapped CPU fault must remain accessible");
    let original = cause
        .downcast_ref::<CpuFault>()
        .expect("cause must preserve its concrete type");
    assert_eq!(original.opcode, fault.opcode);
    assert_eq!(original.addr, fault.addr);
    assert!(cause.source().is_none());

    let error = CpuError::NoCycleRequested;
    let standard: &dyn Error = &error;
    assert!(standard.source().is_none());
    assert!(!standard.to_string().is_empty());
}
