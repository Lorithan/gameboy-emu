use crate::cpu::{RegisterValue, registers::WordValue};

#[cfg(test)]
mod tests;

pub(super) enum DecodedOpcode {
    Nop,
    LoadImmediate { destination: RegisterValue },
    LoadImmediateWord { destination: WordValue },
    LoadFromHL { destination: RegisterValue },
    StoreToHL { from: RegisterValue },
    StoreImmediateHL,
    Unknown,
}

impl From<u8> for DecodedOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => DecodedOpcode::Nop,
            0x01 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::BC,
            },
            0x11 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::DE,
            },
            0x06 => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::B,
            },
            0x0E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::C,
            },
            0x16 => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::D,
            },
            0x1E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::E,
            },
            0x21 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::HL,
            },
            0x26 => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::H,
            },
            0x2E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::L,
            },
            0x31 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::SP,
            },
            0x36 => DecodedOpcode::StoreImmediateHL,
            0x3E => DecodedOpcode::LoadImmediate {
                destination: RegisterValue::A,
            },
            0x77 => DecodedOpcode::StoreToHL {
                from: RegisterValue::A,
            },
            0x7E => DecodedOpcode::LoadFromHL {
                destination: RegisterValue::A,
            },
            _ => DecodedOpcode::Unknown,
        }
    }
}
