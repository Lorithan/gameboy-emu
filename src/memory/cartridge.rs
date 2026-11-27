use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

const CARTRIDGE_SIZE: usize = 32 * 1024;
const KIB: usize = 1024;
const MIB: usize = KIB * KIB;

#[derive(Debug)]
pub enum CartridgeError {
    IncompleteHeader {
        minimum: usize,
        actual: usize,
    },
    ChecksumFailure {
        expected: u8,
        actual: u8,
    },
    LengthError {
        expected: usize,
        actual: usize,
    },
    GBCUnsupported,
    FileIO {
        err: io::Error,
        path: PathBuf,
    },
    InvalidHeaderCombination {
        cartridge_type: u8,
        rom_size: u8,
        ram_size: u8,
    },
    UnsupportedCartridgeType {
        code: u8,
    },
    UnsupportedRomSize {
        code: u8,
    },
    UnsupportedRamSize {
        code: u8,
    },
}

impl CartridgeError {
    fn from_file_read(value: io::Error, path: &Path) -> Self {
        CartridgeError::FileIO {
            err: value,
            path: path.to_path_buf(),
        }
    }
}

impl std::fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CartridgeError::IncompleteHeader { minimum, actual } => write!(
                f,
                "expected header minimum: {minimum} bytes, actual size: {actual} bytes"
            ),
            CartridgeError::ChecksumFailure { expected, actual } => write!(
                f,
                "checksum failure, expected: {expected}, actual: {actual}"
            ),
            CartridgeError::LengthError { expected, actual } => write!(
                f,
                "expected length: {expected} bytes, actual length: {actual} bytes"
            ),
            CartridgeError::GBCUnsupported => write!(f, "gameboy color currently unsupported"),
            CartridgeError::InvalidHeaderCombination {
                cartridge_type: kind,
                rom_size,
                ram_size,
            } => {
                write!(
                    f,
                    "invalid cartridge. kind: 0x{kind:02X}, rom: 0x{rom_size:02X}, ram: 0x{ram_size:02X}"
                )
            }
            CartridgeError::UnsupportedCartridgeType { code: value } => {
                write!(f, "unsupported cartridge type: {value:02X}")
            }
            CartridgeError::UnsupportedRamSize { code: value } => {
                write!(f, "unsupported ram size byte: {value:02X}")
            }
            CartridgeError::UnsupportedRomSize { code: value } => {
                write!(f, "unsupported rom size byte: {value:02X}")
            }
            CartridgeError::FileIO { path, .. } => {
                write!(f, "could not read cartridge at {}", path.to_string_lossy())
            }
        }
    }
}

impl std::error::Error for CartridgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let CartridgeError::FileIO { err, .. } = self {
            Some(err)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Cartridge {
    header: Header,
    bytes: Box<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgbSupport {
    DmgOnly,
    CgbCompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RamSize {
    None,
    Kib8,
    Kib32,
}

impl RamSize {
    pub fn from_code(value: u8) -> Result<Self, CartridgeError> {
        match value {
            0x00 => Ok(RamSize::None),
            0x02 => Ok(RamSize::Kib8),
            0x03 => Ok(RamSize::Kib32),
            _ => Err(CartridgeError::UnsupportedRamSize { code: value }),
        }
    }

    pub fn bytes(&self) -> usize {
        match self {
            RamSize::None => 0,
            RamSize::Kib8 => 8 * KIB,
            RamSize::Kib32 => 32 * KIB,
        }
    }

    fn code(&self) -> u8 {
        match self {
            RamSize::None => 0,
            RamSize::Kib8 => 2,
            RamSize::Kib32 => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RomSize {
    Kib32,
    Kib64,
    Kib128,
    Kib256,
    Kib512,
    Mib1,
    Mib2,
}

impl RomSize {
    pub fn from_code(value: u8) -> Result<Self, CartridgeError> {
        match value {
            0x00 => Ok(RomSize::Kib32),
            0x01 => Ok(RomSize::Kib64),
            0x02 => Ok(RomSize::Kib128),
            0x03 => Ok(RomSize::Kib256),
            0x04 => Ok(RomSize::Kib512),
            0x05 => Ok(RomSize::Mib1),
            0x06 => Ok(RomSize::Mib2),
            _ => Err(CartridgeError::UnsupportedRomSize { code: value }),
        }
    }

    pub fn banks(&self) -> usize {
        match self {
            RomSize::Kib32 => 2,
            RomSize::Kib64 => 4,
            RomSize::Kib128 => 8,
            RomSize::Kib256 => 16,
            RomSize::Kib512 => 32,
            RomSize::Mib1 => 64,
            RomSize::Mib2 => 128,
        }
    }

    pub fn bytes(&self) -> usize {
        match self {
            RomSize::Kib32 => 32 * KIB,
            RomSize::Kib64 => 64 * KIB,
            RomSize::Kib128 => 128 * KIB,
            RomSize::Kib256 => 256 * KIB,
            RomSize::Kib512 => 512 * KIB,
            RomSize::Mib1 => MIB,
            RomSize::Mib2 => 2 * MIB,
        }
    }

    fn code(&self) -> u8 {
        match self {
            RomSize::Kib32 => 0,
            RomSize::Kib64 => 1,
            RomSize::Kib128 => 2,
            RomSize::Kib256 => 3,
            RomSize::Kib512 => 4,
            RomSize::Mib1 => 5,
            RomSize::Mib2 => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeKind {
    RomOnly,
    Mbc1 { ram: RamSize, battery: bool },
}

impl CartridgeKind {
    fn new(type_code: u8, rom: RomSize, ram: RamSize) -> Result<Self, CartridgeError> {
        let battery = type_code == 3;
        match (type_code, ram) {
            (0x00, RamSize::None) if rom == RomSize::Kib32 => Ok(CartridgeKind::RomOnly),
            (0x01, RamSize::None) => Ok(CartridgeKind::Mbc1 { ram, battery }),
            (0x02 | 0x03, RamSize::Kib8) => Ok(CartridgeKind::Mbc1 { ram, battery }),
            (0x02 | 0x03, RamSize::Kib32) if rom <= RomSize::Kib512 => {
                Ok(CartridgeKind::Mbc1 { ram, battery })
            }
            (0x00..=0x03, _) => Err(CartridgeError::InvalidHeaderCombination {
                cartridge_type: type_code,
                rom_size: rom.code(),
                ram_size: ram.code(),
            }),
            (code, _) => Err(CartridgeError::UnsupportedCartridgeType { code }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub title: Option<String>,
    pub cgb: CgbSupport,
    pub kind: CartridgeKind,
    pub rom_size: RomSize,
}

impl Header {
    const TITLE_START_BYTE: usize = 0x0134;
    const CGB_FLAG_BYTE: usize = 0x0143;
    const CARTRIDGE_TYPE_BYTE: usize = 0x0147;
    const ROM_SIZE_BYTE: usize = 0x0148;
    const RAM_SIZE_BYTE: usize = 0x0149;

    pub fn parse(data: &[u8]) -> Result<Self, CartridgeError> {
        if data.len() < 0x0150 {
            return Err(CartridgeError::IncompleteHeader {
                minimum: 0x0150,
                actual: data.len(),
            });
        }

        let mut checksum = 0u8;
        for byte in data[0x0134..=0x014C].iter() {
            checksum = checksum.wrapping_sub(*byte).wrapping_sub(1);
        }

        if data[0x014D] != checksum {
            return Err(CartridgeError::ChecksumFailure {
                expected: checksum,
                actual: data[0x014D],
            });
        }

        let rom_size = RomSize::from_code(data[Self::ROM_SIZE_BYTE])?;
        let ram = RamSize::from_code(data[Self::RAM_SIZE_BYTE])?;
        let cgb = match data[Self::CGB_FLAG_BYTE] {
            byte if byte & (1 << 7) == 0 => CgbSupport::DmgOnly,
            0x80 => CgbSupport::CgbCompatible,
            _ => Err(CartridgeError::GBCUnsupported)?,
        };

        let title_end = if cgb == CgbSupport::DmgOnly {
            0x0143
        } else {
            0x0142
        };
        let title = parse_title(&data[Self::TITLE_START_BYTE..=title_end]);

        let kind_byte = data[Self::CARTRIDGE_TYPE_BYTE];

        let kind = CartridgeKind::new(kind_byte, rom_size, ram)?;

        Ok(Header {
            title,
            cgb,
            kind,
            rom_size,
        })
    }
}

impl Cartridge {
    pub fn from_bytes(data: &[u8]) -> Result<Cartridge, CartridgeError> {
        let header = Header::parse(data)?;

        if data.len() != header.rom_size.bytes() {
            return Err(CartridgeError::LengthError {
                expected: header.rom_size.bytes(),
                actual: data.len(),
            });
        }

        let bytes = data.into();

        Ok(Cartridge { bytes, header })
    }

    pub fn from_filepath(path: impl AsRef<Path>) -> Result<Cartridge, CartridgeError> {
        let path = path.as_ref();
        let mut f = File::open(path).map_err(|err| CartridgeError::from_file_read(err, path))?;
        let mut data = Vec::with_capacity(CARTRIDGE_SIZE);
        f.read_to_end(&mut data)
            .map_err(|err| CartridgeError::from_file_read(err, path))?;

        Self::from_bytes(&data)
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn rom(&self) -> &[u8] {
        &self.bytes
    }
}

/// `field` is 0134-0143 (16 bytes), or 0134-0142 when 0143 is a CGB flag.
fn parse_title(field: &[u8]) -> Option<String> {
    // The title ends at the first 00; whatever follows is padding.
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    let text = &field[..end];

    // Any unprintable byte before the padding means this isn't a title.
    if !text.iter().all(|b| (0x20..=0x7E).contains(b)) {
        return None;
    }

    let title: String = text.iter().map(|&b| char::from(b)).collect();
    let title = title.trim_end_matches(' ');
    (!title.is_empty()).then(|| title.to_owned())
}
