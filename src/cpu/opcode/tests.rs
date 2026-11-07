use super::{DecodedOpcode, Register, Register16, Target, UnaryOperation};

#[test]
fn hl_memory_opcodes_decode_to_the_expected_direction_and_register() {
    assert!(matches!(
        DecodedOpcode::from(0x36),
        DecodedOpcode::LoadImmediate {
            destination: Target::IndirectHL
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0x77),
        DecodedOpcode::Load {
            destination: Target::IndirectHL,
            source: Target::Register(Register::A)
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0x7E),
        DecodedOpcode::Load {
            destination: Target::Register(Register::A),
            source: Target::IndirectHL
        }
    ));
}

#[test]
fn known_opcode_bytes_decode_to_the_expected_operation_and_destination() {
    assert!(matches!(DecodedOpcode::from(0), DecodedOpcode::Nop));
    for (byte, expected) in [
        (0x06, Register::B),
        (0x0E, Register::C),
        (0x16, Register::D),
        (0x1E, Register::E),
        (0x26, Register::H),
        (0x2E, Register::L),
        (0x3E, Register::A),
    ] {
        match DecodedOpcode::from(byte) {
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
        match DecodedOpcode::from(byte) {
            DecodedOpcode::LoadImmediate16 { destination } => assert_eq!(destination, expected),
            _ => panic!("opcode {byte:#04X} did not decode as a word load"),
        }
    }
}

#[test]
fn every_other_byte_remains_explicitly_unknown() {
    for byte in 0u8..=u8::MAX {
        if [0x2F, 0x37, 0x3F].contains(&byte) {
            continue;
        }
        if [0xC7, 0xCF, 0xD7, 0xDF, 0xE7, 0xEF, 0xF7, 0xFF].contains(&byte) {
            continue;
        }
        if [0xC0, 0xC8, 0xD0, 0xD8].contains(&byte) {
            continue;
        }
        if byte == 0xC9 {
            continue;
        }
        if [0xC4, 0xCC, 0xCD, 0xD4, 0xDC].contains(&byte) {
            continue;
        }
        if [0xC1, 0xC5, 0xD1, 0xD5, 0xE1, 0xE5, 0xF1, 0xF5].contains(&byte) {
            continue;
        }
        if [0xC2, 0xC3, 0xCA, 0xD2, 0xDA, 0xE9].contains(&byte) {
            continue;
        }
        if (0xA0..=0xBF).contains(&byte) || [0xE6, 0xEE, 0xF6, 0xFE].contains(&byte) {
            continue;
        }
        if (0x88..=0x8F).contains(&byte)
            || (0x98..=0x9F).contains(&byte)
            || [0xCE, 0xDE].contains(&byte)
        {
            continue;
        }
        if [0x86, 0x96, 0xC6, 0xD6].contains(&byte) {
            continue;
        }
        if (0x40..=0x7F).contains(&byte) && byte != 0x76 {
            continue;
        }
        if [
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x87, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x97,
        ]
        .contains(&byte)
        {
            continue;
        }
        if [0x20, 0x28, 0x30, 0x38].contains(&byte) {
            continue;
        }
        if [
            0x00, 0x01, 0x04, 0x05, 0x06, 0x0C, 0x0D, 0x0E, 0x11, 0x14, 0x15, 0x16, 0x18, 0x1C,
            0x1D, 0x1E, 0x21, 0x24, 0x25, 0x26, 0x2C, 0x2D, 0x2E, 0x31, 0x34, 0x35, 0x36, 0x3C,
            0x3D, 0x3E,
        ]
        .contains(&byte)
        {
            continue;
        }
        assert!(
            matches!(DecodedOpcode::from(byte), DecodedOpcode::Unknown),
            "unexpectedly accepted opcode {byte:#04X}"
        );
    }
}

#[test]
fn memory_inc_dec_opcodes_have_the_correct_operation() {
    assert!(matches!(
        DecodedOpcode::from(0x34),
        DecodedOpcode::Unary {
            operation: UnaryOperation::Increment,
            target: Target::IndirectHL
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0x35),
        DecodedOpcode::Unary {
            operation: UnaryOperation::Decrement,
            target: Target::IndirectHL
        }
    ));
}
