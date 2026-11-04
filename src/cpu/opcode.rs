use crate::cpu::{RegisterValue, registers::WordValue};

#[cfg(test)]
mod tests;

pub(super) enum DecodedOpcode {
    Nop,
    LoadRegister {
        source: RegisterValue,
        destination: RegisterValue,
    },
    LoadImmediate {
        destination: RegisterValue,
    },
    LoadImmediateWord {
        destination: WordValue,
    },
    LoadFromHL {
        destination: RegisterValue,
    },
    StoreToHL {
        source: RegisterValue,
    },
    StoreImmediateHL,
    Increment(RegisterValue),
    Decrement(RegisterValue),
    IncrementHL,
    DecrementHL,
    AddRegister(RegisterValue),
    SubRegister(RegisterValue),
    AddImmediate,
    SubImmediate,
    AddHL,
    SubHL,
    AddRegisterCarry(RegisterValue),
    SubRegisterCarry(RegisterValue),
    AddImmediateCarry,
    SubImmediateCarry,
    AddHLCarry,
    SubHLCarry,
    Compare(RegisterValue),
    CompareHL,
    CompareImmediate,
    Unknown,
}

impl From<u8> for DecodedOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => DecodedOpcode::Nop,
            0x01 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::BC,
            },
            0x04 => DecodedOpcode::Increment(RegisterValue::B),
            0x05 => DecodedOpcode::Decrement(RegisterValue::B),
            0x06 => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::B,
            },
            0x0C => DecodedOpcode::Increment(RegisterValue::C),
            0x0D => DecodedOpcode::Decrement(RegisterValue::C),
            0x0E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::C,
            },
            0x11 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::DE,
            },
            0x14 => DecodedOpcode::Increment(RegisterValue::D),
            0x15 => DecodedOpcode::Decrement(RegisterValue::D),
            0x16 => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::D,
            },
            0x1C => DecodedOpcode::Increment(RegisterValue::E),
            0x1D => DecodedOpcode::Decrement(RegisterValue::E),
            0x1E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::E,
            },
            0x21 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::HL,
            },
            0x24 => DecodedOpcode::Increment(RegisterValue::H),
            0x25 => DecodedOpcode::Decrement(RegisterValue::H),
            0x26 => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::H,
            },
            0x2C => DecodedOpcode::Increment(RegisterValue::L),
            0x2D => DecodedOpcode::Decrement(RegisterValue::L),
            0x2E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::L,
            },
            0x31 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::SP,
            },
            0x34 => DecodedOpcode::IncrementHL,
            0x35 => DecodedOpcode::DecrementHL,
            0x36 => DecodedOpcode::StoreImmediateHL,
            0x3C => DecodedOpcode::Increment(RegisterValue::A),
            0x3D => DecodedOpcode::Decrement(RegisterValue::A),
            0x3E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::A,
            },
            0x40 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::B,
            },
            0x41 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::B,
            },
            0x42 => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::B,
            },
            0x43 => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::B,
            },
            0x44 => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::B,
            },
            0x45 => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::B,
            },
            0x46 => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::B,
            },
            0x47 => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::B,
            },
            0x48 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::C,
            },
            0x49 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::C,
            },
            0x4A => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::C,
            },
            0x4B => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::C,
            },
            0x4C => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::C,
            },
            0x4D => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::C,
            },
            0x4E => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::C,
            },
            0x4F => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::C,
            },
            0x50 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::D,
            },
            0x51 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::D,
            },
            0x52 => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::D,
            },
            0x53 => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::D,
            },
            0x54 => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::D,
            },
            0x55 => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::D,
            },
            0x56 => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::D,
            },
            0x57 => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::D,
            },
            0x58 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::E,
            },
            0x59 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::E,
            },
            0x5A => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::E,
            },
            0x5B => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::E,
            },
            0x5C => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::E,
            },
            0x5D => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::E,
            },
            0x5E => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::E,
            },
            0x5F => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::E,
            },
            0x60 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::H,
            },
            0x61 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::H,
            },
            0x62 => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::H,
            },
            0x63 => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::H,
            },
            0x64 => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::H,
            },
            0x65 => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::H,
            },
            0x66 => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::H,
            },
            0x67 => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::H,
            },
            0x68 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::L,
            },
            0x69 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::L,
            },
            0x6A => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::L,
            },
            0x6B => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::L,
            },
            0x6C => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::L,
            },
            0x6D => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::L,
            },
            0x6E => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::L,
            },
            0x6F => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::L,
            },
            0x70 => DecodedOpcode::StoreToHL {
                source: RegisterValue::B,
            },
            0x71 => DecodedOpcode::StoreToHL {
                source: RegisterValue::C,
            },
            0x72 => DecodedOpcode::StoreToHL {
                source: RegisterValue::D,
            },
            0x73 => DecodedOpcode::StoreToHL {
                source: RegisterValue::E,
            },
            0x74 => DecodedOpcode::StoreToHL {
                source: RegisterValue::H,
            },
            0x75 => DecodedOpcode::StoreToHL {
                source: RegisterValue::L,
            },
            0x77 => DecodedOpcode::StoreToHL {
                source: RegisterValue::A,
            },
            0x78 => DecodedOpcode::LoadRegister {
                source: RegisterValue::B,
                destination: RegisterValue::A,
            },
            0x79 => DecodedOpcode::LoadRegister {
                source: RegisterValue::C,
                destination: RegisterValue::A,
            },
            0x7A => DecodedOpcode::LoadRegister {
                source: RegisterValue::D,
                destination: RegisterValue::A,
            },
            0x7B => DecodedOpcode::LoadRegister {
                source: RegisterValue::E,
                destination: RegisterValue::A,
            },
            0x7C => DecodedOpcode::LoadRegister {
                source: RegisterValue::H,
                destination: RegisterValue::A,
            },
            0x7D => DecodedOpcode::LoadRegister {
                source: RegisterValue::L,
                destination: RegisterValue::A,
            },
            0x7E => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::A,
            },
            0x7F => DecodedOpcode::LoadRegister {
                source: RegisterValue::A,
                destination: RegisterValue::A,
            },
            0x80 => DecodedOpcode::AddRegister(RegisterValue::B),
            0x81 => DecodedOpcode::AddRegister(RegisterValue::C),
            0x82 => DecodedOpcode::AddRegister(RegisterValue::D),
            0x83 => DecodedOpcode::AddRegister(RegisterValue::E),
            0x84 => DecodedOpcode::AddRegister(RegisterValue::H),
            0x85 => DecodedOpcode::AddRegister(RegisterValue::L),
            0x86 => DecodedOpcode::AddHL,
            0x87 => DecodedOpcode::AddRegister(RegisterValue::A),
            0x88 => DecodedOpcode::AddRegisterCarry(RegisterValue::B),
            0x89 => DecodedOpcode::AddRegisterCarry(RegisterValue::C),
            0x8A => DecodedOpcode::AddRegisterCarry(RegisterValue::D),
            0x8B => DecodedOpcode::AddRegisterCarry(RegisterValue::E),
            0x8C => DecodedOpcode::AddRegisterCarry(RegisterValue::H),
            0x8D => DecodedOpcode::AddRegisterCarry(RegisterValue::L),
            0x8E => DecodedOpcode::AddHLCarry,
            0x8F => DecodedOpcode::AddRegisterCarry(RegisterValue::A),
            0x90 => DecodedOpcode::SubRegister(RegisterValue::B),
            0x91 => DecodedOpcode::SubRegister(RegisterValue::C),
            0x92 => DecodedOpcode::SubRegister(RegisterValue::D),
            0x93 => DecodedOpcode::SubRegister(RegisterValue::E),
            0x94 => DecodedOpcode::SubRegister(RegisterValue::H),
            0x95 => DecodedOpcode::SubRegister(RegisterValue::L),
            0x96 => DecodedOpcode::SubHL,
            0x97 => DecodedOpcode::SubRegister(RegisterValue::A),
            0x98 => DecodedOpcode::SubRegisterCarry(RegisterValue::B),
            0x99 => DecodedOpcode::SubRegisterCarry(RegisterValue::C),
            0x9A => DecodedOpcode::SubRegisterCarry(RegisterValue::D),
            0x9B => DecodedOpcode::SubRegisterCarry(RegisterValue::E),
            0x9C => DecodedOpcode::SubRegisterCarry(RegisterValue::H),
            0x9D => DecodedOpcode::SubRegisterCarry(RegisterValue::L),
            0x9E => DecodedOpcode::SubHLCarry,
            0x9F => DecodedOpcode::SubRegisterCarry(RegisterValue::A),
            0xB8 => DecodedOpcode::Compare(RegisterValue::B),
            0xB9 => DecodedOpcode::Compare(RegisterValue::C),
            0xBA => DecodedOpcode::Compare(RegisterValue::D),
            0xBB => DecodedOpcode::Compare(RegisterValue::E),
            0xBC => DecodedOpcode::Compare(RegisterValue::H),
            0xBD => DecodedOpcode::Compare(RegisterValue::L),
            0xBE => DecodedOpcode::CompareHL,
            0xBF => DecodedOpcode::Compare(RegisterValue::A),
            0xC6 => DecodedOpcode::AddImmediate,
            0xCE => DecodedOpcode::AddImmediateCarry,
            0xD6 => DecodedOpcode::SubImmediate,
            0xDE => DecodedOpcode::SubImmediateCarry,
            0xFE => DecodedOpcode::CompareImmediate,
            _ => DecodedOpcode::Unknown,
        }
    }
}
