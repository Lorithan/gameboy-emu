use super::{Cartridge, CartridgeError};
use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

// Synthetic, non-bootable inputs; no game or boot ROM is required.
const ROM_LENGTH: usize = 32_768;

fn valid_rom() -> Vec<u8> {
    let mut rom = vec![0; ROM_LENGTH];
    rom[0x014D] = 0xE7; // Independently known: -25 modulo 256.
    rom
}

// Fixture preparation uses the sum formula, not the production subtraction loop.
fn refresh_checksum(rom: &mut [u8]) {
    let sum: i32 = rom[0x0134..=0x014C]
        .iter()
        .map(|&byte| i32::from(byte))
        .sum();
    rom[0x014D] = (-sum - 25).rem_euclid(256) as u8;
}

#[test]
fn accepts_zero_header_with_known_checksum_and_preserves_every_byte() {
    let rom = valid_rom();
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");
    assert_eq!(cartridge.bytes.as_slice(), rom.as_slice());
}

#[test]
fn preserves_rom_region_boundaries() {
    let mut rom = valid_rom();
    for (offset, value) in [(0, 0x12), (0x3FFF, 0x34), (0x4000, 0x56), (0x7FFF, 0x78)] {
        rom[offset] = value;
    }
    let cartridge = Cartridge::from_bytes(&rom).expect("valid marked ROM");
    assert_eq!(cartridge.bytes.as_slice(), rom.as_slice());
}

#[test]
fn distinguishes_incomplete_headers_without_panicking() {
    for length in [0, 1, 0x014E, 0x014F] {
        let mut rom = valid_rom();
        rom.resize(length, 0);
        let error = Cartridge::from_bytes(&rom).expect_err("incomplete header must fail");
        assert!(
            matches!(error, CartridgeError::IncompleteHeader { minimum: 0x0150, actual } if actual == length),
            "wrong diagnostic for length {length}: {error:?}"
        );
    }
}

#[test]
fn rejects_wrong_rom_lengths_after_a_complete_header_without_panicking() {
    for length in [0x0150, 32_767, 32_769, 65_536] {
        let mut rom = valid_rom();
        rom.resize(length, 0);
        let error = Cartridge::from_bytes(&rom).expect_err("wrong length must fail");
        assert!(
            matches!(error, CartridgeError::LengthError { expected: ROM_LENGTH, actual } if actual == length),
            "wrong diagnostic for length {length}: {error:?}"
        );
    }
}

#[test]
fn public_rom_accessor_exposes_all_bytes_including_header() {
    let mut rom = valid_rom();
    rom[0] = 0x12;
    rom[0x7FFF] = 0x34;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");
    let exposed: &[u8] = cartridge.rom();
    assert_eq!(exposed.len(), ROM_LENGTH);
    assert_eq!(exposed, rom.as_slice());
    assert_eq!(exposed[0x014D], 0xE7);
}

#[test]
fn metadata_reports_decoded_sizes_in_bytes() {
    let cartridge = Cartridge::from_bytes(&valid_rom()).expect("valid synthetic ROM");
    let metadata = cartridge.metadata();
    assert_eq!(metadata.cartridge_type_code, 0x00);
    assert_eq!(metadata.ram_size, 0);
    assert_eq!(
        metadata.rom_size, ROM_LENGTH,
        "header code 0 means 32 KiB, not zero bytes"
    );
}

#[test]
fn rejects_unsupported_header_fields_with_their_values() {
    for (address, values) in [
        (0x0147, &[0x01, 0x08, 0xFF][..]),
        (0x0148, &[0x01, 0xFF][..]),
        (0x0149, &[0x02, 0xFF][..]),
    ] {
        for &value in values {
            let mut rom = valid_rom();
            rom[address] = value;
            refresh_checksum(&mut rom);
            let error = Cartridge::from_bytes(&rom).expect_err("unsupported header must fail");
            assert!(
                matches!(error, CartridgeError::VerificationFailure { address: actual_address, expected: 0, actual } if actual_address == address && actual == value),
                "wrong diagnostic for {address:#06X}={value:#04X}: {error:?}"
            );
        }
    }
}

#[test]
fn accepts_dmg_compatible_color_header_with_known_checksum() {
    let mut rom = valid_rom();
    rom[0x0143] = 0x80;
    rom[0x014D] = 0x67;
    let cartridge = Cartridge::from_bytes(&rom).expect("DMG-compatible header");
    assert_eq!(cartridge.bytes.as_slice(), rom.as_slice());
}

#[test]
fn rejects_color_only_header() {
    let mut rom = valid_rom();
    rom[0x0143] = 0xC0;
    rom[0x014D] = 0x27;
    assert!(matches!(
        Cartridge::from_bytes(&rom),
        Err(CartridgeError::GBCUnsupported)
    ));
}

#[test]
fn accepts_legacy_title_byte_in_color_flag_position() {
    let mut rom = valid_rom();
    rom[0x0143] = b'A';
    refresh_checksum(&mut rom);
    assert!(Cartridge::from_bytes(&rom).is_ok());
}

#[test]
fn rejects_incorrect_stored_checksum_with_expected_and_actual_values() {
    let mut rom = valid_rom();
    rom[0x014D] = 0xE6;
    assert!(matches!(
        Cartridge::from_bytes(&rom),
        Err(CartridgeError::ChecksumFailure {
            expected: 0xE7,
            actual: 0xE6
        })
    ));
}

#[test]
fn checksum_includes_both_ends_of_the_header_range() {
    for address in [0x0134, 0x014C] {
        let mut rom = valid_rom();
        rom[address] = 1;
        assert!(matches!(
            Cartridge::from_bytes(&rom),
            Err(CartridgeError::ChecksumFailure {
                expected: 0xE6,
                actual: 0xE7
            })
        ));
        rom[0x014D] = 0xE6;
        assert!(Cartridge::from_bytes(&rom).is_ok(), "offset {address:#06X}");
    }
}

#[test]
fn checksum_wraps_with_nonzero_header_bytes() {
    let mut rom = valid_rom();
    rom[0x0134] = 0xFF;
    rom[0x0135] = 0xFF;
    rom[0x014D] = 0xE9; // -(255 + 255) - 25 modulo 256.
    assert!(Cartridge::from_bytes(&rom).is_ok());
}

#[test]
fn logo_and_global_checksum_are_not_loader_validation_requirements() {
    let mut rom = valid_rom();
    rom[0x0104..=0x0133].fill(0xA5);
    rom[0x014E] = 0x12;
    rom[0x014F] = 0x34;
    let cartridge = Cartridge::from_bytes(&rom).expect("ignored fields must not reject input");
    assert_eq!(cartridge.bytes.as_slice(), rom.as_slice());
}

struct ScratchDirectory(PathBuf);

impl ScratchDirectory {
    fn create() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "gameboy-emu-cartridge-tests-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create scratch directory: {error}"),
            }
        }
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        // This directory was created exclusively by this test helper.
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn raw_file_loading_matches_in_memory_loading() {
    let scratch = ScratchDirectory::create();
    let path = scratch.0.join("synthetic.gb");
    let rom = valid_rom();
    fs::write(&path, &rom).expect("write fixture");
    let from_file = Cartridge::from_filepath(&path).expect("read fixture");
    let from_memory = Cartridge::from_bytes(&rom).expect("parse fixture");
    assert_eq!(from_file.bytes, from_memory.bytes);
}

#[test]
fn missing_file_retains_path_and_io_cause() {
    let scratch = ScratchDirectory::create();
    let path = scratch.0.join("does-not-exist.gb");
    match Cartridge::from_filepath(&path).expect_err("missing file must fail") {
        CartridgeError::FileIO { err, path: actual } => {
            assert_eq!(actual, path);
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
        }
        error => panic!("wrong error: {error:?}"),
    }
}

#[test]
fn directory_input_retains_path_and_io_cause() {
    let scratch = ScratchDirectory::create();
    // Windows may reject opening a directory; Unix may reject reading it.
    match Cartridge::from_filepath(&scratch.0).expect_err("directory is not a ROM file") {
        CartridgeError::FileIO { err, path } => {
            assert_eq!(path, scratch.0);
            assert_ne!(err.kind(), io::ErrorKind::NotFound);
        }
        error => panic!("wrong error: {error:?}"),
    }
}

#[test]
fn invalid_file_content_reports_validation_failure() {
    let scratch = ScratchDirectory::create();
    let path = scratch.0.join("bad-checksum.gb");
    let mut rom = valid_rom();
    rom[0x014D] = 0xE6;
    fs::write(&path, rom).expect("write invalid fixture");
    assert!(matches!(
        Cartridge::from_filepath(&path),
        Err(CartridgeError::ChecksumFailure {
            expected: 0xE7,
            actual: 0xE6
        })
    ));
}
