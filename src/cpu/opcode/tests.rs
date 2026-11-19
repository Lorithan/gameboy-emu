use super::{DecodedOpcode, Register, Register16, Target, UnaryOperation};

#[test]
fn illegal_decode_errors_support_standard_leaf_reporting() {
    let error = DecodedOpcode::try_from(0xD3).unwrap_err();
    let standard: &dyn std::error::Error = &error;
    assert!(standard.source().is_none());
    assert!(standard.to_string().contains("D3"));
}

#[test]
fn hl_memory_opcodes_decode_to_the_expected_direction_and_register() {
    assert!(matches!(
        DecodedOpcode::try_from(0x36).unwrap(),
        DecodedOpcode::LoadImmediate {
            destination: Target::IndirectHL
        }
    ));
    assert!(matches!(
        DecodedOpcode::try_from(0x77).unwrap(),
        DecodedOpcode::Load {
            destination: Target::IndirectHL,
            source: Target::Register(Register::A)
        }
    ));
    assert!(matches!(
        DecodedOpcode::try_from(0x7E).unwrap(),
        DecodedOpcode::Load {
            destination: Target::Register(Register::A),
            source: Target::IndirectHL
        }
    ));
}

#[test]
fn known_opcode_bytes_decode_to_the_expected_operation_and_destination() {
    assert!(matches!(
        DecodedOpcode::try_from(0).unwrap(),
        DecodedOpcode::Nop
    ));
    for (byte, expected) in [
        (0x06, Register::B),
        (0x0E, Register::C),
        (0x16, Register::D),
        (0x1E, Register::E),
        (0x26, Register::H),
        (0x2E, Register::L),
        (0x3E, Register::A),
    ] {
        match DecodedOpcode::try_from(byte).unwrap() {
            DecodedOpcode::LoadImmediate {
                destination: Target::Register(register),
            } => assert_eq!(register, expected),
            _ => panic!("opcode {byte:#04X} did not decode as an immediate register load"),
        }
    }
}

#[test]
fn word_loads_decode_to_word_destinations() {
    for (byte, expected) in [
        (0x01, Register16::BC),
        (0x11, Register16::DE),
        (0x21, Register16::HL),
        (0x31, Register16::SP),
    ] {
        match DecodedOpcode::try_from(byte).unwrap() {
            DecodedOpcode::LoadImmediate16 { destination } => assert_eq!(destination, expected),
            _ => panic!("opcode {byte:#04X} did not decode as a word load"),
        }
    }
}

#[test]
fn base_decoder_rejects_exactly_the_eleven_illegal_bytes() {
    const ILLEGAL: [u8; 11] = [
        0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
    ];
    for byte in 0u8..=255 {
        match DecodedOpcode::try_from(byte) {
            Ok(_) => assert!(!ILLEGAL.contains(&byte), "accepted illegal byte {byte:02X}"),
            Err(super::IllegalOpcode(actual)) => {
                assert!(ILLEGAL.contains(&byte), "rejected legal byte {byte:02X}");
                assert_eq!(actual, byte);
            }
        }
    }
}
#[test]
fn memory_inc_dec_opcodes_have_the_correct_operation() {
    assert!(matches!(
        DecodedOpcode::try_from(0x34).unwrap(),
        DecodedOpcode::Unary {
            operation: UnaryOperation::Increment,
            target: Target::IndirectHL
        }
    ));
    assert!(matches!(
        DecodedOpcode::try_from(0x35).unwrap(),
        DecodedOpcode::Unary {
            operation: UnaryOperation::Decrement,
            target: Target::IndirectHL
        }
    ));
}
