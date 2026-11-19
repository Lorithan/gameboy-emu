use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

const CARTRIDGE_SIZE: usize = 32 * 1024;

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
    VerificationFailure {
        address: usize,
        expected: u8,
        actual: u8,
    },
    GBCUnsupported,
    FileIO {
        err: io::Error,
        path: PathBuf,
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
            CartridgeError::VerificationFailure {
                address,
                expected,
                actual,
            } => write!(
                f,
                "verification failed at 0x{address:04X}. expected: 0x{expected:02X}, actual: 0x{actual:02X}"
            ),
            CartridgeError::GBCUnsupported => write!(f, "gameboy color currently unsupported"),
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
    bytes: [u8; CARTRIDGE_SIZE],
}

pub struct CartridgeMetadata {
    pub cartridge_type_code: u8,
    pub rom_size: usize,
    pub ram_size: usize,
}

impl Cartridge {
    pub fn from_bytes(data: &[u8]) -> Result<Cartridge, CartridgeError> {
        if data.len() < 0x0150 {
            return Err(CartridgeError::IncompleteHeader {
                minimum: 0x0150,
                actual: data.len(),
            });
        }
        if data.len() != CARTRIDGE_SIZE {
            return Err(CartridgeError::LengthError {
                actual: data.len(),
                expected: CARTRIDGE_SIZE,
            });
        }

        if data[0x0147] != 0x00 {
            return Err(CartridgeError::VerificationFailure {
                address: 0x147,
                expected: 0x00,
                actual: data[0x147],
            });
        }
        if data[0x0148] != 0x00 {
            return Err(CartridgeError::VerificationFailure {
                address: 0x148,
                expected: 0x00,
                actual: data[0x148],
            });
        }
        if data[0x0149] != 0x00 {
            return Err(CartridgeError::VerificationFailure {
                address: 0x149,
                expected: 0x00,
                actual: data[0x149],
            });
        }
        if data[0x0143] == 0xC0 {
            return Err(CartridgeError::GBCUnsupported);
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

        let bytes = data.try_into().map_err(|_| CartridgeError::LengthError {
            actual: data.len(),
            expected: CARTRIDGE_SIZE,
        })?;

        Ok(Cartridge { bytes })
    }

    pub fn from_filepath(path: impl AsRef<Path>) -> Result<Cartridge, CartridgeError> {
        let path = path.as_ref();
        let mut f = File::open(path).map_err(|err| CartridgeError::from_file_read(err, path))?;
        let mut data = Vec::with_capacity(CARTRIDGE_SIZE);
        f.read_to_end(&mut data)
            .map_err(|err| CartridgeError::from_file_read(err, path))?;

        Self::from_bytes(&data)
    }

    pub fn metadata(&self) -> CartridgeMetadata {
        CartridgeMetadata {
            cartridge_type_code: self.bytes[0x0147],
            rom_size: self.bytes.len(),
            ram_size: self.bytes[0x0149] as usize,
        }
    }

    pub fn rom(&self) -> &[u8] {
        &self.bytes
    }
}
