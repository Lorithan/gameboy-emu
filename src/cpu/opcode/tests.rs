use super::{DecodedOpcode, RegisterValue, WordValue};

#[test]
fn hl_memory_opcodes_decode_to_the_expected_direction_and_register() {
    assert!(matches!(
        DecodedOpcode::from(0x36),
        DecodedOpcode::StoreImmediateHL
    ));
    assert!(matches!(
        DecodedOpcode::from(0x77),
        DecodedOpcode::StoreToHL {
            source: RegisterValue::A
        }
    ));
    assert!(matches!(
        DecodedOpcode::from(0x7E),
        DecodedOpcode::LoadFromHL {
            destination: RegisterValue::A
        }
    ));
}

#[test]
fn known_opcode_bytes_decode_to_the_expected_operation_and_destination() {
    assert!(matches!(DecodedOpcode::from(0), DecodedOpcode::Nop));
    for (byte, expected) in [
        (0x06, RegisterValue::B),
        (0x0E, RegisterValue::C),
        (0x16, RegisterValue::D),
        (0x1E, RegisterValue::E),
        (0x26, RegisterValue::H),
        (0x2E, RegisterValue::L),
        (0x3E, RegisterValue::A),
    ] {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::LoadImmediate { destination } => assert_eq!(destination, expected),
            _ => panic!("opcode {byte:#04X} did not decode as an immediate register load"),
        }
    }
}

#[test]
fn word_loads_decode_to_word_destinations() {
    for (byte, expected) in [
        (0x01, WordValue::BC),
        (0x11, WordValue::DE),
        (0x21, WordValue::HL),
        (0x31, WordValue::SP),
    ] {
        match DecodedOpcode::from(byte) {
            DecodedOpcode::LoadImmediateWord { destination } => assert_eq!(destination, expected),
            _ => panic!("opcode {byte:#04X} did not decode as a word load"),
        }
    }
}

#[test]
fn every_other_byte_remains_explicitly_unknown() {
    for byte in 0u8..=u8::MAX {
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
        if [
            0x00, 0x01, 0x04, 0x05, 0x06, 0x0C, 0x0D, 0x0E, 0x11, 0x14, 0x15, 0x16, 0x1C, 0x1D,
            0x1E, 0x21, 0x24, 0x25, 0x26, 0x2C, 0x2D, 0x2E, 0x31, 0x34, 0x35, 0x36, 0x3C, 0x3D,
            0x3E,
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
        DecodedOpcode::IncrementHL
    ));
    assert!(matches!(
        DecodedOpcode::from(0x35),
        DecodedOpcode::DecrementHL
    ));
}
