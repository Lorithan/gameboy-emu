use crate::cpu::{
    Register,
    registers::{Flag, Flags, Register16},
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JumpCondition {
    Always,
    Z,
    NZ,
    C,
    NC,
}

impl JumpCondition {
    pub(super) fn jump(&self, flag: Flags) -> bool {
        match self {
            JumpCondition::Always => true,
            JumpCondition::Z => flag.get(Flag::Z),
            JumpCondition::NZ => !flag.get(Flag::Z),
            JumpCondition::C => flag.get(Flag::C),
            JumpCondition::NC => !flag.get(Flag::C),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PushOperand {
    BC,
    DE,
    HL,
    AF,
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
    LoadImmediate16 {
        destination: Register16,
    },
    Unary {
        operation: UnaryOperation,
        target: Target,
    },
    Arithmetic {
        kind: ArithmeticKind,
        operand: Operand,
    },
    RelativeJump {
        condition: JumpCondition,
    },
    AbsoluteJump {
        condition: JumpCondition,
    },
    JumpHL,
    Push {
        operand: PushOperand,
    },
    Pop {
        operand: PushOperand,
    },
    Unknown,
}

impl From<u8> for DecodedOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => DecodedOpcode::Nop,
            0x01 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::BC,
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
            0x11 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::DE,
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
            0x18 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::Always,
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
            0x20 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::NZ,
            },
            0x21 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::HL,
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
            0x28 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::Z,
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
            0x30 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::NC,
            },
            0x31 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::SP,
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
            0x38 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::C,
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
            0xC1 => DecodedOpcode::Pop {
                operand: PushOperand::BC,
            },
            0xC2 => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::NZ,
            },
            0xC3 => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::Always,
            },
            0xC5 => DecodedOpcode::Push {
                operand: PushOperand::BC,
            },
            0xC6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: false },
                operand: Operand::Immediate,
            },
            0xCA => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::Z,
            },
            0xCE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: true },
                operand: Operand::Immediate,
            },
            0xD1 => DecodedOpcode::Pop {
                operand: PushOperand::DE,
            },
            0xD2 => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::NC,
            },
            0xD5 => DecodedOpcode::Push {
                operand: PushOperand::DE,
            },
            0xD6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: false },
                operand: Operand::Immediate,
            },
            0xDA => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::C,
            },
            0xDE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: true },
                operand: Operand::Immediate,
            },
            0xE1 => DecodedOpcode::Pop {
                operand: PushOperand::HL,
            },
            0xE5 => DecodedOpcode::Push {
                operand: PushOperand::HL,
            },
            0xE9 => DecodedOpcode::JumpHL,
            0xF1 => DecodedOpcode::Pop {
                operand: PushOperand::AF,
            },
            0xF5 => DecodedOpcode::Push {
                operand: PushOperand::AF,
            },
            0xFE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Compare,
                operand: Operand::Immediate,
            },
            _ => DecodedOpcode::Unknown,
        }
    }
}
