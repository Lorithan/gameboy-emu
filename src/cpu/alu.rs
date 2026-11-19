use crate::cpu::{
    ArithmeticKind, CPU, CbTransform, Register, RotationDirection, ShiftKind, registers::Flag,
};

impl CPU {
    pub(super) fn increment(&mut self, mut value: u8) -> u8 {
        let h = (value & 0x0F) == 0x0F;

        value = value.wrapping_add(1);

        let z = value == 0;
        self.registers.set_flag(Flag::Z, z);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, h);

        value
    }

    pub(super) fn decrement(&mut self, mut value: u8) -> u8 {
        let h = (value & 0x0F) == 0x00;

        value = value.wrapping_sub(1);

        let z = value == 0;
        self.registers.set_flag(Flag::Z, z);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, h);

        value
    }

    fn add(&mut self, left: u8, right: u8, carry: bool) -> u8 {
        let (mut result, mut c) = left.overflowing_add(right);
        if carry {
            let (result2, c2) = result.overflowing_add(1);
            result = result2;
            c = c || c2;
        }
        let z = result == 0;
        let h = ((left ^ right ^ result) & 0x10) != 0;

        self.registers.set_flag(Flag::C, c);
        self.registers.set_flag(Flag::Z, z);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, h);

        result
    }

    fn sub(&mut self, left: u8, right: u8, carry: bool) -> u8 {
        let (mut result, mut c) = left.overflowing_sub(right);
        if carry {
            let (result2, c2) = result.overflowing_sub(1);
            result = result2;
            c = c || c2;
        }
        let z = result == 0;
        let h = ((left ^ right ^ result) & 0x10) != 0;

        self.registers.set_flag(Flag::C, c);
        self.registers.set_flag(Flag::Z, z);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, h);

        result
    }

    pub(super) fn rotate(&self, value: u8, direction: RotationDirection) -> (u8, bool) {
        let carry_in = self.registers.get_flag(Flag::C);

        let (result, carry_out) = match direction {
            RotationDirection::Right { through_carry } => {
                let bit0 = value & 1;
                let new_value = if through_carry {
                    if carry_in {
                        (value >> 1) | (1 << 7)
                    } else {
                        value >> 1
                    }
                } else {
                    (value >> 1) | (bit0 << 7)
                };

                (new_value, bit0 == 1)
            }
            RotationDirection::Left { through_carry } => {
                let bit7 = (value >> 7) & 1;
                let new_value = if through_carry {
                    if carry_in {
                        (value << 1) | 1
                    } else {
                        value << 1
                    }
                } else {
                    (value << 1) | bit7
                };

                (new_value, bit7 == 1)
            }
        };

        (result, carry_out)
    }

    fn shift(&self, value: u8, kind: ShiftKind) -> (u8, bool) {
        match kind {
            ShiftKind::LeftArithmetic => {
                let bit7 = (value >> 7) & 1;
                let new_value = value << 1;

                (new_value, bit7 == 1)
            }
            ShiftKind::RightArithmetic => {
                let bit7 = value & (1 << 7);
                let bit0 = value & 1;
                let new_value = (value >> 1) | (bit7);

                (new_value, bit0 == 1)
            }
            ShiftKind::RightLogical => {
                let bit0 = value & 1;
                let new_value = value >> 1;

                (new_value, bit0 == 1)
            }
        }
    }

    pub(super) fn execute_accumulator(&mut self, operation: ArithmeticKind, left: u8, right: u8) {
        match operation {
            ArithmeticKind::Add { carry } => {
                let result = self.add(left, right, carry);
                self.registers.set(Register::A, result);
            }
            ArithmeticKind::Sub { carry } => {
                let result = self.sub(left, right, carry);
                self.registers.set(Register::A, result);
            }
            ArithmeticKind::And => {
                let result = left & right;
                self.registers.set(Register::A, result);
                self.registers.set_flag(Flag::Z, result == 0);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, true);
                self.registers.set_flag(Flag::C, false);
            }
            ArithmeticKind::Xor => {
                let result = left ^ right;
                self.registers.set(Register::A, result);
                self.registers.set_flag(Flag::Z, result == 0);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::C, false);
            }
            ArithmeticKind::Or => {
                let result = left | right;
                self.registers.set(Register::A, result);
                self.registers.set_flag(Flag::Z, result == 0);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::C, false);
            }
            ArithmeticKind::Compare => {
                self.sub(left, right, false);
            }
        }
    }

    pub(super) fn execute_cb_transform(&mut self, value: u8, operation: CbTransform) -> u8 {
        match operation {
            CbTransform::Rotate { direction } => {
                let (result, carry_out) = self.rotate(value, direction);

                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, result == 0);

                result
            }
            CbTransform::Shift { kind } => {
                let (result, carry_out) = self.shift(value, kind);

                self.registers.set_flag(Flag::C, carry_out);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, result == 0);

                result
            }
            CbTransform::Swap => {
                let low_nibble = value & 0xF;
                let high_nibble = (value & 0xF0) >> 4;
                let result = low_nibble << 4 | high_nibble;

                self.registers.set_flag(Flag::C, false);
                self.registers.set_flag(Flag::N, false);
                self.registers.set_flag(Flag::H, false);
                self.registers.set_flag(Flag::Z, result == 0);

                result
            }
        }
    }
}
