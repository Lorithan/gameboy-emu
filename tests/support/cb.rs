// Arithmetic oracle shared by public integration and private Bus-preservation tests.
pub fn result(opcode: u8, value: u8, flags: u8) -> (u8, u8) {
    let v = u16::from(value);
    let bit = (opcode % 64) / 8;
    let weight = 2u16.pow(u32::from(bit));
    let present = (v / weight) % 2 != 0;
    match opcode / 64 {
        0 => {
            let carry_in = u16::from(flags & 0x10 != 0);
            let (r, c) = match bit {
                0 => ((v * 2) % 256 + v / 128, v >= 128),
                1 => (v / 2 + (v % 2) * 128, v % 2 != 0),
                2 => ((v * 2) % 256 + carry_in, v >= 128),
                3 => (v / 2 + carry_in * 128, v % 2 != 0),
                4 => ((v * 2) % 256, v >= 128),
                5 => (v / 2 + (v / 128) * 128, v % 2 != 0),
                6 => ((v % 16) * 16 + v / 16, false),
                7 => (v / 2, v % 2 != 0),
                _ => unreachable!(),
            };
            (
                r as u8,
                if r == 0 { 0x80 } else { 0 } | if c { 0x10 } else { 0 },
            )
        }
        1 => (
            value,
            (flags & 0x10) + 0x20 + if present { 0 } else { 0x80 },
        ),
        2 => ((if present { v - weight } else { v }) as u8, flags),
        3 => ((if present { v } else { v + weight }) as u8, flags),
        _ => unreachable!(),
    }
}
