#[derive(Debug, Clone, Copy)]
pub(super) struct Object {
    pub(super) y: u8,
    pub(super) x: u8,
    pub(super) tile: u8,
    pub(super) attributes: u8,
}

impl From<&[u8; 4]> for Object {
    fn from(value: &[u8; 4]) -> Self {
        Self {
            y: value[0],
            x: value[1],
            tile: value[2],
            attributes: value[3],
        }
    }
}

impl Object {
    pub(super) fn bg_priority(&self) -> bool {
        self.attributes & (1 << 7) != 0
    }

    pub(super) fn y_flip(&self) -> bool {
        self.attributes & (1 << 6) != 0
    }

    pub(super) fn x_flip(&self) -> bool {
        self.attributes & (1 << 5) != 0
    }

    pub(super) fn uses_obp1(&self) -> bool {
        self.attributes & (1 << 4) != 0
    }
}
