use crate::cpu::{Register, registers::WordValue};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub(super) enum Target {
    Register(Register),
    IndirectHL,
}

impl Target {
    const BY_INDEX: [Target; 8] = [
        Target::Register(Register::B),
        Target::Register(Register::C),
        Target::Register(Register::D),
        Target::Register(Register::E),
        Target::Register(Register::H),
        Target::Register(Register::L),
        Target::IndirectHL,
        Target::Register(Register::A),
    ];

    fn from_bits(bits: u8) -> Self {
        Self::BY_INDEX[(bits & 0b111) as usize]
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Operand {
    Target(Target),
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArithmeticKind {
    Add { carry: bool },
    Sub { carry: bool },
    Compare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnaryOperation {
    Increment,
    Decrement,
}

pub(super) enum DecodedOpcode {
    Nop,
    Load {
        destination: Target,
        source: Target,
    },
    LoadImmediate {
        destination: Target,
    },
    LoadImmediateWord {
        destination: WordValue,
    },
    Unary {
        operation: UnaryOperation,
        target: Target,
    },
    Arithmetic {
        kind: ArithmeticKind,
        operand: Operand,
    },
    Unknown,
}

impl From<u8> for DecodedOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => DecodedOpcode::Nop,
            0x01 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::BC,
            },
            0x04 => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::B),
            },
            0x05 => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::B),
            },
            0x06 => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::B),
            },
            0x0C => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::C),
            },
            0x0D => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::C),
            },
            0x0E => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::C),
            },
            0x11 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::DE,
            },
            0x14 => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::D),
            },
            0x15 => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::D),
            },
            0x16 => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::D),
            },
            0x1C => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::E),
            },
            0x1D => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::E),
            },
            0x1E => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::E),
            },
            0x21 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::HL,
            },
            0x24 => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::H),
            },
            0x25 => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::H),
            },
            0x26 => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::H),
            },
            0x2C => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::L),
            },
            0x2D => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::L),
            },
            0x2E => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::L),
            },
            0x31 => DecodedOpcode::LoadImmediateWord {
                destination: WordValue::SP,
            },
            0x34 => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::IndirectHL,
            },
            0x35 => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::IndirectHL,
            },
            0x36 => DecodedOpcode::LoadImmediate {
                destination: Target::IndirectHL,
            },
            0x3C => DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::A),
            },
            0x3D => DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::A),
            },
            0x3E => DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::A),
            },

            0x76 => DecodedOpcode::Unknown, // HALT currently unimplemented
            0x40..=0x7F => DecodedOpcode::Load {
                destination: Target::from_bits(value >> 3),
                source: Target::from_bits(value),
            },
            0x80..=0xBF => {
                let kind = match (value >> 3) & 0b111 {
                    0 => ArithmeticKind::Add { carry: false },
                    1 => ArithmeticKind::Add { carry: true },
                    2 => ArithmeticKind::Sub { carry: false },
                    3 => ArithmeticKind::Sub { carry: true },
                    7 => ArithmeticKind::Compare,
                    _ => return DecodedOpcode::Unknown,
                };

                DecodedOpcode::Arithmetic {
                    kind,
                    operand: Operand::Target(Target::from_bits(value)),
                }
            }
            0xC6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: false },
                operand: Operand::Immediate,
            },
            0xCE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: true },
                operand: Operand::Immediate,
            },
            0xD6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: false },
                operand: Operand::Immediate,
            },
            0xDE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: true },
                operand: Operand::Immediate,
            },
            0xFE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Compare,
                operand: Operand::Immediate,
            },
            _ => DecodedOpcode::Unknown,
        }
    }
}
