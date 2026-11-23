use gameboy_emu::video::{Frame, Shade};

pub const RGBA_LEN: usize = Frame::WIDTH * Frame::HEIGHT * 4;

pub fn shade_rgba(shade: Shade) -> [u8; 4] {
    match shade {
        Shade::White => [0xFF, 0xFF, 0xFF, 0xFF],
        Shade::LightGray => [0xAA, 0xAA, 0xAA, 0xFF],
        Shade::DarkGray => [0x55, 0x55, 0x55, 0xFF],
        Shade::Black => [0x00, 0x00, 0x00, 0xFF],
    }
}

pub fn write_rgba32(frame: &Frame, out: &mut [u8; RGBA_LEN]) {
    for (i, pixel) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = i % Frame::WIDTH;
        let y = i / Frame::WIDTH;
        let shade = frame.pixel(x, y);

        let color = shade_rgba(shade);
        pixel.copy_from_slice(&color);
    }
}

pub fn integer_viewport(window_w: u32, window_h: u32) -> (i32, i32, u32, u32) {
    let scale = u32::min(window_w / 160, window_h / 144).max(1);

    let width = Frame::WIDTH as u32 * scale;
    let height = Frame::HEIGHT as u32 * scale;

    let x = (window_w as i32 - width as i32) / 2;
    let y = (window_h as i32 - height as i32) / 2;

    (x, y, width, height)
}

#[cfg(test)]
mod tests;
