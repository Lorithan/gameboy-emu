use crate::cpu::{
    Register,
    error::IllegalOpcode,
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
    TransferHLtoSP,
    StoreStackPointer,
    DecodeCbOpcode,
    DisableInterrupts,
    EnabledInterrupts,
    ReturnFromInterrupt,
    Halt,

    Stop,
}

impl TryFrom<u8> for DecodedOpcode {
    type Error = IllegalOpcode;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(DecodedOpcode::Nop),
            0x01 => Ok(DecodedOpcode::LoadImmediate16 {
                destination: Register16::BC,
            }),
            0x02 => Ok(DecodedOpcode::StoreAToIndirect {
                destination: IndirectPair::BC,
            }),
            0x03 => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::BC,
            }),
            0x04 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::B),
            }),
            0x05 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::B),
            }),
            0x06 => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::B),
            }),
            0x07 => Ok(DecodedOpcode::Rotate {
                direction: RotationDirection::Left {
                    through_carry: false,
                },
            }),
            0x08 => Ok(DecodedOpcode::StoreStackPointer),
            0x09 => Ok(DecodedOpcode::AddHL {
                target: Register16::BC,
            }),
            0x0A => Ok(DecodedOpcode::LoadAFromIndirect {
                source: IndirectPair::BC,
            }),
            0x0B => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::BC,
            }),
            0x0C => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::C),
            }),
            0x0D => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::C),
            }),
            0x0E => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::C),
            }),
            0x0F => Ok(DecodedOpcode::Rotate {
                direction: RotationDirection::Right {
                    through_carry: false,
                },
            }),
            0x10 => Ok(DecodedOpcode::Stop),
            0x11 => Ok(DecodedOpcode::LoadImmediate16 {
                destination: Register16::DE,
            }),
            0x12 => Ok(DecodedOpcode::StoreAToIndirect {
                destination: IndirectPair::DE,
            }),
            0x13 => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::DE,
            }),
            0x14 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::D),
            }),
            0x15 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::D),
            }),
            0x16 => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::D),
            }),
            0x17 => Ok(DecodedOpcode::Rotate {
                direction: RotationDirection::Left {
                    through_carry: true,
                },
            }),
            0x18 => Ok(DecodedOpcode::RelativeJump {
                condition: JumpCondition::Always,
            }),
            0x19 => Ok(DecodedOpcode::AddHL {
                target: Register16::DE,
            }),
            0x1A => Ok(DecodedOpcode::LoadAFromIndirect {
                source: IndirectPair::DE,
            }),
            0x1B => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::DE,
            }),
            0x1C => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::E),
            }),
            0x1D => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::E),
            }),
            0x1E => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::E),
            }),
            0x1F => Ok(DecodedOpcode::Rotate {
                direction: RotationDirection::Right {
                    through_carry: true,
                },
            }),
            0x20 => Ok(DecodedOpcode::RelativeJump {
                condition: JumpCondition::NZ,
            }),
            0x21 => Ok(DecodedOpcode::LoadImmediate16 {
                destination: Register16::HL,
            }),
            0x23 => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::HL,
            }),
            0x22 => Ok(DecodedOpcode::StoreAToHL {
                operation: UnaryOperation::Increment,
            }),
            0x24 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::H),
            }),
            0x25 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::H),
            }),
            0x26 => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::H),
            }),
            0x27 => Ok(DecodedOpcode::DecimalAdjust),
            0x28 => Ok(DecodedOpcode::RelativeJump {
                condition: JumpCondition::Z,
            }),
            0x29 => Ok(DecodedOpcode::AddHL {
                target: Register16::HL,
            }),
            0x2A => Ok(DecodedOpcode::LoadAFromHL {
                operation: UnaryOperation::Increment,
            }),
            0x2B => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::HL,
            }),
            0x2C => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::L),
            }),
            0x2D => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::L),
            }),
            0x2E => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::L),
            }),
            0x2F => Ok(DecodedOpcode::ComplementAccumulator),
            0x30 => Ok(DecodedOpcode::RelativeJump {
                condition: JumpCondition::NC,
            }),
            0x31 => Ok(DecodedOpcode::LoadImmediate16 {
                destination: Register16::SP,
            }),
            0x32 => Ok(DecodedOpcode::StoreAToHL {
                operation: UnaryOperation::Decrement,
            }),
            0x33 => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Increment,
                target: Register16::SP,
            }),
            0x34 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::IndirectHL,
            }),
            0x35 => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::IndirectHL,
            }),
            0x36 => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::IndirectHL,
            }),
            0x37 => Ok(DecodedOpcode::SetCarry),
            0x38 => Ok(DecodedOpcode::RelativeJump {
                condition: JumpCondition::C,
            }),
            0x39 => Ok(DecodedOpcode::AddHL {
                target: Register16::SP,
            }),
            0x3A => Ok(DecodedOpcode::LoadAFromHL {
                operation: UnaryOperation::Decrement,
            }),
            0x3B => Ok(DecodedOpcode::Unary16 {
                operation: UnaryOperation::Decrement,
                target: Register16::SP,
            }),
            0x3C => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Increment,
                target: Target::Register(Register::A),
            }),
            0x3D => Ok(DecodedOpcode::Unary {
                operation: UnaryOperation::Decrement,
                target: Target::Register(Register::A),
            }),
            0x3E => Ok(DecodedOpcode::LoadImmediate {
                destination: Target::Register(Register::A),
            }),
            0x3F => Ok(DecodedOpcode::ComplementCarry),
            0x76 => Ok(DecodedOpcode::Halt),
            0x40..=0x7F => Ok(DecodedOpcode::Load {
                destination: Target::from_bits(byte >> 3),
                source: Target::from_bits(byte),
            }),
            0x80..=0xBF => {
                let kind = match (byte >> 3) & 0b111 {
                    0 => ArithmeticKind::Add { carry: false },
                    1 => ArithmeticKind::Add { carry: true },
                    2 => ArithmeticKind::Sub { carry: false },
                    3 => ArithmeticKind::Sub { carry: true },
                    4 => ArithmeticKind::And,
                    5 => ArithmeticKind::Xor,
                    6 => ArithmeticKind::Or,
                    _ => ArithmeticKind::Compare,
                };

                Ok(DecodedOpcode::Arithmetic {
                    kind,
                    operand: Operand::Target(Target::from_bits(byte)),
                })
            }
            0xC0 => Ok(DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::NZ,
            }),
            0xC1 => Ok(DecodedOpcode::Pop {
                operand: PushOperand::BC,
            }),
            0xC2 => Ok(DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::NZ,
            }),
            0xC3 => Ok(DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::Always,
            }),
            0xC4 => Ok(DecodedOpcode::Call {
                condition: JumpCondition::NZ,
            }),
            0xC5 => Ok(DecodedOpcode::Push {
                operand: PushOperand::BC,
            }),
            0xC6 => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: false },
                operand: Operand::Immediate,
            }),
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                let target = (byte & 0x38) as u16;
                Ok(DecodedOpcode::Restart { target })
            }
            0xC8 => Ok(DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::Z,
            }),
            0xC9 => Ok(DecodedOpcode::Return),
            0xCA => Ok(DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::Z,
            }),
            0xCB => Ok(DecodedOpcode::DecodeCbOpcode),
            0xCC => Ok(DecodedOpcode::Call {
                condition: JumpCondition::Z,
            }),
            0xCD => Ok(DecodedOpcode::Call {
                condition: JumpCondition::Always,
            }),
            0xCE => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Add { carry: true },
                operand: Operand::Immediate,
            }),
            0xD0 => Ok(DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::NC,
            }),
            0xD1 => Ok(DecodedOpcode::Pop {
                operand: PushOperand::DE,
            }),
            0xD2 => Ok(DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::NC,
            }),
            0xD4 => Ok(DecodedOpcode::Call {
                condition: JumpCondition::NC,
            }),
            0xD5 => Ok(DecodedOpcode::Push {
                operand: PushOperand::DE,
            }),
            0xD6 => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: false },
                operand: Operand::Immediate,
            }),
            0xD8 => Ok(DecodedOpcode::ConditionalReturn {
                condition: JumpCondition::C,
            }),
            0xD9 => Ok(DecodedOpcode::ReturnFromInterrupt),
            0xDA => Ok(DecodedOpcode::AbsoluteJump {
                condition: JumpCondition::C,
            }),
            0xDC => Ok(DecodedOpcode::Call {
                condition: JumpCondition::C,
            }),
            0xDE => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Sub { carry: true },
                operand: Operand::Immediate,
            }),
            0xE0 => Ok(DecodedOpcode::StoreHighMemory {
                address_source: HighMemoryAddressSource::Immediate,
            }),
            0xE1 => Ok(DecodedOpcode::Pop {
                operand: PushOperand::HL,
            }),
            0xE2 => Ok(DecodedOpcode::StoreHighMemory {
                address_source: HighMemoryAddressSource::C,
            }),
            0xE5 => Ok(DecodedOpcode::Push {
                operand: PushOperand::HL,
            }),
            0xE6 => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::And,
                operand: Operand::Immediate,
            }),
            0xE8 => Ok(DecodedOpcode::AddSignedOffsetToSp),
            0xE9 => Ok(DecodedOpcode::JumpHL),
            0xEA => Ok(DecodedOpcode::StoreAToAbsolute),
            0xEE => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Xor,
                operand: Operand::Immediate,
            }),
            0xF0 => Ok(DecodedOpcode::LoadHighMemory {
                address_source: HighMemoryAddressSource::Immediate,
            }),
            0xF1 => Ok(DecodedOpcode::Pop {
                operand: PushOperand::AF,
            }),
            0xF2 => Ok(DecodedOpcode::LoadHighMemory {
                address_source: HighMemoryAddressSource::C,
            }),
            0xF3 => Ok(DecodedOpcode::DisableInterrupts),
            0xF5 => Ok(DecodedOpcode::Push {
                operand: PushOperand::AF,
            }),
            0xF6 => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Or,
                operand: Operand::Immediate,
            }),
            0xF8 => Ok(DecodedOpcode::LoadHlFromSpOffset),
            0xF9 => Ok(DecodedOpcode::TransferHLtoSP),
            0xFA => Ok(DecodedOpcode::LoadAFromAbsolute),
            0xFB => Ok(DecodedOpcode::EnabledInterrupts),
            0xFE => Ok(DecodedOpcode::Arithmetic {
                kind: ArithmeticKind::Compare,
                operand: Operand::Immediate,
            }),
            _ => Err(IllegalOpcode(byte)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShiftKind {
    LeftArithmetic,
    RightArithmetic,
    RightLogical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CbTransform {
    Rotate { direction: RotationDirection },
    Shift { kind: ShiftKind },
    Swap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CbType {
    Transform(CbTransform),
    Bit(u8),
    Clear(u8),
    Set(u8),
}

impl From<u8> for CbType {
    fn from(bits: u8) -> Self {
        let family = bits >> 6;
        let selector = (bits >> 3) & 0b111;
        match family {
            0 => {
                let transform = match selector {
                    0 => CbTransform::Rotate {
                        direction: RotationDirection::Left {
                            through_carry: false,
                        },
                    },
                    1 => CbTransform::Rotate {
                        direction: RotationDirection::Right {
                            through_carry: false,
                        },
                    },
                    2 => CbTransform::Rotate {
                        direction: RotationDirection::Left {
                            through_carry: true,
                        },
                    },
                    3 => CbTransform::Rotate {
                        direction: RotationDirection::Right {
                            through_carry: true,
                        },
                    },
                    4 => CbTransform::Shift {
                        kind: ShiftKind::LeftArithmetic,
                    },
                    5 => CbTransform::Shift {
                        kind: ShiftKind::RightArithmetic,
                    },
                    6 => CbTransform::Swap,
                    7 => CbTransform::Shift {
                        kind: ShiftKind::RightLogical,
                    },
                    _ => unreachable!(),
                };
                Self::Transform(transform)
            }
            1 => Self::Bit(selector),
            2 => Self::Clear(selector),
            _ => Self::Set(selector),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DecodedCbOpcode {
    pub(super) target: Target,
    pub(super) cb_type: CbType,
}

impl From<u8> for DecodedCbOpcode {
    fn from(value: u8) -> Self {
        let target = Target::from_bits(value);
        let cb_type = CbType::from(value);

        Self { target, cb_type }
    }
}
