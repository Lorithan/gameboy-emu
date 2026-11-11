use crate::cpu::{
    Register,
    registers::{Flag, Flags, Register16},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Operand {
    Target(Target),
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArithmeticKind {
    Add { carry: bool },
    Sub { carry: bool },
    And,
    Xor,
    Or,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RotationDirection {
    Right { through_carry: bool },
    Left { through_carry: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IndirectPair {
    BC,
    DE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HighMemoryAddressSource {
    C,
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Unary16 {
        operation: UnaryOperation,
        target: Register16,
    },
    Arithmetic {
        kind: ArithmeticKind,
        operand: Operand,
    },
    AddHL {
        target: Register16,
    },
    ComplementAccumulator,
    SetCarry,
    ComplementCarry,
    Rotate {
        direction: RotationDirection,
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
    Call {
        condition: JumpCondition,
    },
    Return,
    ConditionalReturn {
        condition: JumpCondition,
    },
    Restart {
        target: u16,
    },
    DecimalAdjust,
    LoadHlFromSpOffset,
    AddSignedOffsetToSp,
    LoadAFromIndirect {
        source: IndirectPair,
    },
    StoreAToIndirect {
        destination: IndirectPair,
    },
    LoadAFromHL {
        operation: UnaryOperation,
    },
    StoreAToHL {
        operation: UnaryOperation,
    },
    LoadAFromAbsolute,
    StoreAToAbsolute,
    LoadHighMemory {
        address_source: HighMemoryAddressSource,
    },
    StoreHighMemory {
        address_source: HighMemoryAddressSource,
    },
    Unknown,
}

impl From<u8> for DecodedOpcode {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => DecodedOpcode::Nop,
            0x01 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::BC,
            },
            0x02 => DecodedOpcode::StoreAToIndirect {
                destination: IndirectPair::BC,
            },
            0x03 => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::BC,
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
            0x07 => DecodedOpcode::Rotate {
                direction: RotationDirection::Left {
                    through_carry: false,
                },
            },
            0x09 => DecodedOpcode::AddHL {
                target: Register16::BC,
            },
            0x0A => DecodedOpcode::LoadAFromIndirect {
                source: IndirectPair::BC,
            },
            0x0B => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::BC,
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
            0x0F => DecodedOpcode::Rotate {
                direction: RotationDirection::Right {
                    through_carry: false,
                },
            },
            0x11 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::DE,
            },
            0x12 => DecodedOpcode::StoreAToIndirect {
                destination: IndirectPair::DE,
            },
            0x13 => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::DE,
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
            0x17 => DecodedOpcode::Rotate {
                direction: RotationDirection::Left {
                    through_carry: true,
                },
            },
            0x18 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::Always,
            },
            0x19 => DecodedOpcode::AddHL {
                target: Register16::DE,
            },
            0x1A => DecodedOpcode::LoadAFromIndirect {
                source: IndirectPair::DE,
            },
            0x1B => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::DE,
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
            0x1F => DecodedOpcode::Rotate {
                direction: RotationDirection::Right {
                    through_carry: true,
                },
            },
            0x20 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::NZ,
            },
            0x21 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::HL,
            },
            0x23 => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::HL,
            },
            0x22 => DecodedOpcode::StoreAToHL {
                operation: UnaryOperation::Increment,
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
            0x27 => DecodedOpcode::DecimalAdjust,
            0x28 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::Z,
            },
            0x29 => DecodedOpcode::AddHL {
                target: Register16::HL,
            },
            0x2A => DecodedOpcode::LoadAFromHL {
                operation: UnaryOperation::Increment,
            },
            0x2B => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::HL,
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
            0x2F => DecodedOpcode::ComplementAccumulator,
            0x30 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::NC,
            },
            0x31 => DecodedOpcode::LoadImmediate16 {
                destination: Register16::SP,
            },
            0x32 => DecodedOpcode::StoreAToHL {
                operation: UnaryOperation::Decrement,
            },
            0x33 => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::SP,
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
            0x37 => DecodedOpcode::SetCarry,
            0x38 => DecodedOpcode::RelativeJump {
                condition: JumpCondition::C,
            },
            0x39 => DecodedOpcode::AddHL {
                target: Register16::SP,
            },
            0x3A => DecodedOpcode::LoadAFromHL {
                operation: UnaryOperation::Decrement,
            },
            0x3B => DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::SP,
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
            0x3F => DecodedOpcode::ComplementCarry,
            0x76 => DecodedOpcode::Unknown, // HALT currently unimplemented
            0x40..=0x7F => DecodedOpcode::Load {
                destination: Target::from_bits(byte >> 3),
                source: Target::from_bits(byte),
            },
            0x80..=0xBF => {
                let kind = match (byte >> 3) & 0b111 {
                    0 => ArithmeticKind::Add { carry: false },
                    1 => ArithmeticKind::Add { carry: true },
                    2 => ArithmeticKind::Sub { carry: false },
                    3 => ArithmeticKind::Sub { carry: true },
                    4 => ArithmeticKind::And,
                    5 => ArithmeticKind::Xor,
                    6 => ArithmeticKind::Or,
                    7 => ArithmeticKind::Compare,
                    _ => return DecodedOpcode::Unknown,
                };

                DecodedOpcode::Arithmetic {
                    kind,
                    operand: Operand::Target(Target::from_bits(byte)),
                }
            }
            0xC0 => DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::NZ,
            },
            0xC1 => DecodedOpcode::Pop {
                operand: PushOperand::BC,
            },
            0xC2 => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::NZ,
            },
            0xC3 => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::Always,
            },
            0xC4 => DecodedOpcode::Call {
                condition: JumpCondition::NZ,
            },
            0xC5 => DecodedOpcode::Push {
                operand: PushOperand::BC,
            },
            0xC6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: false },
                operand: Operand::Immediate,
            },
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                let target = (byte & 0x38) as u16;
                DecodedOpcode::Restart { target }
            }
            0xC8 => DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::Z,
            },
            0xC9 => DecodedOpcode::Return,
            0xCA => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::Z,
            },
            0xCC => DecodedOpcode::Call {
                condition: JumpCondition::Z,
            },
            0xCD => DecodedOpcode::Call {
                condition: JumpCondition::Always,
            },
            0xCE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: true },
                operand: Operand::Immediate,
            },
            0xD0 => DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::NC,
            },
            0xD1 => DecodedOpcode::Pop {
                operand: PushOperand::DE,
            },
            0xD2 => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::NC,
            },
            0xD4 => DecodedOpcode::Call {
                condition: JumpCondition::NC,
            },
            0xD5 => DecodedOpcode::Push {
                operand: PushOperand::DE,
            },
            0xD6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: false },
                operand: Operand::Immediate,
            },
            0xD8 => DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::C,
            },
            0xDA => DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::C,
            },
            0xDC => DecodedOpcode::Call {
                condition: JumpCondition::C,
            },
            0xDE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: true },
                operand: Operand::Immediate,
            },
            0xE0 => DecodedOpcode::StoreHighMemory {
                address_source: HighMemoryAddressSource::Immediate,
            },
            0xE1 => DecodedOpcode::Pop {
                operand: PushOperand::HL,
            },
            0xE2 => DecodedOpcode::StoreHighMemory {
                address_source: HighMemoryAddressSource::C,
            },
            0xE5 => DecodedOpcode::Push {
                operand: PushOperand::HL,
            },
            0xE6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::And,
                operand: Operand::Immediate,
            },
            0xE8 => DecodedOpcode::AddSignedOffsetToSp,
            0xE9 => DecodedOpcode::JumpHL,
            0xEA => DecodedOpcode::StoreAToAbsolute,
            0xEE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Xor,
                operand: Operand::Immediate,
            },
            0xF0 => DecodedOpcode::LoadHighMemory {
                address_source: HighMemoryAddressSource::Immediate,
            },
            0xF1 => DecodedOpcode::Pop {
                operand: PushOperand::AF,
            },
            0xF2 => DecodedOpcode::LoadHighMemory {
                address_source: HighMemoryAddressSource::C,
            },
            0xF5 => DecodedOpcode::Push {
                operand: PushOperand::AF,
            },
            0xF6 => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Or,
                operand: Operand::Immediate,
            },
            0xF8 => DecodedOpcode::LoadHlFromSpOffset,
            0xFA => DecodedOpcode::LoadAFromAbsolute,
            0xFE => DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Compare,
                operand: Operand::Immediate,
            },
            _ => DecodedOpcode::Unknown,
        }
    }
}
