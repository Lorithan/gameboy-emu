#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Shade {
    #[default]
    White,
    LightGray,
    DarkGray,
    Black,
}

impl Shade {
    pub fn from_bits(value: u8) -> Self {
        match value & 3 {
            0 => Shade::White,
            1 => Shade::LightGray,
            2 => Shade::DarkGray,
            _ => Shade::Black,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Frame {
    frame: [Shade; Self::WIDTH * Self::HEIGHT],
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Frame {
    pub const WIDTH: usize = 160;
    pub const HEIGHT: usize = 144;

    pub fn new() -> Self {
        Self {
            frame: [Shade::White; Self::WIDTH * Self::HEIGHT],
        }
    }

    /// Panics out of range
    pub fn pixel(&self, x: usize, y: usize) -> Shade {
        if x >= Self::WIDTH || y >= Self::HEIGHT {
            panic!("coordinates exceed frame boundaries")
        }

        self.frame[y * Self::WIDTH + x]
    }

    pub fn pixels(&self) -> &[Shade] {
        &self.frame
    }

    pub(crate) fn write_pixel(&mut self, x: usize, y: usize, pixel: Shade) {
        if x >= Self::WIDTH || y >= Self::HEIGHT {
            panic!("coordinates exceed frame boundaries")
        }

        self.frame[y * Self::WIDTH + x] = pixel;
    }
}
